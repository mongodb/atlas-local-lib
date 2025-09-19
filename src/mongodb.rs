use mongodb::{Client, Collection, bson::Document, error::Error};
use std::future::Future;

pub trait MongoDbClient {
    fn with_uri_str(&self, uri: &str) -> impl Future<Output = Result<impl MongoDbDatabase, Error>>;
    fn list_database_names(
        &self,
        connection_string: &str,
    ) -> impl Future<Output = Result<Vec<String>, Error>>;
}

pub trait MongoDbDatabase {
    fn collection(&self, name: &str) -> impl MongoDbCollection;
    fn database(&self, name: &str) -> impl MongoDbDatabase;
}

pub trait MongoDbCollection {
    fn find_one(
        &self,
        filter: Document,
    ) -> impl Future<Output = Result<Option<Document>, Error>> + Send;
}

// Real implementations using MongoDB client
pub struct MongoDbAdapter;

impl MongoDbClient for MongoDbAdapter {
    async fn with_uri_str(&self, uri: &str) -> Result<impl MongoDbDatabase, Error> {
        let client = Client::with_uri_str(uri).await?;
        Ok(MongoDatabase {
            client,
            db_name: "admin".to_string(),
        })
    }

    async fn list_database_names(&self, connection_string: &str) -> Result<Vec<String>, Error> {
        let client_options = mongodb::options::ClientOptions::parse(connection_string).await?;
        let mongo_client = Client::with_options(client_options)?;
        mongo_client.list_database_names().await
    }
}

pub struct MongoDatabase {
    client: Client,
    db_name: String,
}

impl MongoDbDatabase for MongoDatabase {
    fn collection(&self, name: &str) -> impl MongoDbCollection {
        let collection = self.client.database(&self.db_name).collection(name);
        MongoCollection { collection }
    }

    fn database(&self, name: &str) -> impl MongoDbDatabase {
        MongoDatabase {
            client: self.client.clone(),
            db_name: name.to_string(),
        }
    }
}

pub struct MongoCollection {
    collection: Collection<Document>,
}

impl MongoDbCollection for MongoCollection {
    async fn find_one(&self, filter: Document) -> Result<Option<Document>, Error> {
        self.collection.find_one(filter).await
    }
}
