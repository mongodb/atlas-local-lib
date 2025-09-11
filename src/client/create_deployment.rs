use bollard::{
    errors::Error::DockerResponseServerError,
    query_parameters::{CreateContainerOptions, InspectContainerOptions, StartContainerOptions},
    secret::{ContainerCreateBody, HealthStatusEnum},
};
use tokio::time;

use crate::{
    GetDeploymentError,
    client::Client,
    docker::{
        DockerCreateContainer, DockerInspectContainer, DockerPullImage, DockerStartContainer,
    },
    models::{ATLAS_LOCAL_IMAGE, CreateDeploymentOptions, Deployment},
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
    #[error("Failed to check status of started container: {0}")]
    ContainerInspect(bollard::errors::Error),
    #[error("Created Deployment {0} is not healthy")]
    UnhealthyDeployment(String),
    #[error("Unable to get details for Deployment: {0}")]
    GetDeploymentError(GetDeploymentError),
}

impl<D: DockerPullImage + DockerCreateContainer + DockerStartContainer + DockerInspectContainer>
    Client<D>
{
    /// Creates a local Atlas deployment.
    pub async fn create_deployment(
        &self,
        deployment_options: &CreateDeploymentOptions,
    ) -> Result<Deployment, CreateDeploymentError> {
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

        // Default to waiting for the deployment to be healthy
        if deployment_options.wait_until_healthy.unwrap_or(true) {
            // Default timeout after 10 minutes
            // Container should become unhealthy before the timeout is reached
            let timeout_duration = deployment_options
                .wait_until_healthy_timeout
                .unwrap_or(time::Duration::from_secs(60) * 10);
            time::timeout(
                timeout_duration,
                self.wait_for_healthy_deployment(&cluster_name),
            )
            .await
            .map_err(|_| {
                CreateDeploymentError::UnhealthyDeployment(format!(
                    "Timeout while waiting for container {cluster_name} to become healthy"
                ))
            })
            .flatten()?;
        }

        // Return the deployment details
        self.get_deployment(&cluster_name)
            .await
            .map_err(CreateDeploymentError::GetDeploymentError)
    }
}

impl<D: DockerInspectContainer> Client<D> {
    async fn wait_for_healthy_deployment(
        &self,
        cluster_name: &str,
    ) -> Result<(), CreateDeploymentError> {
        // Loop until the container is healthy
        loop {
            let status = self
                .docker
                .inspect_container(cluster_name, None::<InspectContainerOptions>)
                .await
                .map_err(CreateDeploymentError::ContainerInspect)?
                .state
                .unwrap();

            match status.health.unwrap().status {
                Some(HealthStatusEnum::HEALTHY) => return Ok(()),
                Some(HealthStatusEnum::UNHEALTHY) => {
                    return Err(CreateDeploymentError::UnhealthyDeployment(
                        cluster_name.to_string(),
                    ));
                }
                Some(HealthStatusEnum::STARTING) => {
                    time::sleep(std::time::Duration::from_secs(1)).await;
                }
                _ => {
                    return Err(CreateDeploymentError::UnhealthyDeployment(
                        cluster_name.to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::{
        errors::Error as BollardError,
        secret::{
            ContainerConfig, ContainerCreateResponse, ContainerInspectResponse, ContainerState,
            ContainerStateStatusEnum,
        },
    };
    use maplit::hashmap;
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

        impl DockerInspectContainer for Docker {
            async fn inspect_container(
                &self,
                container_id: &str,
                options: Option<InspectContainerOptions>,
            ) -> Result<ContainerInspectResponse, BollardError>;
        }
    }

    fn create_test_container_inspect_response() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                health: Some(bollard::secret::Health {
                    status: Some(HealthStatusEnum::HEALTHY),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_unhealthy() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                health: Some(bollard::secret::Health {
                    status: Some(HealthStatusEnum::UNHEALTHY),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_starting() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                health: Some(bollard::secret::Health {
                    status: Some(HealthStatusEnum::STARTING),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
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

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(2)
            .returning(|_, _| Ok(create_test_container_inspect_response()));

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

    #[tokio::test]
    async fn test_create_deployment_wait_for_healthy_deployment_unhealthy() {
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

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_unhealthy()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::UnhealthyDeployment(_)
        ));
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_retries() {
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

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_starting()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(2)
            .returning(|_, _| Ok(create_test_container_inspect_response()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_disabled() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            wait_until_healthy: Some(false),
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

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(&options).await;

        // Assert
        assert!(result.is_ok());
    }
}
