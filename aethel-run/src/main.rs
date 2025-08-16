use aethel_run::container::ContainerBuilder;

fn main() {
    println!("Starting Aethelred container...");

    let container = ContainerBuilder::new("test-container", "/bin/sh").build().unwrap();

    println!("Container {} created", container.id());

    container.wait().unwrap();

    println!("Container exited");
}