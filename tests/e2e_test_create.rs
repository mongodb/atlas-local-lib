use atlas_local::{Client, CreateDeploymentError, models::CreateDeploymentOptions};
use bollard::Docker;
use futures_util::future::join_all;

mod e2e_test_utils;
use crate::e2e_test_utils::{DOCKER_TEST_MUTEX, TestContainerCleaner};

#[tokio::test(flavor = "multi_thread")]
async fn test_create_list_then_delete_deployment() {
    // Acquire the global lock to ensure isolation for docker tests
    // If another test panics, the lock may be poisoned but we still want to run the tests
    let _guard = DOCKER_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut container_cleaner = TestContainerCleaner::new();

    let docker = Docker::connect_with_socket_defaults().unwrap();
    let client = Client::new(docker);

    // Create a deployment
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

    // Create a list of names to track created containers
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
async fn test_create_deployments_same_name() {
    let _guard = DOCKER_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut container_cleaner = TestContainerCleaner::new();

    let docker = Docker::connect_with_socket_defaults().unwrap();
    let client = Client::new(docker);

    // Create a deployment
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

    // Create the same deployment again
    let result = client.create_deployment(&deployment1).await;

    // Check for ContainerAlreadyExists error
    match result {
        Err(CreateDeploymentError::ContainerAlreadyExists(_)) => {
            // If we reach this block, the assertion has passed.
        }
        _ => panic!(
            "The create result did not return the expected CreateDeploymentError::ContainerAlreadyExists(_)."
        ),
    }

    // Check only 1 deployment was created
    let deployments = client
        .list_deployments()
        .await
        .expect("Listing deployments");
    assert_eq!(deployments.len(), 1);
}
