use bollard::{
    errors::Error::DockerResponseServerError,
    query_parameters::{CreateContainerOptions, StartContainerOptions},
    secret::ContainerCreateBody,
};

use crate::{
    client::Client,
    docker::{DockerCreateContainer, DockerPullImage, DockerStartContainer},
    models::{ATLAS_LOCAL_IMAGE, CreateDeploymentOptions},
};

use super::PullImageError;

#[derive(Debug, thiserror::Error)]
pub enum CreateDeploymentError {
    #[error("Failed to create container: {0}")]
    CreateContainer(bollard::errors::Error),
    #[error(transparent)]
    PullImage(#[from] PullImageError),
    #[error("Container already exists: {0}")]
    ContainerAlreadyExists(String),
}

impl<D: DockerPullImage + DockerCreateContainer + DockerStartContainer> Client<D> {
    /// Creates a local Atlas deployment.
    pub async fn create_deployment(
        &self,
        deployment_options: &CreateDeploymentOptions,
    ) -> Result<(), CreateDeploymentError> {
        // Pull the latest image for Atlas Local
        self.pull_image(
            deployment_options
                .image
                .as_ref()
                .unwrap_or(&ATLAS_LOCAL_IMAGE.to_string()),
            deployment_options
                .mongodb_version
                .as_ref()
                .map_or_else(|| "latest".to_string(), |version| version.to_string())
                .as_ref(),
        )
        .await?;

        // Create the container with the correct configuration
        let create_container_options: CreateContainerOptions = deployment_options.into();
        let create_container_config: ContainerCreateBody = deployment_options.into();
        let cluster_name = create_container_options
            .name
            .clone()
            .expect("Container name");

        self.docker
            .create_container(Some(create_container_options), create_container_config)
            .await
            .map_err(|err| match err {
                DockerResponseServerError {
                    status_code: 409, ..
                } => CreateDeploymentError::ContainerAlreadyExists(cluster_name.to_string()),
                _ => CreateDeploymentError::CreateContainer(err),
            })?;

        // Start the Atlas Local container
        self.docker
            .start_container(&cluster_name.to_string(), None::<StartContainerOptions>)
            .await
            .map_err(CreateDeploymentError::CreateContainer)?;

        Ok(())
    }
}
