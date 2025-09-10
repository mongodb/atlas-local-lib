use mongodb::{options::ClientOptions, Client as MongoClient};
use crate::{
    client::Client,
    docker::DockerInspectContainer,
    models::{MongoDBPortBinding, GetConnectionStringOptions},
};

use super::GetDeploymentError;

#[derive(Debug, thiserror::Error)]
pub enum GetConnectionStringError {
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
    #[error("Missing port binding information")]
    MissingPortBinding,
    #[error("Failed to connect to MongoDB: {0}")]
    MongoConnect(#[from] mongodb::error::Error),
}

impl<D: DockerInspectContainer> Client<D> {
    // Gets a local Atlas deployment's connection string.
    pub async fn get_connection_string<'a>(
        &self,
        req: GetConnectionStringOptions<'a>,
    ) -> Result<String, GetConnectionStringError> {
        // Get deployment
        let deployment = self.get_deployment(req.container_id_or_name).await?;

        // Extract port binding and auth credentials
        let port = match &deployment.port_bindings {
            Some(MongoDBPortBinding { port, .. }) => *port,
            _ => return Err(GetConnectionStringError::MissingPortBinding),
        };
        // No error is returned if username or password is missing - just assume no auth is set
        let username = deployment
            .mongodb_initdb_root_username
            .as_deref()
            .unwrap_or("");
        let password = deployment
            .mongodb_initdb_root_password
            .as_deref()
            .unwrap_or("");

        // Construct the connection string
        let connection_string = if username.is_empty() && password.is_empty() {
            format!("mongodb://localhost:{}/?directConnection=true", port)
        } else {
            format!(
                "mongodb://{}:{}@localhost:{}/?directConnection=true",
                username, password, port
            )
        };

        // Optionally verify the connection string by connecting to MongoDB and executing a simple command
        if req.verify.unwrap_or(false) {
            let client_options = ClientOptions::parse(&connection_string).await?;
            let mongo_client = MongoClient::with_options(client_options)?;
            mongo_client.list_database_names().await?;
        }

        Ok(connection_string)
    }
}
