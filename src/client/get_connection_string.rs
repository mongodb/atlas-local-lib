use crate::{
    client::Client,
    docker::DockerInspectContainer,
    models::{GetConnectionStringOptions, MongoDBPortBinding},
    mongodb::MongoDbClient,
};
use bollard::secret::PortBinding;

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
    pub async fn get_connection_string(
        &self,
        req: GetConnectionStringOptions,
    ) -> Result<String, GetConnectionStringError> {
        // Get deployment
        let deployment = self.get_deployment(&req.container_id_or_name).await?;

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

        // Construct the connection string
        let connection_string =
            format_connection_string(hostname, req.db_username, req.db_password, port);

        // Optionally, verify the connection string
        if req.verify.unwrap_or(false) {
            verify_connection_string(&connection_string, self.mongodb_client.as_ref())
                .await
                .map_err(GetConnectionStringError::MongoConnect)?;
        }

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

// verify_connection_string verifies the provided connection string by attempting to connect to MongoDB and running a simple command.
async fn verify_connection_string(
    connection_string: &str,
    mongo_client: &dyn MongoDbClient,
) -> Result<(), mongodb::error::Error> {
    let _database = mongo_client.list_database_names(connection_string).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GetDeploymentIdError,
        mongodb::MongoDbClient,
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
    }

    mock! {
        MongoClientFactory {}

        #[async_trait::async_trait]
        impl MongoDbClient for MongoClientFactory {
            async fn list_database_names(&self, connection_string: &str) -> Result<Vec<String>, mongodb::error::Error>;
            async fn get_deployment_id(&self, connection_string: &str) -> Result<String, GetDeploymentIdError>;
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

        let client =
            Client::with_mongo_client_factory(mock_docker, Box::new(MockMongoClientFactory::new()));
        let req = GetConnectionStringOptions {
            container_id_or_name: "test-deployment".to_string(),
            db_username: Some("testuser".to_string()),
            db_password: Some("testpass".to_string()),
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

        let mock_mongo_client = MockMongoClientFactory::new();
        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));
        let req = GetConnectionStringOptions {
            container_id_or_name: "test-deployment".to_string(),
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

        let mock_mongo_client = MockMongoClientFactory::new();
        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));
        let req = GetConnectionStringOptions {
            container_id_or_name: "nonexistent-deployment".to_string(),
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

        let mock_mongo_client = MockMongoClientFactory::new();
        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));
        let req = GetConnectionStringOptions {
            container_id_or_name: "test-deployment".to_string(),
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

    #[tokio::test]
    async fn test_get_connection_string_verify_success() {
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

        let mut mock_mongo_client = MockMongoClientFactory::new();
        mock_mongo_client
            .expect_list_database_names()
            .with(mockall::predicate::eq(
                "mongodb://testuser:testpass@127.0.0.1:27017/?directConnection=true",
            ))
            .times(1)
            .returning(|_| Ok(vec!["admin".to_string(), "test".to_string()]));

        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));

        let req = GetConnectionStringOptions {
            container_id_or_name: "test-deployment".to_string(),
            db_username: Some("testuser".to_string()),
            db_password: Some("testpass".to_string()),
            verify: Some(true),
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
}
