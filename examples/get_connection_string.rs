use anyhow::{Context, Result};
use atlas_local::{models::GetConnectionStringOptions, Client};
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_socket_defaults().context("connecting to docker")?;
    let client = Client::new(docker);

    let deployments = client
        .list_deployments()
        .await
        .context("listing deployments")?;

    println!("DEPLOYMENT \t CONNECTION STRING");
    for deployment in deployments {
        let username = &deployment.mongodb_initdb_root_username.clone().unwrap_or_default();
        let password = &deployment.mongodb_initdb_root_password.clone().unwrap_or_default();
        

        let req = GetConnectionStringOptions {
            container_id_or_name: &deployment.container_id,
            db_username: Some(username),
            db_password: Some(password),
            verify: Some(true),
        };
        let conn_str = client
        .get_connection_string(req)
        .await
        .unwrap_or_else(|e| format!("Error: {}", e));

        println!(
            "[{}] \t{}",
            deployment.name.unwrap_or_default(),
            conn_str
        );
    }

    Ok(())
}
