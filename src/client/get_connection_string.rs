use crate::{
    client::get_mongodb_secret::get_mongodb_secret,
    docker::{DockerInspectContainer, RunCommandInContainer, RunCommandInContainerError},
    models::MongoDBPortBinding,
};
use bollard::secret::PortBinding;

use super::GetDeploymentError;

#[derive(Debug, thiserror::Error)]
pub enum GetConnectionStringError {
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
    #[error("Failed to get MongoDB username: {0}")]
    GetMongodbUsername(RunCommandInContainerError),
    #[error("Failed to get MongoDB password: {0}")]
    GetMongodbPassword(RunCommandInContainerError),
    #[error("Missing port binding information")]
    MissingPortBinding,
}

impl<D: DockerInspectContainer + RunCommandInContainer> crate::client::Client<D> {
    // Gets a local Atlas deployment's connection string.
    pub async fn get_connection_string(
        &self,
        container_id_or_name: String,
    ) -> Result<String, GetConnectionStringError> {
        // Get deployment
        let deployment = self.get_deployment(&container_id_or_name).await?;

        // Extract port binding
        let port = match &deployment.port_bindings {
            Some(MongoDBPortBinding { port, .. }) => Some(*port),
            _ => None,
        };
        let port = port
            .flatten()
            .ok_or(GetConnectionStringError::MissingPortBinding)?;

        let hostname = PortBinding::from(
            deployment
                .port_bindings
                .as_ref()
                .ok_or(GetConnectionStringError::MissingPortBinding)?,
        )
        .host_ip
        .ok_or(GetConnectionStringError::MissingPortBinding)?;

        // Try to get the MongoDB root username
        let mongodb_root_username = get_mongodb_secret(
            self.docker.as_ref(),
            &deployment,
            |d| d.mongodb_initdb_root_username.as_deref(),
            |d| d.mongodb_initdb_root_username_file.as_deref(),
        )
        .await
        .map_err(GetConnectionStringError::GetMongodbUsername)?;

        // Try to get the MongoDB root password
        let mongodb_root_password = get_mongodb_secret(
            self.docker.as_ref(),
            &deployment,
            |d| d.mongodb_initdb_root_password.as_deref(),
            |d| d.mongodb_initdb_root_password_file.as_deref(),
        )
        .await
        .map_err(GetConnectionStringError::GetMongodbPassword)?;

        // Construct the connection string
        let connection_string =
            format_connection_string(hostname, mongodb_root_username, mongodb_root_password, port);

        Ok(connection_string)
    }
}

// format_connection_string creates a MongoDB connection string with format depending on presence of username/password.
fn format_connection_string(
    hostname: String,
    username: Option<String>,
    password: Option<String>,
    port: u16,
) -> String {
    let auth_string = match (username, password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => {
            format!("{u}:{p}@")
        }
        _ => "".to_string(),
    };

    format!("mongodb://{auth_string}{hostname}:{port}/?directConnection=true",)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        client::Client,
        docker::{
            CommandOutput, DockerInspectContainer, RunCommandInContainer,
            RunCommandInContainerError,
        },
        test_utils::{
            create_container_inspect_response_no_auth, create_container_inspect_response_with_auth,
        },
    };
    use bollard::{
        errors::Error as BollardError,
        query_parameters::InspectContainerOptions,
        secret::{
            ContainerConfig, ContainerInspectResponse, ContainerState, ContainerStateStatusEnum,
        },
    };
    use maplit::hashmap;
    use mockall::mock;

    mock! {
        Docker {}

        impl DockerInspectContainer for Docker {
            async fn inspect_container(
                &self,
                container_id: &str,
                options: Option<InspectContainerOptions>,
            ) -> Result<ContainerInspectResponse, BollardError>;
        }

        impl RunCommandInContainer for Docker {
            async fn run_command_in_container(
                &self,
                container_id: &str,
                command: Vec<String>,
            ) -> Result<CommandOutput, RunCommandInContainerError>;
        }
    }

    #[tokio::test]
    async fn test_get_connection_string() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Mock call to get_deployment
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(create_container_inspect_response_with_auth(27017)));

        let client = Client::new(mock_docker);
        let container_id_or_name = "test-deployment".to_string();

        // Act
        let result = client.get_connection_string(container_id_or_name).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "mongodb://testuser:testpass@127.0.0.1:27017/?directConnection=true"
        );
    }

    #[tokio::test]
    async fn test_get_connection_string_no_auth() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Mock call to get_deployment
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(create_container_inspect_response_no_auth(27017)));

        let client = Client::new(mock_docker);
        let container_id_or_name = "test-deployment".to_string();

        // Act
        let result = client.get_connection_string(container_id_or_name).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "mongodb://127.0.0.1:27017/?directConnection=true"
        );
    }

    #[tokio::test]
    async fn test_get_connection_string_get_deployment_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Mock call to get_deployment
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
        let container_id_or_name = "nonexistent-deployment".to_string();

        // Act
        let result = client.get_connection_string(container_id_or_name).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetConnectionStringError::GetDeployment(_)
        ));
    }

    #[tokio::test]
    async fn test_get_connection_string_missing_port_binding() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let container_inspect_response = ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "7.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                ..Default::default()
            }),
            network_settings: Some(bollard::secret::NetworkSettings {
                ports: Some(hashmap! {}), // No port mappings
                ..Default::default()
            }),
            ..Default::default()
        };

        // Mock call to get_deployment
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        let client = Client::new(mock_docker);
        let container_id_or_name = "test-deployment".to_string();

        // Act
        let result = client.get_connection_string(container_id_or_name).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetConnectionStringError::MissingPortBinding
        ));
    }

    #[tokio::test]
    async fn test_get_connection_string_verify_success() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Mock call to get_deployment
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(create_container_inspect_response_with_auth(27017)));

        let client = Client::new(mock_docker);

        let container_id_or_name = "test-deployment".to_string();

        // Act
        let result = client.get_connection_string(container_id_or_name).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "mongodb://testuser:testpass@127.0.0.1:27017/?directConnection=true"
        );
    }
}
