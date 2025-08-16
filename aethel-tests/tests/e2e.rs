use std::time::Duration;
use std::process::{Child, Command, Stdio};
use tokio::time::sleep;
use tonic::transport::Channel;
use futures::StreamExt;

use aethel_common::proto::aethel::aethel_service_client::AethelServiceClient;
use aethel_common::proto::aethel::{
    CreateContainerRequest, DeleteContainerRequest, ListContainersRequest,
};

struct Daemon {
    process: Child,
}

impl Daemon {
    fn start() -> Self {
        let process = Command::new("cargo")
            .args(&[
                "run",
                "--package",
                "aethel-d",
                "--bin",
                "aethel-d",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start aethel-d");
        Daemon { process }
    }

    async fn wait_for_ready() {
        // In a real scenario, you'd probe the gRPC port. For now, a sleep is simple.
        sleep(Duration::from_secs(2)).await;
    }

    async fn connect() -> Result<AethelServiceClient<Channel>, tonic::transport::Error> {
        AethelServiceClient::connect("http://[::1]:50051").await
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        // Ensure the daemon is killed when the test is over.
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}


#[tokio::test]
async fn stress_test_lifecycle() {
    if unsafe { libc::getuid() } != 0 {
        eprintln!("Skipping stress_test_lifecycle: must be run as root.");
        return;
    }

    let _daemon = Daemon::start();
    Daemon::wait_for_ready().await;
    let mut client = Daemon::connect().await.expect("Failed to connect to daemon");

    let num_containers = 10;
    let mut create_tasks = Vec::new();

    for i in 0..num_containers {
        let mut client = client.clone();
        let task = tokio::spawn(async move {
            let req = CreateContainerRequest {
                image_name: "alpine".to_string(),
                name: format!("stress-{}", i),
                command: "sleep".to_string(),
                args: vec!["30".to_string()],
            };
            client.create_container(req).await
        });
        create_tasks.push(task);
    }

    let mut container_ids = Vec::new();
    for task in create_tasks {
        let resp = task.await.unwrap().expect("Failed to create container");
        container_ids.push(resp.into_inner().id);
    }

    assert_eq!(container_ids.len(), num_containers);
    
    let mut delete_tasks = Vec::new();
    for id in container_ids {
        let mut client = client.clone();
        let task = tokio::spawn(async move {
            let req = DeleteContainerRequest { container_id: id };
            client.delete_container(req).await
        });
        delete_tasks.push(task);
    }

    for task in delete_tasks {
        task.await.unwrap().expect("Failed to delete container");
    }

    let list_req = ListContainersRequest {};
    let final_list = client.list_containers(list_req).await.unwrap().into_inner();
    let final_containers: Vec<_> = final_list.map(|c| c.unwrap()).collect::<Vec<_>>().await;
    assert!(final_containers.is_empty(), "Not all containers were deleted");
}

#[tokio::test]
async fn daemon_restart_resilience() {
    if unsafe { libc::getuid() } != 0 {
        eprintln!("Skipping daemon_restart_resilience: must be run as root.");
        return;
    }

    // --- First run ---
    let mut daemon1 = Daemon::start();
    Daemon::wait_for_ready().await;
    let mut client1 = Daemon::connect().await.expect("Failed to connect to first daemon");

    let create_req = CreateContainerRequest {
        image_name: "alpine".to_string(),
        name: "resilience-test".to_string(),
        command: "sleep".to_string(),
        args: vec!["60".to_string()],
    };
    client1.create_container(create_req).await.expect("Failed to create container on first run");

    // Ensure the container is listed
    let list_req1 = ListContainersRequest {};
    let list1 = client1.list_containers(list_req1).await.unwrap().into_inner();
    let containers1: Vec<_> = list1.map(|c| c.unwrap()).collect::<Vec<_>>().await;
    assert_eq!(containers1.len(), 1, "Container not created in first run");
    
    // --- Kill and restart ---
    daemon1.process.kill().unwrap();
    daemon1.process.wait().unwrap();
    drop(daemon1);

    sleep(Duration::from_secs(1)).await;

    // --- Second run ---
    let _daemon2 = Daemon::start();
    Daemon::wait_for_ready().await;
    let mut client2 = Daemon::connect().await.expect("Failed to connect to second daemon");

    // Verify state is clean (no persistence yet)
    let list_req2 = ListContainersRequest {};
    let list2 = client2.list_containers(list_req2).await.unwrap().into_inner();
    let containers2: Vec<_> = list2.map(|c| c.unwrap()).collect::<Vec<_>>().await;
    assert!(containers2.is_empty(), "Daemon restarted with unexpected state");

    // Verify daemon is functional
    let create_req2 = CreateContainerRequest {
        image_name: "alpine".to_string(),
        name: "post-restart-test".to_string(),
        command: "sleep".to_string(),
        args: vec!["10".to_string()],
    };
    client2.create_container(create_req2).await.expect("Failed to create container on second run");

    let list_req3 = ListContainersRequest {};
    let list3 = client2.list_containers(list_req3).await.unwrap().into_inner();
    let containers3: Vec<_> = list3.map(|c| c.unwrap()).collect::<Vec<_>>().await;
    assert_eq!(containers3.len(), 1, "Container not created in second run");
} 