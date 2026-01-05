use bollard::{
    Docker,
    container::LogOutput,
    errors::Error,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    query_parameters::{
        CreateContainerOptions, CreateImageOptionsBuilder, InspectContainerOptions,
        ListContainersOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions,
        StopContainerOptions,
    },
    secret::{
        ContainerCreateBody, ContainerCreateResponse, ContainerInspectResponse, ContainerSummary,
    },
};
use futures_util::{Stream, StreamExt};

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
            result?;
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

pub trait DockerPauseContainer {
    fn pause_container(&self, container_id: &str) -> impl Future<Output = Result<(), Error>>;
}

impl DockerPauseContainer for Docker {
    async fn pause_container(&self, container_id: &str) -> Result<(), Error> {
        self.pause_container(container_id).await
    }
}

pub trait DockerUnpauseContainer {
    fn unpause_container(&self, container_id: &str) -> impl Future<Output = Result<(), Error>>;
}

impl DockerUnpauseContainer for Docker {
    async fn unpause_container(&self, container_id: &str) -> Result<(), Error> {
        self.unpause_container(container_id).await
    }
}

pub trait RunCommandInContainer {
    fn run_command_in_container(
        &self,
        container_id: &str,
        command: Vec<String>,
    ) -> impl Future<Output = Result<CommandOutput, RunCommandInContainerError>>;
}

pub struct CommandOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RunCommandInContainerError {
    #[error("Failed to create exec: {0}")]
    CreateExec(Error),
    #[error("Failed to start exec: {0}")]
    StartExec(Error),
    #[error("Failed to get output, output was not attached")]
    GetOutput,
    #[error("Failed to get output: {0}")]
    GetOutputError(Error),
}

impl RunCommandInContainer for Docker {
    async fn run_command_in_container(
        &self,
        container_id: &str,
        command: Vec<String>,
    ) -> Result<CommandOutput, RunCommandInContainerError> {
        let exec = self
            .create_exec(
                container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(command),
                    ..Default::default()
                },
            )
            .await
            .map_err(RunCommandInContainerError::CreateExec)?;

        let exec = self
            .start_exec(
                &exec.id,
                Some(StartExecOptions {
                    detach: false,
                    tty: false,
                    output_capacity: None,
                }),
            )
            .await
            .map_err(RunCommandInContainerError::StartExec)?;

        let StartExecResults::Attached { mut output, .. } = exec else {
            return Err(RunCommandInContainerError::GetOutput);
        };

        let mut stdout = String::new();
        let mut stderr = String::new();

        while let Some(result) = output.next().await {
            let log_ouput = result.map_err(RunCommandInContainerError::GetOutputError)?;

            match log_ouput {
                LogOutput::StdOut { message } => {
                    stdout.push_str(&String::from_utf8_lossy(message.as_ref()));
                }
                LogOutput::StdErr { message } => {
                    stderr.push_str(&String::from_utf8_lossy(message.as_ref()));
                }
                _ => {}
            }
        }

        Ok(CommandOutput {
            stdout: stdout.lines().map(str::to_string).collect(),
            stderr: stderr.lines().map(str::to_string).collect(),
        })
    }
}

pub trait DockerLogContainer {
    fn logs<'a>(
        &'a self,
        container_id: &'a str,
        options: Option<LogsOptions>,
    ) -> impl Stream<Item = Result<LogOutput, Error>> + 'a;
}

impl DockerLogContainer for Docker {
    fn logs<'a>(
        &'a self,
        container_id: &'a str,
        options: Option<LogsOptions>,
    ) -> impl Stream<Item = Result<LogOutput, Error>> + 'a {
        self.logs(container_id, options)
    }
}
