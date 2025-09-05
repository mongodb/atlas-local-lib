use atlas_local::{Client, models::CreateDeploymentOptions};
use bollard::Docker;
mod e2e_test_utils;

use crate::e2e_test_utils::{
    DOCKER_TEST_MUTEX, TestContainerCleaner, create_persistent_unrelated_container,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_list_deployment_empty() {
    // Acquire the global lock to ensure isolation for docker tests
    let _guard = DOCKER_TEST_MUTEX.lock().await;

    let docker = Docker::connect_with_socket_defaults().unwrap();
    let client = Client::new(docker);

    // List all active deployments
    let deployments = client
        .list_deployments()
        .await
        .expect("Listing Deployments");

    // Check the list is empty
    assert!(deployments.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_only_deployments() {
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

    // List all active deployments
    let deployments = client
        .list_deployments()
        .await
        .expect("Listing Deployments");

    // Check there is only 1 deployment and it is correct
    assert_eq!(deployments.len(), 1);
    assert_eq!(
        deployments.first().unwrap().name,
        Some(deployment_name.to_string())
    );
}
