use anyhow::{Context, Result};
use atlas_local::Client;
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_defaults().context("connecting to docker")?;
    let client = Client::new(docker.clone());

    let deployments = client
        .list_deployments()
        .await
        .context("listing deployments")?;

    println!("DEPLOYMENT \t CONNECTION STRING");
    for deployment in deployments {
        let container_id_or_name = deployment.container_id;
        let conn_str = client
            .get_connection_string(container_id_or_name.to_string())
            .await
            .unwrap_or_else(|e| format!("Error: {}", e));

        println!("[{}] \t{}", deployment.name.unwrap_or_default(), conn_str);
    }

    Ok(())
}
