#![cfg(feature = "e2e-tests")]
use atlas_local::{
    Client,
    models::{CreateDeploymentOptions, GetConnectionStringOptions, MongoDBPortBinding},
};
use bollard::{Docker, query_parameters::RemoveContainerOptionsBuilder};
use tokio::runtime::Handle;

pub struct TestContainerCleaner {
    container_names: Vec<String>,
}

#[allow(clippy::new_without_default)]
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

#[tokio::test(flavor = "multi_thread")]
async fn test_e2e_smoke_test() {
    let mut container_cleaner = TestContainerCleaner::new();

    let docker = Docker::connect_with_defaults().unwrap();
    let client = Client::new(docker.clone());

    // Count number of active deployments
    let start_deployment_count = client
        .list_deployments()
        .await
        .expect("Listing deployments")
        .len();

    // Create a deployment
    let name = "test_deployment_name";
    let username = "testuser";
    let password = "testpassword";
    container_cleaner.add_container(name);
    let deployment1 = CreateDeploymentOptions {
        name: Some(name.to_string()),
        mongodb_initdb_root_username: Some(username.to_string()),
        mongodb_initdb_root_password: Some(password.to_string()),
        ..Default::default()
    };
    client
        .create_deployment(&deployment1)
        .await
        .expect("Creating deployment");

    // Count deployments and verify a deployment was created
    let deployments = client
        .list_deployments()
        .await
        .expect("Listing deployments");
    assert_eq!(deployments.len() - start_deployment_count, 1);
    assert_eq!(deployments.first().unwrap().name.as_deref(), Some(name));

    // Get port of created deployment to verify connection string
    let port = match &deployments.first().unwrap().port_bindings {
        Some(MongoDBPortBinding { port, .. }) => *port,
        _ => panic!("No port binding found"),
    };

    // Get Connection String
    let get_conn_string_req = GetConnectionStringOptions {
        container_id_or_name: name,
        db_username: Some(username),
        db_password: Some(password),
        verify: Some(true),
    };

    let conn_string = client
        .get_connection_string(get_conn_string_req)
        .await
        .expect("Getting connection string");

    if std::path::Path::new("/.dockerenv").exists() {
        assert_eq!(
            conn_string,
            format!(
                "mongodb://{}:{}@docker-dind:{}/?directConnection=true",
                username,
                password,
                port.unwrap()
            )
        );
    } else {
        assert_eq!(
            conn_string,
            format!(
                "mongodb://{}:{}@127.0.0.1:{}/?directConnection=true",
                username,
                password,
                port.unwrap()
            )
        );
    }

    // Delete Deployment
    client
        .delete_deployment(name)
        .await
        .expect("Deleting deployment");

    // Count deployments and verify a deployment was deleted
    let end_deployment_count = client
        .list_deployments()
        .await
        .expect("Listing deployments")
        .len();
    assert_eq!(start_deployment_count, end_deployment_count);
}
