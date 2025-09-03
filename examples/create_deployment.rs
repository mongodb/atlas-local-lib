use anyhow::{Context, Result};
use atlas_local::{Client, models::CreateDeploymentOptions};
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_socket_defaults().context("connecting to docker")?;
    let client = Client::new(docker);

    let deployment1 = CreateDeploymentOptions {
        name: "local1234".to_string(),
        ..Default::default()
    };
    client
        .create_deployment(&deployment1)
        .await
        .context("creating deployment local 1234")?;

    print_deployments(&client).await?;

    client
        .create_deployment(&CreateDeploymentOptions::default())
        .await
        .context("creating default deployment")?;

    print_deployments(&client).await?;

    Ok(())
}

async fn print_deployments(client: &Client) -> Result<()> {
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
