use aethel_common::proto::aethel::aethel_service_client::AethelServiceClient;
use aethel_common::proto::aethel::{
    CreateContainerRequest, DeleteContainerRequest, GetContainerRequest, ListContainersRequest,
    StreamLogsRequest,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new container
    Create {
        #[arg(short, long)]
        image: String,
        /// Optional name for the container
        #[arg(short, long)]
        name: Option<String>,
        /// The command to run inside the container
        #[arg(default_value = "/bin/sh")]
        command: String,
        /// Arguments for the command
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// List all containers
    Ps {},
    /// Get detailed information about a container
    Get {
        #[arg()]
        container_id: String,
    },
    /// Delete a container
    Rm {
        #[arg()]
        container_id: String,
    },
    /// Stream logs from a container
    Logs {
        #[arg(short, long)]
        container_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut client = AethelServiceClient::connect("http://[::1]:50051").await?;

    match &cli.command {
        Commands::Create { image, name, command, args } => {
            let request = CreateContainerRequest {
                image_name: image.clone(),
                name: name.clone().unwrap_or_default(),
                command: command.clone(),
                args: args.clone(),
            };
            let response = client.create_container(request).await?.into_inner();
            println!(
                "Container created with ID: {} and Name: {}",
                response.id, response.name
            );
        }
        Commands::Ps {} => {
            let request = ListContainersRequest {};
            let mut stream = client.list_containers(request).await?.into_inner();
            println!(
                "{:<36} {:<20} {:<20} {:<10} {:<15}",
                "ID", "NAME", "IMAGE", "STATUS", "IP"
            );
            while let Some(container) = stream.message().await? {
                println!(
                    "{:<36} {:<20} {:<20} {:<10} {:<15}",
                    container.id, container.name, container.image, container.status, container.ip_address
                );
            }
        }
        Commands::Get { container_id } => {
            let request = GetContainerRequest {
                container_id: container_id.clone(),
            };
            let response = client.get_container(request).await?.into_inner();
            println!("{:#?}", response);
        }
        Commands::Rm { container_id } => {
            let request = DeleteContainerRequest {
                container_id: container_id.clone(),
            };
            client.delete_container(request).await?;
            println!("Container {} deleted", container_id);
        }
        Commands::Logs { container_id } => {
            let request = StreamLogsRequest {
                container_id: container_id.clone(),
            };
            let mut stream = client.stream_logs(request).await?.into_inner();

            while let Some(log_entry) = stream.message().await? {
                print!("{}", log_entry.entry);
            }
        }
    }

    Ok(())
}
