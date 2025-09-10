use bollard::query_parameters::InspectContainerOptions;

use crate::{
    client::Client,
    docker::DockerInspectContainer,
    models::{Deployment, IntoDeploymentError},
};

#[derive(Debug, thiserror::Error)]
pub enum GetDeploymentError {
    #[error("Failed to inspect container: {0}")]
    ContainerInspect(#[from] bollard::errors::Error),
    #[error("The container is not a local Atlas deployment: {0}")]
    IntoDeployment(#[from] IntoDeploymentError),
}

impl<D: DockerInspectContainer> Client<D> {
    /// Inspects a container.
    ///
    /// # Arguments
    ///
    /// * `container_id_or_name` - The ID or name of the container to inspect.
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