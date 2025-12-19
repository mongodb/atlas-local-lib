use bollard::query_parameters::InspectContainerOptions;
use bollard::secret::HealthStatusEnum;
use tokio::time;

use crate::{client::Client, docker::DockerInspectContainer, models::WatchOptions};

#[derive(Debug, thiserror::Error)]
pub enum WatchDeploymentError {
    #[error("Failed to inspect container: {0}")]
    ContainerInspect(#[from] bollard::errors::Error),
    #[error("Timeout while waiting for container {cluster_name} to become healthy")]
    Timeout { cluster_name: String },
    #[error("Deployment {deployment_name} is not healthy [status: {status}]")]
    UnhealthyDeployment {
        deployment_name: String,
        status: HealthStatusEnum,
    },
}

impl<D: DockerInspectContainer> Client<D> {
    /// Waits for a deployment to become healthy.
    ///
    /// This method polls the container's health status until it becomes healthy,
    /// or until the timeout specified in the options is reached.
    ///
    /// # Arguments
    ///
    /// * `cluster_name` - The name or ID of the container to watch
    /// * `options` - Configuration options including timeout duration
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the container becomes healthy, or an error if:
    /// - The container inspection fails
    /// - The container becomes unhealthy
    /// - The timeout is reached
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas_local::models::WatchOptions;
    /// use std::time::Duration;
    ///
    /// # async fn example(client: atlas_local::Client) -> Result<(), Box<dyn std::error::Error>> {
    /// let options = WatchOptions::builder()
    ///     .timeout_duration(Duration::from_secs(300))
    ///     .build();
    ///
    /// client.wait_for_healthy_deployment("my-deployment", options).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_healthy_deployment(
        &self,
        cluster_name: &str,
        options: WatchOptions,
    ) -> Result<(), WatchDeploymentError> {
        let timeout_duration = options
            .timeout_duration
            .unwrap_or(time::Duration::from_secs(60) * 10);
        time::timeout(
            timeout_duration,
            self.wait_for_healthy_deployment_inner(cluster_name, options),
        )
        .await
        .map_err(|_| WatchDeploymentError::Timeout {
            cluster_name: cluster_name.to_string(),
        })?
    }

    async fn wait_for_healthy_deployment_inner(
        &self,
        cluster_name: &str,
        options: WatchOptions,
    ) -> Result<(), WatchDeploymentError> {
        // Loop until the container is healthy
        loop {
            let mut status = self
                .docker
                .inspect_container(cluster_name, None::<InspectContainerOptions>)
                .await
                .map_err(WatchDeploymentError::ContainerInspect)?
                .state
                .and_then(|s| s.health)
                .and_then(|h| h.status)
                .ok_or_else(|| WatchDeploymentError::UnhealthyDeployment {
                    deployment_name: cluster_name.to_string(),
                    status: HealthStatusEnum::NONE,
                })?;

            // If allow_unhealthy_initial_state is set then we handle it as a starting state
            if options.allow_unhealthy_initial_state && status == HealthStatusEnum::UNHEALTHY {
                status = HealthStatusEnum::STARTING;
            }

            match status {
                HealthStatusEnum::HEALTHY => return Ok(()),
                HealthStatusEnum::STARTING => {
                    time::sleep(std::time::Duration::from_secs(1)).await;
                }
                HealthStatusEnum::NONE | HealthStatusEnum::EMPTY | HealthStatusEnum::UNHEALTHY => {
                    return Err(WatchDeploymentError::UnhealthyDeployment {
                        deployment_name: cluster_name.to_string(),
                        status,
                    });
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
            ContainerConfig, ContainerInspectResponse, ContainerState, ContainerStateStatusEnum,
            HealthStatusEnum,
        },
    };
    use maplit::hashmap;
    use mockall::mock;
    use pretty_assertions::assert_eq;

    mock! {
        Docker {}

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

    fn create_test_container_inspect_response_no_state() -> ContainerInspectResponse {
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
            state: None,
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_no_health() -> ContainerInspectResponse {
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
                health: None,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = WatchOptions::builder().build();

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
        let result = client
            .wait_for_healthy_deployment("test-deployment", options)
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_unhealthy() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = WatchOptions::builder().build();

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
        let result = client
            .wait_for_healthy_deployment("test-deployment", options)
            .await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            WatchDeploymentError::UnhealthyDeployment { .. }
        ));
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_retries() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = WatchOptions::builder().build();

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
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .wait_for_healthy_deployment("test-deployment", options)
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_timeout() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = WatchOptions::builder()
            .timeout_duration(time::Duration::from_millis(100))
            .build();

        // Mock inspect_container to always return STARTING status, which will cause timeout
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .returning(|_, _| Ok(create_test_container_inspect_response_starting()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .wait_for_healthy_deployment("test-deployment", options)
            .await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            WatchDeploymentError::Timeout { cluster_name } => {
                assert_eq!(cluster_name, "test-deployment");
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_no_state() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = WatchOptions::builder().build();

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_no_state()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .wait_for_healthy_deployment("test-deployment", options)
            .await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            WatchDeploymentError::UnhealthyDeployment {
                deployment_name,
                status,
            } => {
                assert_eq!(deployment_name, "test-deployment");
                assert_eq!(status, HealthStatusEnum::NONE);
            }
            _ => panic!("Expected UnhealthyDeployment error"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_no_health() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = WatchOptions::builder().build();

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_no_health()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .wait_for_healthy_deployment("test-deployment", options)
            .await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            WatchDeploymentError::UnhealthyDeployment {
                deployment_name,
                status,
            } => {
                assert_eq!(deployment_name, "test-deployment");
                assert_eq!(status, HealthStatusEnum::NONE);
            }
            _ => panic!("Expected UnhealthyDeployment error"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_container_inspect_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = WatchOptions::builder().build();

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 404,
                    message: "No such container".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .wait_for_healthy_deployment("test-deployment", options)
            .await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            WatchDeploymentError::ContainerInspect(_)
        ));
    }
}
