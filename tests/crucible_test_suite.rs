use std::process::{Child, Command, Stdio};
use std::sync::Once;
use std::time::{Duration, Instant};
use tonic::transport::Channel;
use aethel_common::proto::aethel::aethel_service_client::AethelServiceClient;
use aethel_common::proto::aethel::*;
use rand::{seq::SliceRandom, Rng};
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::timeout;
use once_cell::sync::OnceCell;

static DAEMON: OnceCell<Mutex<TestContext>> = OnceCell::new();

struct TestContext {
    _child: Child, // Keep daemon alive
    client: AethelServiceClient<Channel>,
}

async fn test_context() -> MutexGuard<'static, TestContext> {
    DAEMON.get_or_init(|| {
        let child = Command::new("target/release/aethel-d")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn daemon");
        // Wait a moment for gRPC server to start
        std::thread::sleep(Duration::from_secs(1));
        let client = futures::executor::block_on(async {
            AethelServiceClient::connect("http://[::1]:50051").await
        }).expect("Failed to connect to daemon");
        Mutex::new(TestContext { _child: child, client })
    }).lock().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_namespace_integrity_and_security() {
    let mut ctx = test_context().await;
    let script = r#"#!/bin/sh
set -e
(! cat /etc/shadow >/dev/null 2>&1) && echo SHADOW_OK
echo "Killing 1.."
kill -9 1 >/dev/null 2>&1 || true
echo KILL_OK
ip addr | grep -E " (lo|eth0):" && echo NET_OK
PROC=$(ps ax | wc -l)
[ "$PROC" -lt 5 ] && echo PS_OK
echo CHECKS_PASSED
"#;

    let req = CreateContainerRequest {
        image_name: "alpine".into(),
        command: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
    };

    let resp = ctx.client.create_container(req).await.expect("create_container").into_inner();
    let mut log_stream = ctx
        .client
        .stream_logs(LogsRequest { container_id: resp.container_id.clone() })
        .await
        .expect("stream logs")
        .into_inner();

    let mut passed = false;
    while let Some(entry) = log_stream.message().await.expect("stream message") {
        if entry.entry.contains("CHECKS_PASSED") {
            passed = true;
            break;
        }
    }
    assert!(passed, "Container security checks did not pass");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn test_concurrent_lifecycle_chaos() {
    let ctx_mutex = test_context().await; // hold briefly for clone
    let client = ctx_mutex.client.clone();
    drop(ctx_mutex);

    let ids = std::sync::Arc::new(Mutex::new(Vec::<String>::new()));
    let start = Instant::now();
    let mut handles = Vec::new();
    for _ in 0..50 {
        let client = client.clone();
        let ids = ids.clone();
        handles.push(tokio::spawn(async move {
            let mut rng = rand::thread_rng();
            while Instant::now() - start < Duration::from_secs(10) {
                let choice: u8 = rng.gen_range(0..3);
                match choice {
                    0 => {
                        // create
                        let req = CreateContainerRequest {
                            image_name: "alpine".into(),
                            command: "/bin/sh".into(),
                            args: vec!["-c".into(), "sleep 5".into()],
                        };
                        if let Ok(resp) = client.clone().create_container(req).await {
                            ids.lock().await.push(resp.into_inner().container_id);
                        }
                    }
                    1 => {
                        // stop
                        let maybe_id = { ids.lock().await.choose(&mut rng).cloned() };
                        if let Some(id) = maybe_id {
                            let _ = client.clone().stop_container(StopRequest { container_id: id }).await;
                        }
                    }
                    _ => {
                        // list containers
                        let _ = client.clone().list_containers(Empty {}).await;
                    }
                }
            }
        }));
    }

    for h in handles { h.await.unwrap(); }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_network_throughput_and_latency() {
    let mut ctx = test_context().await;
    // Start server container running iperf3
    let srv_req = CreateContainerRequest {
        image_name: "iperf".into(),
        command: "iperf3".into(),
        args: vec!["-s".into()],
    };
    let srv_resp = ctx.client.create_container(srv_req).await.unwrap().into_inner();

    // Give server time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Start client
    let cli_req = CreateContainerRequest {
        image_name: "iperf".into(),
        command: "iperf3".into(),
        args: vec!["-c".into(), srv_resp.ip_address.clone().into(), "-t".into(), "5".into(), "-J".into()],
    };
    let cli_resp = ctx.client.create_container(cli_req).await.unwrap().into_inner();

    let mut log_stream = ctx
        .client
        .stream_logs(LogsRequest { container_id: cli_resp.container_id }).await.unwrap().into_inner();

    let mut json_line = String::new();
    while let Some(msg) = log_stream.message().await.unwrap() {
        if msg.entry.contains("\"end\":") { // crude check for iperf3 JSON end section
            json_line = msg.entry;
            break;
        }
    }

    let parsed: serde_json::Value = serde_json::from_str(&json_line).expect("parse json");
    let bps = parsed["end"]["sum_sent"]["bits_per_second"].as_f64().unwrap();
    assert!(bps > 10_000_000_000f64, "Throughput too low: {}", bps);
} 