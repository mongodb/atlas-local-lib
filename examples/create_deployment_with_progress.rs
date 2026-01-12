use std::io::{self, Write};

use anyhow::{Context, Result};
use atlas_local::{Client, client::CreateDeploymentStepOutcome};
use bollard::Docker;
use tokio::sync::oneshot::error::RecvError;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_defaults().context("connecting to docker")?;
    let client = Client::new(docker);

    // Create a deployment with an automatically generated name, and default settings
    // More details about sample data can be found here: https://docs.mongodb.com/atlas/sample-data/
    let mut create_deployment_progress = client.create_deployment(Default::default());

    // Print the progress of each step
    // Waiting for steps to complete is optional
    print_step(
        "Pulling the latest version of the MongoDB image",
        create_deployment_progress.wait_for_pull_image_outcome(),
    )
    .await?;

    print_step(
        "Creating the deployment",
        create_deployment_progress.wait_for_create_container_outcome(),
    )
    .await?;

    print_step(
        "Starting the deployment",
        create_deployment_progress.wait_for_start_container_outcome(),
    )
    .await?;

    print_step(
        "Waiting for the deployment to be healthy",
        create_deployment_progress.wait_for_wait_for_healthy_deployment_outcome(),
    )
    .await?;

    let deployment = create_deployment_progress
        .await
        .context("waiting for deployment to complete")?;

    println!(
        "[{}] \t{}",
        deployment.mongodb_version,
        deployment.name.unwrap_or_default()
    );

    Ok(())
}

async fn print_step(
    step: &str,
    step_future: impl Future<Output = Result<CreateDeploymentStepOutcome, RecvError>>,
) -> Result<()> {
    // Print the step message with a fixed width of 50 characters
    print!("{step:<50}");
    io::stdout().flush()?;

    // Wait for the step to complete and convert the outcome to a string
    let outcome = match step_future.await.context("waiting for step to complete")? {
        CreateDeploymentStepOutcome::Success => "Success",
        CreateDeploymentStepOutcome::Skipped => "Skipped",
        CreateDeploymentStepOutcome::Failure => "Failure",
    };

    // Print the outcome with a fixed width of 10 characters
    println!(" {outcome:<10}");

    Ok(())
}
