use bollard::Docker;

mod create_deployment;
mod delete_deployment;
mod get_connection_string;
mod get_deployment;
mod list_deployments;
mod pull_image;

pub use create_deployment::CreateDeploymentError;
pub use delete_deployment::DeleteDeploymentError;
pub use get_connection_string::GetConnectionStringError;
pub use get_deployment::GetDeploymentError;
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
}

impl<D> Client<D> {
    /// Creates a new Atlas Local client.
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
        Client { docker }
    }
}
