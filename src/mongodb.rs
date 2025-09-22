use mongodb::{Client, Collection, bson::Document, error::Error};
use std::future::Future;
use std::pin::Pin;

type ConnectionResult =
    Pin<Box<dyn Future<Output = Result<Box<dyn MongoDbConnection>, Error>> + Send>>;

pub trait MongoDbClient: Send + Sync {
    fn with_uri_str(&self, uri: &str) -> ConnectionResult;
    fn list_database_names(
        &self,
        connection_string: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, Error>> + Send>>;
}

pub trait MongoDbConnection: Send + Sync {
    fn database(&self, name: &str) -> Box<dyn MongoDbDatabase>;
}

pub trait MongoDbDatabase: Send + Sync {
    fn collection(&self, name: &str) -> Box<dyn MongoDbCollection>;
}

pub trait MongoDbCollection: Send + Sync {
    fn find_one(
        &self,
        filter: Document,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Document>, Error>> + Send>>;
}

// Real implementations using MongoDB client
pub struct MongoDbAdapter;

impl MongoDbClient for MongoDbAdapter {
    fn with_uri_str(&self, uri: &str) -> ConnectionResult {
        let uri = uri.to_string();
        Box::pin(async move {
            let client = Client::with_uri_str(&uri).await?;
            Ok(Box::new(MongoDbClientWrapper { client }) as Box<dyn MongoDbConnection>)
        })
    }

    fn list_database_names(
        &self,
        connection_string: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, Error>> + Send>> {
        let connection_string = connection_string.to_string();
        Box::pin(async move {
            let client_options = mongodb::options::ClientOptions::parse(&connection_string).await?;
            let mongo_client = Client::with_options(client_options)?;
            mongo_client.list_database_names().await
        })
    }
}

pub struct MongoDbClientWrapper {
    client: Client,
}

impl MongoDbConnection for MongoDbClientWrapper {
    fn database(&self, name: &str) -> Box<dyn MongoDbDatabase> {
        Box::new(MongoDbDatabaseWrapper {
            database: self.client.database(name),
        })
    }
}

pub struct MongoDbDatabaseWrapper {
    database: mongodb::Database,
}

impl MongoDbDatabase for MongoDbDatabaseWrapper {
    fn collection(&self, name: &str) -> Box<dyn MongoDbCollection> {
        let collection = self.database.collection(name);
        Box::new(MongoDbCollectionWrapper { collection })
    }
}

pub struct MongoDbCollectionWrapper {
    collection: Collection<Document>,
}

impl MongoDbCollection for MongoDbCollectionWrapper {
    fn find_one(
        &self,
        filter: Document,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Document>, Error>> + Send>> {
        let collection = self.collection.clone();
        Box::pin(async move { collection.find_one(filter).await })
    }
}
