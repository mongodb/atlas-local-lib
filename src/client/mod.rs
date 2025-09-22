use crate::mongodb::{MongoDbAdapter, MongoDbClient};
use bollard::Docker;

mod create_deployment;
mod delete_deployment;
mod get_connection_string;
mod get_deployment;
mod get_deployment_id;
mod list_deployments;
mod pull_image;

pub use create_deployment::CreateDeploymentError;
pub use delete_deployment::DeleteDeploymentError;
pub use get_connection_string::GetConnectionStringError;
pub use get_deployment::GetDeploymentError;
pub use get_deployment_id::GetDeploymentIdError;
pub use pull_image::PullImageError;

/// The main entry point for interacting with local Atlas deployments.
///
/// `Client` provides a high-level interface for managing MongoDB Atlas local deployments
/// through Docker. It serves as the primary abstraction layer between your application
/// and the underlying Docker containers running Atlas services.
///
/// # Examples
///
/// See the [module-level documentation](crate) for a complete example of creating
/// a new client instance.
pub struct Client<D = Docker> {
    docker: D,
    mongo_client_factory: Box<dyn MongoDbClient + Send + Sync>,
}

impl<D> Client<D> {
    /// Creates a new Atlas Local client with the default MongoDB adapter.
    ///
    /// # Arguments
    ///
    /// * `docker` - A connected Docker client instance from the `bollard` crate
    ///
    /// # Returns
    ///
    /// A new `Client` instance ready to manage Atlas Local deployments.
    ///
    /// # Examples
    ///
    /// See the [module-level documentation](crate) for usage examples.    
    pub fn new(docker: D) -> Client<D> {
        Client {
            docker,
            mongo_client_factory: Box::new(MongoDbAdapter {}),
        }
    }

    /// Creates a new Atlas Local client with a custom MongoDB client factory.
    ///
    /// This constructor is primarily useful for testing scenarios where you need
    /// to inject mock implementations of the MongoDB client.
    ///
    /// # Arguments
    ///
    /// * `docker` - A Docker client implementation
    /// * `mongo_client_factory` - A MongoDB client factory implementation
    ///
    /// # Returns
    ///
    /// A new `Client` instance with the specified implementations.
    pub fn with_mongo_client_factory(
        docker: D,
        mongo_client_factory: Box<dyn MongoDbClient>,
    ) -> Client<D> {
        Client {
            docker,
            mongo_client_factory,
        }
    }
}
