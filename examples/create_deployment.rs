use anyhow::{Context, Result};
use atlas_local::{Client, models::CreateDeploymentOptions};
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_defaults().context("connecting to docker")?;
    let client = Client::new(docker);

    // Create a deployment with the name local1234 and loaded sample data
    // More details about sample data can be found here: https://docs.mongodb.com/atlas/sample-data/
    let deployment1 = CreateDeploymentOptions {
        name: Some("local1234".to_string()),
        load_sample_data: Some(true),
        ..Default::default()
    };
    let deployment = client
        .create_deployment(&deployment1)
        .await
        .context("creating deployment local 1234")?;

    println!(
        "[{}] \t{}",
        deployment.mongodb_version,
        deployment.name.unwrap_or_default()
    );

    let deployment2 = client
        .create_deployment(&CreateDeploymentOptions::default())
        .await
        .context("creating default deployment")?;

    println!(
        "[{}] \t{}",
        deployment2.mongodb_version,
        deployment2.name.unwrap_or_default()
    );

    Ok(())
}
