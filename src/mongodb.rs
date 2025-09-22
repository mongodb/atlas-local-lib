use crate::client::GetDeploymentIdError;
use async_trait::async_trait;
use mongodb::{Client, bson::Document, error::Error};

#[async_trait]
pub trait ListDatabases {
    async fn list_database_names(&self, connection_string: &str) -> Result<Vec<String>, Error>;
}

#[async_trait]
pub trait GetDeploymentId {
    async fn get_deployment_id(
        &self,
        connection_string: &str,
    ) -> Result<String, GetDeploymentIdError>;
}

pub trait MongoClientFactory: GetDeploymentId + ListDatabases {}

// Real implementations using MongoDB client
pub struct MongoDbAdapter;

#[async_trait]
impl ListDatabases for MongoDbAdapter {
    async fn list_database_names(&self, connection_string: &str) -> Result<Vec<String>, Error> {
        let client_options = mongodb::options::ClientOptions::parse(connection_string).await?;
        let mongo_client = Client::with_options(client_options)?;
        mongo_client.list_database_names().await
    }
}

#[async_trait]
impl GetDeploymentId for MongoDbAdapter {
    async fn get_deployment_id(
        &self,
        connection_string: &str,
    ) -> Result<String, GetDeploymentIdError> {
        let client = Client::with_uri_str(connection_string).await?;
        let admin_db = client.database("admin");
        let collection = admin_db.collection("atlascli");

        let atlas_doc: Document =
            collection
                .find_one(Document::new())
                .await?
                .ok_or(GetDeploymentIdError::NotFound(
                    "atlascli document".to_string(),
                ))?;

        atlas_doc
            .get_str("uuid")
            .map(|s| s.to_string())
            .map_err(|_| GetDeploymentIdError::NotFound("uuid".to_string()))
    }
}

impl MongoClientFactory for MongoDbAdapter {}
