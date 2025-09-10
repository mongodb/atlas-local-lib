use bollard::query_parameters::{RemoveContainerOptions, StopContainerOptions};

use crate::{
    client::Client,
    docker::{DockerInspectContainer, DockerRemoveContainer, DockerStopContainer},
};

use super::GetDeploymentError;

#[derive(Debug, thiserror::Error)]
pub enum DeleteDeploymentError {
    #[error("Failed to delete container: {0}")]
    ContainerStop(bollard::errors::Error),
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
    #[error("Failed to delete remove: {0}")]
    ContainerRemove(bollard::errors::Error),
}

impl<D: DockerStopContainer + DockerRemoveContainer + DockerInspectContainer> Client<D> {
    /// Deletes a local Atlas deployment.
    pub async fn delete_deployment(&self, name: &str) -> Result<(), DeleteDeploymentError> {
        // Check that a deployment with that name exists and get the container ID.
        // This ensures we only try to delete valid Atlas local deployments.
        let deployment = self.get_deployment(name).await?;
        let container_id = deployment.container_id.as_str();

        // Attempt to stop the container gracefully before removal.
        self.docker
            .stop_container(container_id, None::<StopContainerOptions>)
            .await
            .map_err(DeleteDeploymentError::ContainerStop)?;

        // Remove the container from Docker.
        self.docker
            .remove_container(container_id, None::<RemoveContainerOptions>)
            .await
            .map_err(DeleteDeploymentError::ContainerRemove)?;

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

        impl DockerStopContainer for Docker {
            async fn stop_container(
                &self,
                container_id: &str,
                options: Option<StopContainerOptions>,
            ) -> Result<(), BollardError>;
        }

        impl DockerRemoveContainer for Docker {
            async fn remove_container(
                &self,
                container_id: &str,
                options: Option<RemoveContainerOptions>,
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
    async fn test_delete_deployment() {
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
            .expect_stop_container()
            .with(
                mockall::predicate::eq("test_container_id"),
                mockall::predicate::eq(None::<StopContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_remove_container()
            .with(
                mockall::predicate::eq("test_container_id"),
                mockall::predicate::eq(None::<RemoveContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.delete_deployment("test-deployment").await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_deployment_get_deployment_error() {
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
        let result = client.delete_deployment("nonexistent-deployment").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DeleteDeploymentError::GetDeployment(_)
        ));
    }

    #[tokio::test]
    async fn test_delete_deployment_stop_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(1)
            .returning(move |_, _| Ok(create_test_container_inspect_response()));

        mock_docker
            .expect_stop_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.delete_deployment("test-deployment").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DeleteDeploymentError::ContainerStop(_)
        ));
    }

    #[tokio::test]
    async fn test_delete_deployment_remove_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .times(1)
            .returning(move |_, _| Ok(create_test_container_inspect_response()));

        mock_docker
            .expect_stop_container()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_remove_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.delete_deployment("test-deployment").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DeleteDeploymentError::ContainerRemove(_)
        ));
    }
}
