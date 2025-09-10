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

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::{errors::Error as BollardError, secret::ContainerCreateResponse};
    use mockall::mock;
    use pretty_assertions::assert_eq;

    mock! {
        Docker {}

        impl DockerPullImage for Docker {
            async fn pull_image(&self, image: &str, tag: &str) -> Result<(), BollardError>;
        }

        impl DockerCreateContainer for Docker {
            async fn create_container(
                &self,
                options: Option<CreateContainerOptions>,
                config: ContainerCreateBody,
            ) -> Result<ContainerCreateResponse, BollardError>;
        }

        impl DockerStartContainer for Docker {
            async fn start_container(
                &self,
                container_id: &str,
                options: Option<StartContainerOptions>,
            ) -> Result<(), BollardError>;
        }
    }

    #[tokio::test]
    async fn test_create_deployment() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_deployment_pull_image_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker.expect_pull_image().times(1).returning(|_, _| {
            Err(BollardError::DockerResponseServerError {
                status_code: 500,
                message: "Internal Server Error".to_string(),
            })
        });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::PullImage(_)
        ));
    }

    #[tokio::test]
    async fn test_create_deployment_container_already_exists() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 409,
                    message: "Conflict".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::ContainerAlreadyExists(name) => {
                assert_eq!(name, "test-deployment");
            }
            _ => panic!("Expected ContainerAlreadyExists error"),
        }
    }

    #[tokio::test]
    async fn test_create_deployment_create_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::CreateContainer(_)
        ));
    }

    #[tokio::test]
    async fn test_create_deployment_start_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::CreateContainer(_)
        ));
    }
}
