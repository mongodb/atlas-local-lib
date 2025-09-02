#![doc = include_str!("../README.md")]

const ATLAS_LOCAL_IMAGE: &str = "mongodb/mongodb-atlas-local";
const ATLAS_LOCAL_TAG: &str = "latest";

use bollard::{
    Docker,
    query_parameters::{
        CreateContainerOptionsBuilder, CreateImageOptionsBuilder, InspectContainerOptions,
        ListContainersOptionsBuilder, RemoveContainerOptions, StartContainerOptions,
        StopContainerOptions,
    },
    secret::ContainerCreateBody,
};
use futures_util::StreamExt;
use maplit::hashmap;

use crate::models::{
    Deployment, IntoDeploymentError, LOCAL_DEPLOYMENT_LABEL_KEY, LOCAL_DEPLOYMENT_LABEL_VALUE,
};

pub mod models;

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
pub struct Client {
    #[allow(dead_code)] // TODO: remove this once we have methods on the client struct
    docker: Docker,
}

impl Client {
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
    pub fn new(docker: Docker) -> Client {
        Client { docker }
    }

    ///Creates a local Atlas deployment.
    pub async fn create_deployment(&self, cluster_name: &str) -> Result<(), CreateDeploymentError> {
        // Check if a container with that name already exists
        if self.check_container_exists(cluster_name).await? {
            return Err(CreateDeploymentError::ContainerAlreadyExists);
        }

        // Pull the latest image for Atlas Local
        self.pull_image().await?;

        // Create the container with the correct configuration
        let create_container_options = Some(
            CreateContainerOptionsBuilder::default()
                .name(cluster_name)
                .build(),
        );

        let create_container_config = ContainerCreateBody {
            image: Some(ATLAS_LOCAL_IMAGE.to_string()),
            labels: Some(
                hashmap! { LOCAL_DEPLOYMENT_LABEL_KEY.to_string() => LOCAL_DEPLOYMENT_LABEL_VALUE.to_string() },
            ),
            exposed_ports: Some(hashmap! {
                "27017/tcp".to_string() => hashmap! {},
            }),
            ..Default::default()
        };

        let _create_response = self
            .docker
            .create_container(create_container_options, create_container_config)
            .await?;

        // Start the Atlas Local container
        self.docker
            .start_container(cluster_name, None::<StartContainerOptions>)
            .await?;

        Ok(())
    }

    /// Deletes a local Atlas deployment.
    pub async fn delete_deployment(&self, name: &str) -> Result<(), DeleteDeploymentError> {
        // Check that a deployment with that name exists and get the container ID.
        // This ensures we only try to delete valid Atlas local deployments.
        let deployment = self
            .get_deployment(name)
            .await
            .map_err(DeleteDeploymentError::GetDeployment)?;
        let container_id = deployment.container_id.as_str();

        // Attempt to stop the container gracefully before removal.
        self.docker
            .stop_container(container_id, None::<StopContainerOptions>)
            .await?;

        // Remove the container from Docker.
        self.docker
            .remove_container(container_id, None::<RemoveContainerOptions>)
            .await?;

        Ok(())
    }

    /// Lists all local Atlas deployments.
    pub async fn list_deployments(&self) -> Result<Vec<Deployment>, GetDeploymentError> {
        // Build the list containers options which will filter for containers with the local deployment label
        let list_container_options = ListContainersOptionsBuilder::default()
            .all(true)
            .filters(&hashmap! {
                "label" => vec![format!("{}={}", LOCAL_DEPLOYMENT_LABEL_KEY, LOCAL_DEPLOYMENT_LABEL_VALUE)],
            })
            .build();

        // Get all the containers using the list containers options
        let container_summaries = self
            .docker
            .list_containers(Some(list_container_options))
            .await?;

        // Create the output vector used to return the deployments
        let mut deployments = Vec::with_capacity(container_summaries.len());

        // Iterate over the container summaries and get the deployment details for each container
        for container_summary in container_summaries {
            // Get the container ID from the container summary
            // This should always be present, but it's cleaner to not use unwrap and skip if it's not present
            if let Some(container_id) = container_summary.id {
                // Get the deployment details for the container
                let deployment = self.get_deployment(container_id.as_ref()).await?;
                deployments.push(deployment);
            }
        }

        Ok(deployments)
    }

    pub async fn get_deployment(
        &self,
        container_id_or_name: &str,
    ) -> Result<Deployment, GetDeploymentError> {
        // Inspect the container to get the deployment details
        let container_inspect_response = self
            .docker
            .inspect_container(container_id_or_name, None::<InspectContainerOptions>)
            .await?;

        // Convert the container inspect response to a deployment
        Ok(container_inspect_response.try_into()?)
    }

    pub async fn pull_image(&self) -> Result<(), CreateDeploymentError> {
        // Build the options for pulling the Atlas Local Docker image
        let create_image_options = CreateImageOptionsBuilder::default()
            .from_image(ATLAS_LOCAL_IMAGE)
            .tag(ATLAS_LOCAL_TAG)
            .build();

        // Start pulling the image, which returns a stream of progress events
        let mut stream = self
            .docker
            .create_image(Some(create_image_options), None, None);

        // Iterate over the stream and check for errors
        while let Some(result) = stream.next().await {
            let image_info = result.map_err(CreateDeploymentError::PullImage)?;

            // Optionally print the status of the image pull in debug mode
            if let Some(status) = image_info.status {
                #[cfg(debug_assertions)]
                println!("{}", status);
            }
        }

        Ok(())
    }

    pub async fn check_container_exists(&self, name: &str) -> Result<bool, bollard::errors::Error> {
        // Build the options for listing containers with a filter on the name
        let list_container_options = ListContainersOptionsBuilder::default()
            .all(true)
            .filters(&hashmap! {
                "name" => vec![name.to_string()],
            })
            .build();

        // List the containers with the specified name filter
        let containers = self
            .docker
            .list_containers(Some(list_container_options))
            .await?;
        // Return true if any containers were found with that name
        Ok(!containers.is_empty())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetDeploymentError {
    #[error("Failed to inspect container: {0}")]
    ContainerInspect(#[from] bollard::errors::Error),
    #[error("The container is not a local Atlas deployment: {0}")]
    IntoDeployment(#[from] IntoDeploymentError),
}
#[derive(Debug, thiserror::Error)]
pub enum DeleteDeploymentError {
    #[error("Failed to delete container: {0}")]
    ContainerInspect(#[from] bollard::errors::Error),
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
}
#[derive(Debug, thiserror::Error)]
pub enum CreateDeploymentError {
    #[error("Failed to create container: {0}")]
    CreateContainer(#[from] bollard::errors::Error),
    #[error("Failed to pull image: {0}")]
    PullImage(bollard::errors::Error),
    #[error("Container with that name already exists")]
    ContainerAlreadyExists,
}
