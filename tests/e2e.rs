use atlas_local::{Client, models::CreateDeploymentOptions};
use bollard::{
    Docker,
    query_parameters::{
        CreateContainerOptionsBuilder, RemoveContainerOptionsBuilder, StartContainerOptions,
    },
    secret::ContainerCreateBody,
};
use futures_util::future::join_all;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::runtime::Handle;
// Mutex that ensures e2e tests that create deployments are completed in isolation
// Important for avoiding naming conflicts and for counting how many docker containers were created
static DOCKER_TEST_MUTEX: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

pub struct TestContainerCleaner {
    container_names: Vec<String>,
}

impl TestContainerCleaner {
    pub fn new() -> Self {
        let container_names = Vec::new();
        Self { container_names }
    }

    pub fn add_container(&mut self, name: &str) {
        self.container_names.push(name.to_string());
    }
}

// Runs when TestContainerCleaner goes out of scope at either end of test or panic
impl Drop for TestContainerCleaner {
    fn drop(&mut self) {
        let docker = Docker::connect_with_defaults().unwrap();
        let runtime_handle = Handle::current();

        // Blocks current thread to ensure no new tests are started until these containers are cleaned up
        tokio::task::block_in_place(move || {
            runtime_handle.block_on(async {
                // Removes all containers created during this test
                for container_name in &self.container_names {
                    let _ = docker
                        .remove_container(
                            container_name,
                            Some(RemoveContainerOptionsBuilder::default().force(true).build()),
                        )
                        .await;
                }
            })
        });
    }
}

async fn create_persistent_unrelated_container(
    docker: &Docker,
    name: &str,
) -> Result<(), bollard::errors::Error> {
    let options = Some(CreateContainerOptionsBuilder::default().name(name).build());
    let config = ContainerCreateBody {
        image: Some("alpine:3.18.10".to_string()),
        cmd: Some(vec!["sleep".to_string(), "3600".to_string()]),
        ..Default::default()
    };
    docker.create_container(options, config).await?;

    docker
        .start_container(name, None::<StartContainerOptions>)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use atlas_local::{DeleteDeploymentError, GetDeploymentError};
    use bollard::{Docker, query_parameters::ListContainersOptions};

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_list_then_delete_container() {
        // Acquire the global lock to ensure isolation for docker tests
        // If another test panics, the lock may be poisoned but we still want to run the tests
        let _guard = DOCKER_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut container_cleaner = TestContainerCleaner::new();

        let docker = Docker::connect_with_socket_defaults().unwrap();
        let client = Client::new(docker);

        let name = "test_deployment_name";

        container_cleaner.add_container(name);
        let deployment1 = CreateDeploymentOptions {
            name: Some(name.to_string()),
            ..Default::default()
        };
        client
            .create_deployment(&deployment1)
            .await
            .expect("Creating deployment");

        // List deployments and verify a deployment was created
        let deployments = client
            .list_deployments()
            .await
            .expect("Listing deployments");
        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments.first().unwrap().name.as_deref(), Some(name));

        // Delete Deployment
        client
            .delete_deployment(name)
            .await
            .expect("Deleting deployment");

        // List deployments and verify a deployment was deleted
        let deployments = client
            .list_deployments()
            .await
            .expect("Listing deployments");
        assert_eq!(deployments.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_five_deployments_concurrent() {
        let _guard = DOCKER_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut container_cleaner = TestContainerCleaner::new();

        let docker = Docker::connect_with_socket_defaults().unwrap();
        let client = Client::new(docker);

        let names = [
            "container1",
            "container2",
            "container3",
            "container4",
            "container5",
        ];
        // Ensure all created containers are cleaned up
        for name in names {
            container_cleaner.add_container(name);
        }

        let deployment1 = CreateDeploymentOptions {
            name: Some(names[0].to_string()),
            ..Default::default()
        };
        let deployment2 = CreateDeploymentOptions {
            name: Some(names[1].to_string()),
            ..Default::default()
        };
        let deployment3 = CreateDeploymentOptions {
            name: Some(names[2].to_string()),
            ..Default::default()
        };
        let deployment4 = CreateDeploymentOptions {
            name: Some(names[3].to_string()),
            ..Default::default()
        };
        let deployment5 = CreateDeploymentOptions {
            name: Some(names[4].to_string()),
            ..Default::default()
        };

        let futures = [
            client.create_deployment(&deployment1),
            client.create_deployment(&deployment2),
            client.create_deployment(&deployment3),
            client.create_deployment(&deployment4),
            client.create_deployment(&deployment5),
        ];
        // Create deployments concurrently
        join_all(futures).await;

        // Check all 5 deployments were created successfully
        let deployments = client
            .list_deployments()
            .await
            .expect("Listing deployments");
        assert_eq!(deployments.len(), 5);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_only_deletes_atlas_local() {
        let _guard = DOCKER_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut container_cleaner = TestContainerCleaner::new();

        let docker = Docker::connect_with_socket_defaults().unwrap();
        let client = Client::new(docker);

        let deployment_name = "test_deployment_name";

        container_cleaner.add_container(deployment_name);
        let deployment1 = CreateDeploymentOptions {
            name: Some(deployment_name.to_string()),
            ..Default::default()
        };
        client
            .create_deployment(&deployment1)
            .await
            .expect("Creating deployment");

        let dummy_container_name = "dummy-container";
        container_cleaner.add_container(dummy_container_name);
        let docker = Docker::connect_with_socket_defaults().unwrap();
        client
            .pull_image("alpine", "3.18.10")
            .await
            .expect("Pulling Alpine image");
        create_persistent_unrelated_container(&docker, dummy_container_name)
            .await
            .expect("Creating dummy container");

        // We are using docker directly rather than client because we want to list all containers not just deployments
        let containers = docker
            .list_containers(None::<ListContainersOptions>)
            .await
            .expect("Listing containers");
        assert_eq!(containers.len(), 2);

        client
            .delete_deployment(deployment_name)
            .await
            .expect("Deleting deployment");
        let delete_result: Result<(), atlas_local::DeleteDeploymentError> =
            client.delete_deployment(deployment_name).await;

        // Check Error is of the correct type
        match delete_result {
            Err(DeleteDeploymentError::GetDeployment(GetDeploymentError::ContainerInspect(_))) => {
                // If we reach this block, the assertion has passed.
            }
            _ => {
                // If the error doesn't match the expected pattern, panic.
                panic!(
                    "The delete result did not return the expected DeleteDeploymentError::GetDeployment(GetDeploymentError::ContainerInspect(_))."
                );
            }
        }

        // Check only the deployment was deleted and the alpine container was not
        let containers = docker
            .list_containers(None::<ListContainersOptions>)
            .await
            .expect("Listing containers");
        assert_eq!(containers.len(), 1);
    }
}
