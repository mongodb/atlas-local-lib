use crate::{
    client::Client,
    docker::{DockerInspectContainer},
    models::{GetConnectionStringOptions, MongoDBPortBinding},
};
use bollard::{query_parameters::InspectContainerOptions, Docker};
use mongodb::{Client as MongoClient, options::ClientOptions};

use super::GetDeploymentError;

#[derive(Debug, thiserror::Error)]
pub enum GetConnectionStringError {
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
    #[error("Missing port binding information")]
    MissingPortBinding,
    #[error("Failed to connect to MongoDB: {0}")]
    MongoConnect(#[from] mongodb::error::Error),
}

impl<D: DockerInspectContainer> Client<D> {
    // Gets a local Atlas deployment's connection string.
    pub async fn get_connection_string<'a>(
        &self,
        req: GetConnectionStringOptions<'a>,
    ) -> Result<String, GetConnectionStringError> {
        // Get deployment
        let deployment = self.get_deployment(req.container_id_or_name).await?;

        // Extract port binding
        let port = match &deployment.port_bindings {
            Some(MongoDBPortBinding { port, .. }) => Some(*port),
            _ => None,
        };
        let port = port.flatten().ok_or(GetConnectionStringError::MissingPortBinding)?;

        let hostname = get_hostname().await.map_err(|_| GetConnectionStringError::MissingPortBinding)?;
        // Construct the connection string
        let connection_string = format_connection_string(&hostname,req.db_username, req.db_password, port);
        print!("Connection String: {}", connection_string);

        // Optionally, verify the connection string
        if req.verify.unwrap_or(false) {
            verify_connection_string(&connection_string)
                .await
                .map_err(GetConnectionStringError::MongoConnect)?;
        }

        Ok(connection_string)
    }
}

pub async fn get_dind_host_ip() -> Option<String> {
    let docker = Docker::connect_with_socket_defaults().ok()?;
    // "docker" is the default service name in GitHub Actions
    let inspect = docker.inspect_container("docker", None::<InspectContainerOptions>).await.ok()?;
    let network_settings = inspect.network_settings?;
    let networks = network_settings.networks?;
    // Get the first network's IPAddress, print and return it
    if let Some(ip) = networks.values().next()?.ip_address.clone() {
        return Some(ip);
    }
    None
}

async fn get_hostname() -> std::io::Result<String> {
    if std::path::Path::new("/.dockerenv").exists() {
        print!("In docker, searching for Docker socket...");
        if std::path::Path::new("/var/run/docker.sock").exists() {
            print!("Detected Docker socket, attempting to get host IP from 'docker' container...");
            if let Some(ip) = get_dind_host_ip().await {
                print!("Found Docker host IP: {}", ip);
                return Ok(ip);
            }
        }
    }
    print!("Could not find Docker host IP, defaulting to 127.0.0.1");
    Ok("127.0.0.1".to_string())
}

// format_connection_string creates a MongoDB connection string with format depending on presence of username/password.
fn format_connection_string(hostname: &str, username: Option<&str>, password: Option<&str>, port: u16) -> String {
    let auth_string = match (username, password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => {
            format!("{u}:{p}@")
        }
        _ => "".to_string(),
    };

    format!("mongodb://{auth_string}{hostname}:{port}/?directConnection=true",)
}

// verify_connection_string verifies the provided connection string by attempting to connect to MongoDB and running a simple command.
async fn verify_connection_string(connection_string: &str) -> Result<(), mongodb::error::Error> {
    let client_options = ClientOptions::parse(connection_string).await?;
    let mongo_client = MongoClient::with_options(client_options)?;
    mongo_client.list_database_names().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
    }

    fn create_container_inspect_response_with_auth(port: u16) -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "7.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                ..Default::default()
            }),
            network_settings: Some(bollard::secret::NetworkSettings {
                ports: Some(hashmap! {
                    "27017/tcp".to_string() => Some(vec![
                        bollard::secret::PortBinding {
                            host_ip: Some("127.0.0.1".to_string()),
                            host_port: Some(port.to_string()),
                        }
                    ])
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_container_inspect_response_no_auth(port: u16) -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "7.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                ..Default::default()
            }),
            network_settings: Some(bollard::secret::NetworkSettings {
                ports: Some(hashmap! {
                    "27017/tcp".to_string() => Some(vec![
                        bollard::secret::PortBinding {
                            host_ip: Some("127.0.0.1".to_string()),
                            host_port: Some(port.to_string()),
                        }
                    ])
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_get_connection_string() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(create_container_inspect_response_with_auth(27017)));

        let client = Client::new(mock_docker);
        let req = GetConnectionStringOptions {
            container_id_or_name: "test-deployment",
            db_username: Some("testuser"),
            db_password: Some("testpass"),
            verify: None,
        };

        // Act
        let result = client.get_connection_string(req).await;

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

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(create_container_inspect_response_no_auth(27017)));

        let client = Client::new(mock_docker);
        let req = GetConnectionStringOptions {
            container_id_or_name: "test-deployment",
            db_username: None,
            db_password: None,
            verify: None,
        };

        // Act
        let result = client.get_connection_string(req).await;

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
        let req = GetConnectionStringOptions {
            container_id_or_name: "nonexistent-deployment",
            db_username: None,
            db_password: None,
            verify: None,
        };

        // Act
        let result = client.get_connection_string(req).await;

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

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        let client = Client::new(mock_docker);
        let req = GetConnectionStringOptions {
            container_id_or_name: "test-deployment",
            db_username: None,
            db_password: None,
            verify: None,
        };

        // Act
        let result = client.get_connection_string(req).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetConnectionStringError::MissingPortBinding
        ));
    }
}
