use bollard::query_parameters::{RemoveContainerOptions, StopContainerOptions};

use crate::{
    client::Client,
    docker::{DockerInspectContainer, DockerRemoveContainer, DockerStopContainer},
};

use super::{GetDeploymentError};

#[derive(Debug, thiserror::Error)]
pub enum DeleteDeploymentError {
    #[error("Failed to delete container: {0}")]
    ContainerInspect(#[from] bollard::errors::Error),
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
}

impl<D: DockerStopContainer + DockerRemoveContainer + DockerInspectContainer> Client<D> {
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
}