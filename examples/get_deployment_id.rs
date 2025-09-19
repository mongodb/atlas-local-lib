use anyhow::{Context, Result};
use atlas_local::{Client, models::CreateDeploymentOptions};
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_defaults().context("connecting to docker")?;
    let client = Client::new(docker.clone());

    let deployment_options = CreateDeploymentOptions::default();
    let deployment = client
        .create_deployment(&deployment_options)
        .await
        .context("creating deployment")?;

    let deployment_id = client
        .get_deployment_id(&deployment.container_id)
        .await
        .context("getting deployment id")?;

    println!("Deployment ID: {}", deployment_id);

    Ok(())
}
