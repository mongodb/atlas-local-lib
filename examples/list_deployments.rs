use anyhow::{Context, Result};
use atlas_local::Client;
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_socket_defaults().context("connecting to docker")?;
    let client = Client::new(docker);

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
