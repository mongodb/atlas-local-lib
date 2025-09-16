use anyhow::{Context, Result};
use atlas_local::{models::{BindingType, CreateDeploymentOptions, MongoDBPortBinding}, Client};
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_defaults().context("connecting to docker")?;
    let client = Client::new(docker);

    let deployment1 = CreateDeploymentOptions {
        name: Some("local1234".to_string()),
        mongodb_port_binding: Some(MongoDBPortBinding {
            port: Some(27017),
            binding_type: BindingType::AnyInterface,
        }),
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
