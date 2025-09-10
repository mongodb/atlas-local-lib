use bollard::{
    Docker,
    errors::Error,
    query_parameters::{
        CreateContainerOptions, CreateImageOptionsBuilder, InspectContainerOptions,
        ListContainersOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
    },
    secret::{
        ContainerCreateBody, ContainerCreateResponse, ContainerInspectResponse, ContainerSummary,
    },
};
use futures_util::StreamExt;

pub trait DockerInspectContainer {
    fn inspect_container(
        &self,
        container_id: &str,
        options: Option<InspectContainerOptions>,
    ) -> impl Future<Output = Result<ContainerInspectResponse, Error>>;
}

impl DockerInspectContainer for Docker {
    async fn inspect_container(
        &self,
        container_id: &str,
        options: Option<InspectContainerOptions>,
    ) -> Result<ContainerInspectResponse, Error> {
        self.inspect_container(container_id, options).await
    }
}

pub trait DockerListContainers {
    fn list_containers(
        &self,
        options: Option<ListContainersOptions>,
    ) -> impl Future<Output = Result<Vec<ContainerSummary>, Error>>;
}

impl DockerListContainers for Docker {
    async fn list_containers(
        &self,
        options: Option<ListContainersOptions>,
    ) -> Result<Vec<ContainerSummary>, Error> {
        self.list_containers(options).await
    }
}

pub trait DockerPullImage {
    fn pull_image(&self, image: &str, tag: &str) -> impl Future<Output = Result<(), Error>>;
}

impl DockerPullImage for Docker {
    async fn pull_image(&self, image: &str, tag: &str) -> Result<(), Error> {
        // Build the options for pulling the Atlas Local Docker image
        let create_image_options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .tag(tag)
            .build();

        // Start pulling the image, which returns a stream of progress events
        let mut stream = self.create_image(Some(create_image_options), None, None);

        // Iterate over the stream and check for errors
        while let Some(result) = stream.next().await {
            if let Err(e) = result {
                return Err(e);
            }
        }

        Ok(())
    }
}

pub trait DockerStopContainer {
    fn stop_container(
        &self,
        container_id: &str,
        options: Option<StopContainerOptions>,
    ) -> impl Future<Output = Result<(), Error>>;
}

impl DockerStopContainer for Docker {
    async fn stop_container(
        &self,
        container_id: &str,
        options: Option<StopContainerOptions>,
    ) -> Result<(), Error> {
        self.stop_container(container_id, options).await
    }
}

pub trait DockerRemoveContainer {
    fn remove_container(
        &self,
        container_id: &str,
        options: Option<RemoveContainerOptions>,
    ) -> impl Future<Output = Result<(), Error>>;
}

impl DockerRemoveContainer for Docker {
    async fn remove_container(
        &self,
        container_id: &str,
        options: Option<RemoveContainerOptions>,
    ) -> Result<(), Error> {
        self.remove_container(container_id, options).await
    }
}

pub trait DockerCreateContainer {
    fn create_container(
        &self,
        options: Option<CreateContainerOptions>,
        config: ContainerCreateBody,
    ) -> impl Future<Output = Result<ContainerCreateResponse, Error>>;
}

impl DockerCreateContainer for Docker {
    async fn create_container(
        &self,
        options: Option<CreateContainerOptions>,
        config: ContainerCreateBody,
    ) -> Result<ContainerCreateResponse, Error> {
        self.create_container(options, config).await
    }
}

pub trait DockerStartContainer {
    fn start_container(
        &self,
        container_id: &str,
        options: Option<StartContainerOptions>,
    ) -> impl Future<Output = Result<(), Error>>;
}

impl DockerStartContainer for Docker {
    async fn start_container(
        &self,
        container_id: &str,
        options: Option<StartContainerOptions>,
    ) -> Result<(), Error> {
        self.start_container(container_id, options).await
    }
}
