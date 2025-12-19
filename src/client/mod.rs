use bollard::Docker;

mod create_deployment;
mod delete_deployment;
mod get_connection_string;
mod get_deployment;
mod get_deployment_id;
mod get_logs;
mod get_mongodb_secret;
mod list_deployments;
mod pause_deployment;
mod pull_image;
mod start_deployment;
mod stop_deployment;
mod unpause_deployment;

pub use create_deployment::CreateDeploymentError;
pub use delete_deployment::DeleteDeploymentError;
pub use get_connection_string::GetConnectionStringError;
pub use get_deployment::GetDeploymentError;
pub use get_deployment_id::GetDeploymentIdError;
pub use get_logs::GetLogsError;
pub use pause_deployment::PauseDeploymentError;
pub use pull_image::PullImageError;
pub use start_deployment::StartDeploymentError;
pub use stop_deployment::StopDeploymentError;
pub use unpause_deployment::UnpauseDeploymentError;

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
        Client { docker }
    }
}
