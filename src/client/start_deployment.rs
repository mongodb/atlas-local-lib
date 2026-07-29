use bollard::query_parameters::StartContainerOptions;

use crate::{
    client::Client,
    docker::{DockerInspectContainer, DockerStartContainer},
    models::{StartDeploymentOptions, WatchOptions},
};

use super::{GetDeploymentError, WatchDeploymentError};

#[derive(Debug, thiserror::Error)]
pub enum StartDeploymentError {
    #[error("Failed to start container: {0}")]
    ContainerStart(String),
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
    #[error("Error when waiting for deployment to become healthy: {0}")]
    WatchDeployment(#[from] WatchDeploymentError),
}

impl<D: DockerStartContainer + DockerInspectContainer> Client<D> {
    /// Starts a local Atlas deployment.
    ///
    /// By default this waits for the deployment to become healthy before
    /// returning. Set [`StartDeploymentOptions::wait_until_healthy`] to `false`
    /// to return as soon as the container has been started.
    pub async fn start_deployment(
        &self,
        name: &str,
        options: StartDeploymentOptions,
    ) -> Result<(), StartDeploymentError> {
        // Check that a deployment with that name exists and get the container ID.
        // This ensures we only try to start valid Atlas local deployments.
        let deployment = self.get_deployment(name).await?;
        let container_id = deployment.container_id.as_str();

        // Start the container.
        self.docker
            .start_container(container_id, None::<StartContainerOptions>)
            .await
            .map_err(|e| StartDeploymentError::ContainerStart(e.to_string()))?;

        // Default to waiting for the deployment to be healthy, so that is
        // ready to accept connections when this returns.
        if options.wait_until_healthy.unwrap_or(true) {
            let watch_options = WatchOptions {
                timeout_duration: options.wait_until_healthy_timeout,
                allow_unhealthy_initial_state: true,
            };
            self.wait_for_healthy_deployment(name, watch_options)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{docker::DockerError, models::ContainerHealthStatus};
    use bollard::{
        models::{ContainerInspectResponse, Health, HealthStatusEnum},
        query_parameters::InspectContainerOptions,
    };
    use mockall::mock;

    mock! {
        Docker {}

        impl DockerStartContainer for Docker {
            async fn start_container(
                &self,
                container_id: &str,
                options: Option<StartContainerOptions>,
            ) -> Result<(), DockerError>;
        }

        impl DockerInspectContainer for Docker {
            async fn inspect_container(
                &self,
                container_id: &str,
                options: Option<InspectContainerOptions>,
            ) -> Result<ContainerInspectResponse, DockerError>;
        }
    }

    fn create_healthy_test_container_inspect_response() -> ContainerInspectResponse {
        create_test_container_inspect_response_with_health(Some(HealthStatusEnum::HEALTHY))
    }

    fn create_test_container_inspect_response_with_health(
        health: Option<HealthStatusEnum>,
    ) -> ContainerInspectResponse {
        use bollard::models::{ContainerConfig, ContainerState, ContainerStateStatusEnum};
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert("mongodb-atlas-local".to_string(), "container".to_string());
        labels.insert("version".to_string(), "8.0.0".to_string());
        labels.insert("mongodb-type".to_string(), "community".to_string());

        let env_vars = vec!["TOOL=ATLASCLI".to_string()];

        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(labels),
                env: Some(env_vars),
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                health: health.map(|status| Health {
                    status: Some(status),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_start_deployment_waits_until_healthy() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(2)
            .returning(move |_, _| Ok(create_healthy_test_container_inspect_response()));

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test_container_id"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .start_deployment("test-deployment", StartDeploymentOptions::default())
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_deployment_waits_while_starting() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut calls = 0;

        mock_docker
            .expect_inspect_container()
            .times(3)
            .returning(move |_, _| {
                calls += 1;
                // First call resolves the deployment, then STARTING, then HEALTHY.
                Ok(match calls {
                    2 => create_test_container_inspect_response_with_health(Some(
                        HealthStatusEnum::STARTING,
                    )),
                    _ => create_healthy_test_container_inspect_response(),
                })
            });

        mock_docker
            .expect_start_container()
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .start_deployment("test-deployment", StartDeploymentOptions::default())
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_deployment_unhealthy_initial_state_is_allowed() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut calls = 0;

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(3)
            .returning(move |_, _| {
                calls += 1;
                Ok(match calls {
                    2 => create_test_container_inspect_response_with_health(Some(
                        HealthStatusEnum::UNHEALTHY,
                    )),
                    _ => create_healthy_test_container_inspect_response(),
                })
            });

        mock_docker
            .expect_start_container()
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .start_deployment("test-deployment", StartDeploymentOptions::default())
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_deployment_unhealthy_times_out() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = StartDeploymentOptions::builder()
            .wait_until_healthy_timeout(std::time::Duration::from_millis(50))
            .build();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(2)
            .returning(|_, _| {
                Ok(create_test_container_inspect_response_with_health(Some(
                    HealthStatusEnum::UNHEALTHY,
                )))
            });

        mock_docker
            .expect_start_container()
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.start_deployment("test-deployment", options).await;

        // Assert
        assert!(matches!(
            result,
            Err(StartDeploymentError::WatchDeployment(
                WatchDeploymentError::Timeout { .. }
            ))
        ));
    }

    #[tokio::test]
    async fn test_start_deployment_watch_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut calls = 0;

        mock_docker
            .expect_inspect_container()
            .times(2)
            .returning(move |_, _| {
                calls += 1;
                // The health check finds no health information at all.
                Ok(if calls == 1 {
                    create_healthy_test_container_inspect_response()
                } else {
                    create_test_container_inspect_response_with_health(None)
                })
            });

        mock_docker
            .expect_start_container()
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .start_deployment("test-deployment", StartDeploymentOptions::default())
            .await;

        // Assert
        assert!(matches!(
            result,
            Err(StartDeploymentError::WatchDeployment(
                WatchDeploymentError::UnhealthyDeployment {
                    status: ContainerHealthStatus::None,
                    ..
                }
            ))
        ));
    }

    #[tokio::test]
    async fn test_start_deployment_without_waiting() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(create_healthy_test_container_inspect_response()));

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test_container_id"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .start_deployment(
                "test-deployment",
                StartDeploymentOptions::builder()
                    .wait_until_healthy(false)
                    .build(),
            )
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_deployment_get_deployment_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(1)
            .returning(|_, _| Err(DockerError::NotFound));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .start_deployment("nonexistent-deployment", StartDeploymentOptions::default())
            .await;

        // Assert
        assert!(matches!(
            result,
            Err(StartDeploymentError::GetDeployment(
                GetDeploymentError::ContainerInspect(DockerError::NotFound)
            ))
        ));
    }

    #[tokio::test]
    async fn test_start_deployment_start_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(1)
            .returning(move |_, _| Ok(create_healthy_test_container_inspect_response()));

        mock_docker
            .expect_start_container()
            .times(1)
            .returning(|_, _| Err(DockerError::ServerError));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .start_deployment("test-deployment", StartDeploymentOptions::default())
            .await;

        // Assert
        assert!(matches!(
            result,
            Err(StartDeploymentError::ContainerStart(_))
        ));
    }
}
