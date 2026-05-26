use anyhow::{Context, Result};
use atlas_local::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect_with_socket_defaults().context("connecting to docker")?;

    client
        .delete_deployment("local1234")
        .await
        .context("Deleting atlas local container local1234")?;
    println!("local1234 successfully deleted");

    // This should fail as the container is not a local Atlas container
    client
        .delete_deployment("other_none_local_atlas")
        .await
        .context("Attempting to delete other non-atlas container")?;

    Ok(())
}
