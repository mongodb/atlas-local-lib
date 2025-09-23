use mongodb::error::Error;

use crate::{Client, docker::DockerInspectContainer, models::GetConnectionStringOptions};

#[derive(Debug, thiserror::Error)]
pub enum GetDeploymentIdError {
    #[error("Failed to get connection string: {0}")]
    GetConnectionString(#[from] crate::client::get_connection_string::GetConnectionStringError),
    #[error("Failed to connect to MongoDB: {0}")]
    MongoConnect(#[from] Error),
    #[error("Could not find {0}")]
    NotFound(String),
}

impl<D: DockerInspectContainer> Client<D> {
    /// Gets the Atlas deployment ID for a local Atlas deployment.
    pub async fn get_deployment_id(
        &self,
        cluster_id_or_name: &str,
    ) -> Result<String, GetDeploymentIdError> {
        let get_connection_string_options = GetConnectionStringOptions {
            container_id_or_name: cluster_id_or_name.to_string(),
            db_username: None,
            db_password: None,
        };
        let connection_string = self
            .get_connection_string(get_connection_string_options)
            .await?;

        self.mongodb_client
            .get_deployment_id(&connection_string)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mongodb::MongoDbClient, test_utils::create_container_inspect_response_with_auth};
    use bollard::{
        errors::Error as BollardError, query_parameters::InspectContainerOptions,
        secret::ContainerInspectResponse,
    };
    use mockall::{mock, predicate::eq};

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
            async fn get_deployment_id(&self, connection_string: &str) -> Result<String, GetDeploymentIdError>;
        }
    }

    #[tokio::test]
    async fn test_get_deployment_id_mongo_connection_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoClientFactory::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        mock_mongo_client
            .expect_get_deployment_id()
            .with(eq("mongodb://127.0.0.1:27017/?directConnection=true"))
            .returning(|_| {
                Err(GetDeploymentIdError::MongoConnect(
                    mongodb::error::Error::from(mongodb::error::ErrorKind::Io(
                        std::io::Error::new(
                            std::io::ErrorKind::ConnectionRefused,
                            "Connection refused",
                        )
                        .into(),
                    )),
                ))
            });

        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));

        // Act
        let result = client.get_deployment_id("test-cluster").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentIdError::MongoConnect(_)
        ));
    }

    #[tokio::test]
    async fn test_get_deployment_id_atlascli_document_not_found() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoClientFactory::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        mock_mongo_client
            .expect_get_deployment_id()
            .with(eq("mongodb://127.0.0.1:27017/?directConnection=true"))
            .returning(|_| {
                Err(GetDeploymentIdError::NotFound(
                    "atlascli document".to_string(),
                ))
            });

        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));

        // Act
        let result = client.get_deployment_id("test-cluster").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentIdError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_get_deployment_id_uuid_not_found() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoClientFactory::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        mock_mongo_client
            .expect_get_deployment_id()
            .with(eq("mongodb://127.0.0.1:27017/?directConnection=true"))
            .returning(|_| Err(GetDeploymentIdError::NotFound("uuid".to_string())));

        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));

        // Act
        let result = client.get_deployment_id("test-cluster").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentIdError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_get_deployment_id_success() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoClientFactory::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        mock_mongo_client
            .expect_get_deployment_id()
            .with(eq("mongodb://127.0.0.1:27017/?directConnection=true"))
            .returning(|_| Ok("test-uuid".to_string()));

        let client = Client::with_mongo_client_factory(mock_docker, Box::new(mock_mongo_client));

        // Act
        let result = client.get_deployment_id("test-cluster").await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-uuid");
    }
}
