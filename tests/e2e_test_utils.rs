use bollard::{
    Docker,
    query_parameters::{
        CreateContainerOptionsBuilder, CreateImageOptionsBuilder, RemoveContainerOptionsBuilder,
        StartContainerOptions,
    },
    secret::ContainerCreateBody,
};
use futures_util::StreamExt;
use std::sync::LazyLock;
use tokio::{runtime::Handle, sync::Mutex};

// Mutex that ensures e2e tests that create deployments are completed in isolation
// Important for avoiding naming conflicts and for counting how many docker containers were created
pub static DOCKER_TEST_MUTEX: LazyLock<Mutex<i32>> = LazyLock::new(|| Mutex::new(0));

#[derive(Default)]
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

#[allow(dead_code)]
pub async fn create_persistent_unrelated_container(
    docker: &Docker,
    name: &str,
) -> Result<(), bollard::errors::Error> {
    const ALPINE_IMAGE: &str = "alpine";
    const ALPINE_TAG: &str = "3.18.10";
    // Create Alpine image
    let create_image_options = CreateImageOptionsBuilder::default()
        .from_image(ALPINE_IMAGE)
        .tag(ALPINE_TAG)
        .build();

    // Start pulling the image, which returns a stream of progress events
    let mut stream = docker.create_image(Some(create_image_options), None, None);

    // Iterate over the stream and check for errors
    while let Some(result) = stream.next().await {
        let _image_info = result?;
    }

    // Create Alpine Container
    let options = Some(CreateContainerOptionsBuilder::default().name(name).build());
    let config = ContainerCreateBody {
        image: Some(format!("{ALPINE_IMAGE}:{ALPINE_TAG}").to_string()),
        cmd: Some(vec!["sleep".to_string(), "3600".to_string()]),
        ..Default::default()
    };
    docker.create_container(options, config).await?;

    // Start Container
    docker
        .start_container(name, None::<StartContainerOptions>)
        .await?;
    Ok(())
}
