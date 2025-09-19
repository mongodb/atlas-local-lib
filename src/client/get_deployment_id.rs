use mongodb::{bson::Document, error::Error};

use crate::{
    docker::DockerInspectContainer, models::GetConnectionStringOptions, mongodb::{MongoDbAdapter, MongoDbClient, MongoDbCollection, MongoDbConnection, MongoDbDatabase}, Client
};

#[derive(Debug, thiserror::Error)]
pub enum GetDeploymentIdError {
    #[error("Failed to get connection string: {0}")]
    GetConnectionString(#[from] crate::client::get_connection_string::GetConnectionStringError),
    #[error("Failed to connect to MongoDB: {0}")]
    MongoConnect(#[from] Error),
    #[error("No atlascli document found")]
    NoAtlasCliDoc,
    #[error("No UUID found in atlascli document")]
    NoUUID,
}

impl<D: DockerInspectContainer> Client<D> {
    /// Gets the Atlas deployment ID for a local Atlas deployment.
    pub async fn get_deployment_id(
        &self,
        cluster_id_or_name: &str,
    ) -> Result<String, GetDeploymentIdError> {
        self.get_deployment_id_with_client(cluster_id_or_name, &MongoDbAdapter)
            .await
    }

    async fn get_deployment_id_with_client<M: MongoDbClient>(
        &self,
        cluster_id_or_name: &str,
        mongo_client: &M,
    ) -> Result<String, GetDeploymentIdError> {
        let get_connection_string_options = GetConnectionStringOptions {
            container_id_or_name: cluster_id_or_name.to_string(),
            db_username: None,
            db_password: None,
            verify: Some(false),
        };
        let connection_string = self
            .get_connection_string(get_connection_string_options)
            .await?;

        let client = mongo_client.with_uri_str(&connection_string).await?;
        let admin_db = client.database("admin");
        let collection = admin_db.collection("atlascli");

        let atlas_doc = collection
            .find_one(Document::new())
            .await?
            .ok_or(GetDeploymentIdError::NoAtlasCliDoc)?;

        atlas_doc
            .get_str("uuid")
            .map(|s| s.to_string())
            .map_err(|_| GetDeploymentIdError::NoUUID)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mongodb::MongoDbConnection;
    use crate::test_utils::create_container_inspect_response_with_auth;
    use bollard::{
        errors::Error as BollardError, query_parameters::InspectContainerOptions,
        secret::ContainerInspectResponse,
    };
    use mockall::{mock, predicate::eq};
    use mongodb::bson::Document;
    use mongodb::error::{Error, ErrorKind};

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
        MongoAdapter {}

        #[allow(refining_impl_trait)]
        impl MongoDbClient for MongoAdapter {
            async fn with_uri_str(&self, uri: &str) -> Result<MockMongoConnection, Error>;
            async fn list_database_names(&self, connection_string: &str) -> Result<Vec<String>, Error>;
        }
    }

    mock! {
        MongoConnection {}

        #[allow(refining_impl_trait)]
        impl MongoDbConnection for MongoConnection {
            fn database(&self, name: &str) -> MockMongoDatabase;
        }
    }

    mock! {
        MongoDatabase {}

        #[allow(refining_impl_trait)]
        impl MongoDbDatabase for MongoDatabase {
            fn collection(&self, name: &str) -> MockMongoCollection;
        }
    }

    mock! {
        MongoCollection {}

        impl MongoDbCollection for MongoCollection {
            async fn find_one(&self, filter: Document) -> Result<Option<Document>, Error>;
        }
    }

    fn create_mock_collection_with_doc(doc: Option<Document>) -> MockMongoCollection {
        let mut mock_collection = MockMongoCollection::new();
        mock_collection
            .expect_find_one()
            .with(eq(Document::new()))
            .returning(move |_| Ok(doc.clone()));
        mock_collection
    }

    fn create_mock_admin_db(doc: Option<Document>) -> MockMongoDatabase {
        let mut admin_db = MockMongoDatabase::new();
        admin_db
            .expect_collection()
            .with(eq("atlascli"))
            .returning(move |_| create_mock_collection_with_doc(doc.clone()));
        admin_db
    }

    fn create_mock_connection_with_admin_db(doc: Option<Document>) -> MockMongoConnection {
        let mut mock_connection = MockMongoConnection::new();
        mock_connection
            .expect_database()
            .with(eq("admin"))
            .returning(move |_| create_mock_admin_db(doc.clone()));
        mock_connection
    }

    fn create_mongo_client_mock(mock_mongo_client: &mut MockMongoAdapter, doc: Option<Document>) {
        mock_mongo_client
            .expect_with_uri_str()
            .with(eq("mongodb://127.0.0.1:27017/?directConnection=true"))
            .returning(move |_| Ok(create_mock_connection_with_admin_db(doc.clone())));
    }

    #[tokio::test]
    async fn test_get_deployment_id_mongo_connection_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoAdapter::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        mock_mongo_client
            .expect_with_uri_str()
            .with(eq("mongodb://127.0.0.1:27017/?directConnection=true"))
            .returning(|_| {
                Err(Error::from(ErrorKind::Io(
                    std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        "Connection refused",
                    )
                    .into(),
                )))
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .get_deployment_id_with_client("test-cluster", &mock_mongo_client)
            .await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentIdError::MongoConnect(_)
        ));
    }

    #[tokio::test]
    async fn test_get_deployment_id_no_atlascli_doc() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoAdapter::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        // Mock successful connection to MongoDB, but no atlascli doc
        create_mongo_client_mock(&mut mock_mongo_client, None);

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .get_deployment_id_with_client("test-cluster", &mock_mongo_client)
            .await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentIdError::NoAtlasCliDoc
        ));
    }

    #[tokio::test]
    async fn test_get_deployment_id_no_uuid() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoAdapter::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        // Mock successful connection to MongoDB, but no UUID in atlascli doc
        create_mongo_client_mock(&mut mock_mongo_client, Some(Document::new()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .get_deployment_id_with_client("test-cluster", &mock_mongo_client)
            .await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GetDeploymentIdError::NoUUID));
    }

    #[tokio::test]
    async fn test_get_deployment_id_success() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut mock_mongo_client = MockMongoAdapter::new();

        // Mock successful connection string retrieval
        mock_docker
            .expect_inspect_container()
            .returning(|_, _| Ok(create_container_inspect_response_with_auth(27017)));

        // Mock successful connection to MongoDB, and successful retrieval of UUID
        let mut doc = Document::new();
        doc.insert("uuid", "test-uuid");
        create_mongo_client_mock(&mut mock_mongo_client, Some(doc));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .get_deployment_id_with_client("test-cluster", &mock_mongo_client)
            .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-uuid");
    }
}
