use anyhow::{Context, Result};
use atlas_local::{
    Client,
    models::{CreateDeploymentOptions, Deployment},
};
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_socket_defaults().context("connecting to docker")?;
    let client = Client::new(docker);

    // Create a deployment for demonstration
    let deployment_name = "lifecycle-demo";
    println!("Creating deployment '{}'...", deployment_name);
    client
        .create_deployment(&CreateDeploymentOptions {
            name: Some(deployment_name.to_string()),
            wait_until_healthy: Some(true),
            ..Default::default()
        })
        .await?;

    print_deployment_state(&client, deployment_name).await?;

    // Stop the deployment
    println!("Stopping deployment '{}'...", deployment_name);
    client
        .stop_deployment(deployment_name)
        .await
        .context("stopping deployment")?;

    print_deployment_state(&client, deployment_name).await?;

    // Start the deployment
    println!("Starting deployment '{}'...", deployment_name);
    client
        .start_deployment(deployment_name)
        .await
        .context("starting deployment")?;

    print_deployment_state(&client, deployment_name).await?;

    // Remove the deployment
    println!("Removing deployment '{}'...", deployment_name);
    client
        .delete_deployment(deployment_name)
        .await
        .context("removing deployment")?;

    println!("Deployment '{}' removed successfully", deployment_name);

    Ok(())
}

async fn print_deployment_state(client: &Client, deployment_name: &str) -> Result<()> {
    let Deployment { state, .. } = client
        .get_deployment(deployment_name)
        .await
        .context("getting deployment")?;

    println!("State: {state}");

    Ok(())
}
