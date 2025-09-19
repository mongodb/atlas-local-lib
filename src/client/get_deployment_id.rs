use crate::{docker::DockerInspectContainer, models::GetConnectionStringOptions};
use mongodb::{bson::Document, error::Error, Collection};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum GetDeploymentIdError {
    #[error("Failed to get connection string: {0}")]
    GetConnectionString(#[from] crate::client::get_connection_string::GetConnectionStringError),
    #[error("Failed to connect to MongoDB: {0}")]
    MongoConnect(#[from] Error),
    #[error("No atlascli document found")]
    NoAtlasCliDoc,
}

impl<D: DockerInspectContainer> super::Client<D> {
    /// Gets the Atlas deployment ID for a local Atlas deployment.
    pub async fn get_deployment_id(&self, cluster_id_or_name: &str) -> Result<String, GetDeploymentIdError> {
        let get_connection_string_options = GetConnectionStringOptions {
            container_id_or_name: cluster_id_or_name.to_string(),
            db_username: None,
            db_password: None,
            verify: Some(false),
        };
        let connection_string = self.get_connection_string(get_connection_string_options).await?;
        println!("Connection String: {}", connection_string);
        let client = mongodb::Client::with_uri_str(connection_string).await?;
        let db = client.database("admin");
        let collection: Collection<Document> = db.collection("atlascli");
        let doc = collection.find_one(Document::new()).await?;
        match doc {
            Some(atlas_doc) => {
                if let Ok(uuid) = atlas_doc.get_str("uuid") {
                    Ok(uuid.to_string())
                } else {
                    Err(GetDeploymentIdError::NoAtlasCliDoc)
                }
            },
            None => Err(GetDeploymentIdError::NoAtlasCliDoc),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_deployment_id() {
        todo!();
    }
}
