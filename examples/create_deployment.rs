use std::collections::HashMap;

use anyhow::{Context, Result};
use atlas_local::{
    Client,
    bollard::models::{HostConfig, PortBinding},
    models::{BindingType, CreateDeploymentOptions, MongoDBPortBinding},
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect_with_defaults().context("connecting to docker")?;

    // Create a deployment with the name local1234 and loaded sample data
    // More details about sample data can be found here: https://docs.mongodb.com/atlas/sample-data/
    let deployment1 = CreateDeploymentOptions {
        name: Some("local1234".to_string()),
        load_sample_data: Some(true),
        ..Default::default()
    };
    let deployment1 = client
        .create_deployment(deployment1)
        .await
        .context("creating deployment local 1234")?;

    println!(
        "[{}] \t{}",
        deployment1.mongodb_version,
        deployment1.name.unwrap_or_default()
    );

    let deployment2_options = CreateDeploymentOptions {
        name: Some("local5678".to_string()),
        mongodb_port_binding: Some(MongoDBPortBinding::new(Some(50000), BindingType::Loopback)),
        ..Default::default()
    };
    // This overrides port 50000 and lets Docker choose an available port.
    let additional_host_config = HostConfig {
        memory: Some(512 * 1024 * 1024),
        port_bindings: Some(HashMap::from([(
            "27017/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: None,
            }]),
        )])),
        ..Default::default()
    };
    let deployment2 = client
        .create_deployment_with_host_config(deployment2_options, additional_host_config)
        .await
        .context("creating deployment local 5678 with custom host config")?;

    println!(
        "[{}] \t{}",
        deployment2.mongodb_version,
        deployment2.name.unwrap_or_default()
    );

    Ok(())
}
