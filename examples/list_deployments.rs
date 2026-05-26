use anyhow::{Context, Result};
use atlas_local::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect_with_socket_defaults().context("connecting to docker")?;

    let deployments = client
        .list_deployments()
        .await
        .context("listing deployments")?;

    println!("Deployments:");
    for deployment in deployments {
        println!(
            "[{}] \t{}",
            deployment.mongodb_version,
            deployment.name.unwrap_or_default()
        );
    }

    Ok(())
}
