use crate::client::GetDeploymentIdError;
use async_trait::async_trait;
use mongodb::{Client, bson::Document};

#[async_trait]
pub trait MongoDbClient: Send + Sync {
    async fn get_deployment_id(
        &self,
        connection_string: &str,
    ) -> Result<String, GetDeploymentIdError>;
}
// Real implementations using MongoDB client
pub struct MongoDbAdapter;

#[async_trait]
impl MongoDbClient for MongoDbAdapter {
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
