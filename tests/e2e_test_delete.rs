mod e2e_test_utils;
use atlas_local::{
    Client, DeleteDeploymentError, GetDeploymentError,
    models::{CreateDeploymentOptions, GetLocalDeploymentLabelsError, IntoDeploymentError},
};
use bollard::{Docker, query_parameters::ListContainersOptions};

use crate::e2e_test_utils::{
    DOCKER_TEST_MUTEX, TestContainerCleaner, create_persistent_unrelated_container,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_only_deletes_atlas_local() {
    // Acquire the global lock to ensure isolation for docker tests
    let _guard = DOCKER_TEST_MUTEX.lock().await;
    let mut container_cleaner = TestContainerCleaner::new();

    let docker = Docker::connect_with_socket_defaults().unwrap();
    let client = Client::new(docker);

    // Create a deployment
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

    // Create a dummy container
    let docker = Docker::connect_with_socket_defaults().unwrap();
    let dummy_container_name = "dummy-container";
    container_cleaner.add_container(dummy_container_name);
    create_persistent_unrelated_container(&docker, dummy_container_name)
        .await
        .expect("Creating dummy container");

    // We are using docker directly rather than client because we want to list all containers not just deployments
    let containers = docker
        .list_containers(None::<ListContainersOptions>)
        .await
        .expect("Listing containers");
    assert_eq!(containers.len(), 2);

    // Delete the deployment
    client
        .delete_deployment(deployment_name)
        .await
        .expect("Deleting deployment");
    // Attempt to delete the dummy container
    let delete_result = client.delete_deployment(dummy_container_name).await;

    // Check Error is of the correct type
    match delete_result {
        Err(DeleteDeploymentError::GetDeployment(GetDeploymentError::IntoDeployment(
            IntoDeploymentError::LocalDeploymentLabels(
                GetLocalDeploymentLabelsError::NotALocalDeployment,
            ),
        ))) => {
            // If we reach this block, the assertion has passed.
        }
        Err(e) => {
            panic!(
                "Deleting the dummy container did not return the expected error. Instead, it returned: {:?}",
                e
            );
        }
        Ok(_) => {
            panic!("Deletion of dummy container was successful when it should fail");
        }
    }

    // Check only the deployment was deleted and the alpine container was not
    let containers = docker
        .list_containers(None::<ListContainersOptions>)
        .await
        .expect("Listing containers");
    assert_eq!(containers.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_deletion_deployment_does_not_exist() {
    let _guard = DOCKER_TEST_MUTEX.lock().await;

    let docker = Docker::connect_with_socket_defaults().unwrap();
    let client = Client::new(docker);

    // Delete a deployment without creating it first
    let delete_result = client.delete_deployment("does-not-exist").await;

    // Check Error is of the correct type
    match delete_result {
        Err(DeleteDeploymentError::GetDeployment(GetDeploymentError::ContainerInspect(_))) => {
            // If we reach this block, the assertion has passed.
        }
        _ => {
            panic!(
                "The delete result did not return the expected DeleteDeploymentError::GetDeployment(GetDeploymentError::ContainerInspect(_))."
            );
        }
    }
}
