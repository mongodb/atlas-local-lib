use anyhow::{Context, Result};
use atlas_local::{
    Client,
    models::{CreateDeploymentOptions, LogsOptions, Tail},
};
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

    // Configure log options
    let log_options = LogsOptions::builder()
        .stdout(true) // Include stdout
        .stderr(true) // Include stderr
        .tail(Tail::Number(100)) // Get last 100 lines
        .timestamps(true) // Include timestamps
        .build();

    // Get logs from the deployment
    let logs = client
        .get_logs(&deployment.container_id, Some(log_options))
        .await
        .context("getting logs")?;

    println!("Container logs:");
    for log in logs {
        // Use print! instead of println! because logs contain new line characters
        print!("{}", log);
    }

    Ok(())
}
