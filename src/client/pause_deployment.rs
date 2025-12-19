use crate::{
    client::Client,
    docker::{DockerInspectContainer, DockerPauseContainer},
};

use super::GetDeploymentError;

#[derive(Debug, thiserror::Error)]
pub enum PauseDeploymentError {
    #[error("Failed to pause container: {0}")]
    ContainerPause(String),
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
}

impl<D: DockerPauseContainer + DockerInspectContainer> Client<D> {
    /// Pauses a local Atlas deployment.
    pub async fn pause_deployment(&self, name: &str) -> Result<(), PauseDeploymentError> {
        // Check that a deployment with that name exists and get the container ID.
        // This ensures we only try to pause valid Atlas local deployments.
        let deployment = self.get_deployment(name).await?;
        let container_id = deployment.container_id.as_str();

        // Pause the container.
        self.docker
            .pause_container(container_id)
            .await
            .map_err(|e| PauseDeploymentError::ContainerPause(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::{
        errors::Error as BollardError, query_parameters::InspectContainerOptions,
        secret::ContainerInspectResponse,
    };
    use mockall::mock;

    mock! {
        Docker {}

        impl DockerPauseContainer for Docker {
            async fn pause_container(&self, container_id: &str) -> Result<(), BollardError>;
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
        use bollard::secret::{ContainerConfig, ContainerState, ContainerStateStatusEnum};
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
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_pause_deployment() {
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
            .returning(move |_, _| Ok(create_test_container_inspect_response()));

        mock_docker
            .expect_pause_container()
            .with(mockall::predicate::eq("test_container_id"))
            .times(1)
            .returning(|_| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.pause_deployment("test-deployment").await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pause_deployment_get_deployment_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 404,
                    message: "No such container".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.pause_deployment("nonexistent-deployment").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PauseDeploymentError::GetDeployment(_)
        ));
    }

    #[tokio::test]
    async fn test_pause_deployment_pause_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(1)
            .returning(move |_, _| Ok(create_test_container_inspect_response()));

        mock_docker
            .expect_pause_container()
            .times(1)
            .returning(|_| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.pause_deployment("test-deployment").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PauseDeploymentError::ContainerPause(_)
        ));
    }
}
