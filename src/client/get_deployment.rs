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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreationSource, MongodbType, State};
    use bollard::{
        errors::Error as BollardError,
        secret::{
            ContainerConfig, ContainerInspectResponse, ContainerState, ContainerStateStatusEnum,
        },
    };
    use maplit::hashmap;
    use mockall::mock;
    use semver::Version;
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

    #[tokio::test]
    async fn test_get_deployment() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let container_inspect_response = ContainerInspectResponse {
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
                ..Default::default()
            }),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment("test-deployment").await;

        // Assert
        assert!(result.is_ok());

        assert_eq!(
            result.unwrap(),
            Deployment {
                container_id: "test_container_id".to_string(),
                name: Some("test-deployment".to_string()),
                state: State::Running,
                mongodb_type: MongodbType::Community,
                mongodb_version: Version::new(8, 0, 0),
                port_bindings: None,
                creation_source: Some(CreationSource::AtlasCLI),
                local_seed_location: None,
                mongodb_initdb_database: None,
                mongodb_initdb_root_password_file: None,
                mongodb_initdb_root_password: None,
                mongodb_initdb_root_username_file: None,
                mongodb_initdb_root_username: None,
                mongot_log_file: None,
                runner_log_file: None,
                do_not_track: None,
                telemetry_base_url: None,
            }
        );
    }

    #[tokio::test]
    async fn test_get_deployment_container_inspect_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("nonexistent-deployment"),
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
        let result = client.get_deployment("nonexistent-deployment").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentError::ContainerInspect(_)
        ));
    }

    #[tokio::test]
    async fn test_get_deployment_into_deployment_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let container_inspect_response = ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: None, // Missing labels will cause IntoDeploymentError
                ..Default::default()
            }),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("invalid-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment("invalid-deployment").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentError::IntoDeployment(_)
        ));
    }
}
