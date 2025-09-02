#![doc = include_str!("../README.md")]

use bollard::{
    Docker,
    query_parameters::{
        InspectContainerOptions, ListContainersOptionsBuilder, RemoveContainerOptions,
        StopContainerOptions,
    },
};
use maplit::hashmap;

use crate::models::{
    Deployment, IntoDeploymentError, LOCAL_DEPLOYMENT_LABEL_KEY, LOCAL_DEPLOYMENT_LABEL_VALUE,
};

pub mod models;

/// The main entry point for interacting with local Atlas deployments.
///
/// `Client` provides a high-level interface for managing MongoDB Atlas local deployments
/// through Docker. It serves as the primary abstraction layer between your application
/// and the underlying Docker containers running Atlas services.
///
/// # Examples
///
/// See the [module-level documentation](crate) for a complete example of creating
/// a new client instance.
pub struct Client {
    #[allow(dead_code)] // TODO: remove this once we have methods on the client struct
    docker: Docker,
}

impl Client {
    /// Creates a new Atlas Local client.
    ///
    /// # Arguments
    ///
    /// * `docker` - A connected Docker client instance from the `bollard` crate
    ///
    /// # Returns
    ///
    /// A new `Client` instance ready to manage Atlas Local deployments.
    ///
    /// # Examples
    ///
    /// See the [module-level documentation](crate) for usage examples.
    pub fn new(docker: Docker) -> Client {
        Client { docker }
    }

    /// Deletes a local Atlas deployment.
    pub async fn delete_deployment(&self, name: &str) -> Result<(), DeleteDeploymentError> {
        // Check that a deployment with that name exists and get the container ID.
        // This ensures we only try to delete valid Atlas local deployments.
        let deployment = self
            .get_deployment(name)
            .await
            .map_err(DeleteDeploymentError::GetDeployment)?;
        let container_id = deployment.container_id.as_str();

        // Attempt to stop the container gracefully before removal.
        self.docker
            .stop_container(container_id, None::<StopContainerOptions>)
            .await?;

        // Remove the container from Docker.
        self.docker
            .remove_container(container_id, None::<RemoveContainerOptions>)
            .await?;

        Ok(())
    }

    /// Lists all local Atlas deployments.
    pub async fn list_deployments(&self) -> Result<Vec<Deployment>, GetDeploymentError> {
        // Build the list containers options which will filter for containers with the local deployment label
        let list_container_options = ListContainersOptionsBuilder::default()
            .all(true)
            .filters(&hashmap! {
                "label" => vec![format!("{}={}", LOCAL_DEPLOYMENT_LABEL_KEY, LOCAL_DEPLOYMENT_LABEL_VALUE)],
            })
            .build();

        // Get all the containers using the list containers options
        let container_summaries = self
            .docker
            .list_containers(Some(list_container_options))
            .await?;

        // Create the output vector used to return the deployments
        let mut deployments = Vec::with_capacity(container_summaries.len());

        // Iterate over the container summaries and get the deployment details for each container
        for container_summary in container_summaries {
            // Get the container ID from the container summary
            // This should always be present, but it's cleaner to not use unwrap and skip if it's not present
            if let Some(container_id) = container_summary.id {
                // Get the deployment details for the container
                let deployment = self.get_deployment(container_id.as_ref()).await?;
                deployments.push(deployment);
            }
        }

        Ok(deployments)
    }

    pub async fn get_deployment(
        &self,
        container_id_or_name: &str,
    ) -> Result<Deployment, GetDeploymentError> {
        // Inspect the container to get the deployment details
        let container_inspect_response = self
            .docker
            .inspect_container(container_id_or_name, None::<InspectContainerOptions>)
            .await?;

        // Convert the container inspect response to a deployment
        Ok(container_inspect_response.try_into()?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetDeploymentError {
    #[error("Failed to inspect container: {0}")]
    ContainerInspect(#[from] bollard::errors::Error),
    #[error("The container is not a local Atlas deployment: {0}")]
    IntoDeployment(#[from] IntoDeploymentError),
}
#[derive(Debug, thiserror::Error)]
pub enum DeleteDeploymentError {
    #[error("Failed to delete container: {0}")]
    ContainerInspect(#[from] bollard::errors::Error),
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
}
