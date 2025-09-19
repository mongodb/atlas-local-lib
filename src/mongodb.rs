use mongodb::{Client, Collection, bson::Document, error::Error};
use std::future::Future;

pub trait MongoDbClient {
    fn with_uri_str(
        &self,
        uri: &str,
    ) -> impl Future<Output = Result<impl MongoDbConnection, Error>>;
    fn list_database_names(
        &self,
        connection_string: &str,
    ) -> impl Future<Output = Result<Vec<String>, Error>>;
}

pub trait MongoDbConnection {
    fn database(&self, name: &str) -> impl MongoDbDatabase;
}

pub trait MongoDbDatabase {
    fn collection(&self, name: &str) -> impl MongoDbCollection;
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
    async fn with_uri_str(&self, uri: &str) -> Result<impl MongoDbConnection, Error> {
        let client = Client::with_uri_str(uri).await?;
        Ok(MongoClientWrapper { client })
    }

    async fn list_database_names(&self, connection_string: &str) -> Result<Vec<String>, Error> {
        let client_options = mongodb::options::ClientOptions::parse(connection_string).await?;
        let mongo_client = Client::with_options(client_options)?;
        mongo_client.list_database_names().await
    }
}

pub struct MongoClientWrapper {
    client: Client,
}

impl MongoDbConnection for MongoClientWrapper {
    fn database(&self, name: &str) -> impl MongoDbDatabase {
        MongoDatabaseWrapper {
            database: self.client.database(name),
        }
    }
}

pub struct MongoDatabaseWrapper {
    database: mongodb::Database,
}

impl MongoDbDatabase for MongoDatabaseWrapper {
    fn collection(&self, name: &str) -> impl MongoDbCollection {
        let collection = self.database.collection(name);
        MongoCollection { collection }
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
