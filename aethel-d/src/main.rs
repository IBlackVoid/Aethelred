use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::net::Ipv4Addr;
use std::os::unix::io::{FromRawFd, RawFd};

use aethel_common::proto::aethel::aethel_service_server::{AethelService, AethelServiceServer};
use aethel_common::proto::aethel::{CreateContainerRequest, CreateContainerResponse, Empty, ContainerInfo, StopRequest, StopResponse, LogsRequest, LogEntry};
use aethel_run::ContainerBuilder;
use aethel_storage::prepare_rootfs;

use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::io::{AsyncReadExt, BufReader};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

mod network;

#[derive(Debug, Clone)]
pub struct Container {
    id: String,
    image: String,
    status: String,
    pid: u32,
    ip_address: Ipv4Addr,
}

pub struct MyAethelService {
    containers: Arc<Mutex<HashMap<String, Container>>>,
    log_broadcasters: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    next_ip: Arc<Mutex<u8>>,
    net_handle: Arc<rtnetlink::Handle>,
}

async fn log_forwarder(pipe_fd: RawFd, broadcaster: broadcast::Sender<String>) {
    let pipe = unsafe { tokio::fs::File::from_raw_fd(pipe_fd) };
    let mut reader = BufReader::new(pipe);
    let mut buf = [0; 1024];

    while let Ok(n) = reader.read(&mut buf).await {
        if n == 0 {
            break;
        }
        if let Ok(line) = String::from_utf8(buf[..n].to_vec()) {
            let _ = broadcaster.send(line);
        }
    }
}

#[tonic::async_trait]
impl AethelService for MyAethelService {
    async fn create_container(
        &self,
        request: Request<CreateContainerRequest>,
    ) -> Result<Response<CreateContainerResponse>, Status> {
        let req = request.into_inner();
        let container_id = uuid::Uuid::new_v4().to_string();
        let rootfs_path = format!("/tmp/aethel/{}", container_id);
        let image_path = "./images";

        prepare_rootfs(Path::new(image_path), &req.image_name, Path::new(&rootfs_path))
            .map_err(|e| Status::internal(format!("prepare_rootfs failed: {}", e)))?;

        let (log_tx, _) = broadcast::channel(1024);
        self.log_broadcasters.lock().await.insert(container_id.clone(), log_tx.clone());

        let builder = ContainerBuilder::new(&container_id, &req.command)
            .map_err(|e| Status::internal(format!("container build failed: {}", e)))?;
        let (child_pid, pipe_fd) = unsafe { builder.with_rootfs(Path::new(&rootfs_path)).build() }
            .map_err(|e| Status::internal(format!("container build failed: {}", e)))?;

        tokio::spawn(log_forwarder(pipe_fd, log_tx));

        let mut next_ip_guard = self.next_ip.lock().await;
        let ip = Ipv4Addr::new(172, 29, 0, *next_ip_guard);
        *next_ip_guard += 1;

        network::setup_container_net(&self.net_handle, child_pid as i32, &container_id, ip)
            .await
            .map_err(|e| Status::internal(format!("network setup failed: {}", e)))?;

        let container = Container {
            id: container_id.clone(),
            image: req.image_name,
            status: "Running".to_string(),
            pid: child_pid as u32,
            ip_address: ip,
        };

        self.containers.lock().await.insert(container_id.clone(), container);

        Ok(Response::new(CreateContainerResponse { container_id, ip_address: ip.to_string() }))
    }

    type ListContainersStream = ReceiverStream<Result<ContainerInfo, Status>>;

    async fn list_containers(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListContainersStream>, Status> {
        let (tx, rx) = mpsc::channel(4);
        let containers = self.containers.lock().await.clone();

        tokio::spawn(async move {
            for (_, container) in containers.iter() {
                if tx.send(Ok(ContainerInfo {
                    id: container.id.clone(),
                    image: container.image.clone(),
                    status: container.status.clone(),
                    ip_address: container.ip_address.to_string(),
                }))
                .await
                .is_err()
                {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn stop_container(
        &self,
        request: Request<StopRequest>,
    ) -> Result<Response<StopResponse>, Status> {
        let req = request.into_inner();
        let mut containers = self.containers.lock().await;
        if let Some(container) = containers.get_mut(&req.container_id) {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(container.pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            )
            .map_err(|e| Status::internal(format!("failed to stop container: {}", e)))?;
            container.status = "Stopped".to_string();
            Ok(Response::new(StopResponse { success: true }))
        } else {
            Ok(Response::new(StopResponse { success: false }))
        }
    }

    type StreamLogsStream = ReceiverStream<Result<LogEntry, Status>>;

    async fn stream_logs(
        &self,
        request: Request<LogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        let broadcaster = {
            let broadcasters = self.log_broadcasters.lock().await;
            broadcasters.get(&req.container_id).cloned()
        };

        let Some(broadcaster) = broadcaster else {
            return Err(Status::not_found("Container not found"));
        };

        tokio::spawn(async move {
            let mut subscriber = broadcaster.subscribe();
            loop {
                match subscriber.recv().await {
                    Ok(line) => {
                        if tx.send(Ok(LogEntry { entry: line })).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (conn, handle, _) = rtnetlink::new_connection()?;
    tokio::spawn(conn);

    network::setup_bridge(&handle).await?;

    let command = std::process::Command::new("iptables")
        .args(&["-t", "nat", "-A", "POSTROUTING", "-s", "172.29.0.0/24", "!", "-o", "aethel0", "-j", "MASQUERADE"])
        .status()?;

    if !command.success() {
        return Err("Failed to set up NAT".into());
    }

    let addr = "[::1]:50051".parse()?;
    let service = MyAethelService {
        containers: Arc::new(Mutex::new(HashMap::new())),
        log_broadcasters: Arc::new(Mutex::new(HashMap::new())),
        next_ip: Arc::new(Mutex::new(2)),
        net_handle: Arc::new(handle),
    };

    Server::builder()
        .add_service(AethelServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
