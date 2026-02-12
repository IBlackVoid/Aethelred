use aethel_common::proto::aethel::aethel_service_client::AethelServiceClient;
use aethel_common::proto::aethel::{CreateContainerRequest, StopRequest, LogsRequest};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run { 
        #[arg(short, long)]
        image: String,
        #[arg(default_value = "/bin/sh")]
        command: String,
    },
    Ps {},
    Stop { 
        #[arg(short, long)]
        container_id: String 
    },
    Logs { 
        #[arg(short, long)]
        container_id: String 
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut client = AethelServiceClient::connect("http://[::1]:50051").await?;

    match &cli.command {
        Commands::Run { image, command } => {
            let request = tonic::Request::new(CreateContainerRequest {
                image_name: image.clone(),
                command: command.clone(),
                args: vec![],
            });
            let response = client.create_container(request).await?.into_inner();
            println!("Container created with ID: {} and IP: {}", response.container_id, response.ip_address);
        }
        Commands::Ps {} => {
            let mut stream = client.list_containers(tonic::Request::new(aethel_common::proto::aethel::Empty {})).await?.into_inner();
            println!("{:<36} {:<20} {:<10} {:<15}", "ID", "IMAGE", "STATUS", "IP");
            while let Some(container) = stream.message().await? {
                println!("{:<36} {:<20} {:<10} {:<15}", container.id, container.image, container.status, container.ip_address);
            }
        }
        Commands::Stop { container_id } => {
            let request = tonic::Request::new(StopRequest {
                container_id: container_id.clone(),
            });
            let response = client.stop_container(request).await?;
            println!("Container stopped: {}", response.into_inner().success);
        }
        Commands::Logs { container_id } => {
            let request = tonic::Request::new(LogsRequest {
                container_id: container_id.clone(),
            });
            let mut stream = client.stream_logs(request).await?.into_inner();

            while let Some(log_entry) = stream.message().await? {
                println!("{}", log_entry.entry);
            }
        }
    }

    Ok(())
}
