use bollard::{
    Docker,
    container::LogOutput,
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
use futures_util::{Stream, StreamExt, TryStreamExt};

use crate::models::ContainerHealthStatus;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DockerError {
    #[error("resource not modified")]
    NotModified,
    #[error("bad request")]
    BadRequest,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("internal server error")]
    ServerError,
    #[error("docker error (status {status_code:?}): {message}")]
    Other {
        status_code: Option<u16>,
        message: String,
    },
}

impl From<bollard::errors::Error> for DockerError {
    fn from(err: bollard::errors::Error) -> Self {
        match err {
            bollard::errors::Error::DockerResponseServerError {
                status_code,
                message,
            } => match status_code {
                304 => DockerError::NotModified,
                400 => DockerError::BadRequest,
                401 => DockerError::Unauthorized,
                403 => DockerError::Forbidden,
                404 => DockerError::NotFound,
                409 => DockerError::Conflict,
                500 => DockerError::ServerError,
                _ => DockerError::Other {
                    status_code: Some(status_code),
                    message,
                },
            },
            _ => DockerError::Other {
                status_code: None,
                message: err.to_string(),
            },
        }
    }
}

impl From<bollard::secret::HealthStatusEnum> for ContainerHealthStatus {
    fn from(status: bollard::secret::HealthStatusEnum) -> Self {
        match status {
            bollard::secret::HealthStatusEnum::EMPTY => ContainerHealthStatus::Empty,
            bollard::secret::HealthStatusEnum::HEALTHY => ContainerHealthStatus::Healthy,
            bollard::secret::HealthStatusEnum::UNHEALTHY => ContainerHealthStatus::Unhealthy,
            bollard::secret::HealthStatusEnum::NONE => ContainerHealthStatus::None,
            bollard::secret::HealthStatusEnum::STARTING => ContainerHealthStatus::Starting,
        }
    }
}

pub trait DockerInspectContainer {
    fn inspect_container(
        &self,
        container_id: &str,
        options: Option<InspectContainerOptions>,
    ) -> impl Future<Output = Result<ContainerInspectResponse, DockerError>> + Send;
}

impl DockerInspectContainer for Docker {
    async fn inspect_container(
        &self,
        container_id: &str,
        options: Option<InspectContainerOptions>,
    ) -> Result<ContainerInspectResponse, DockerError> {
        self.inspect_container(container_id, options)
            .await
            .map_err(DockerError::from)
    }
}

pub trait DockerListContainers {
    fn list_containers(
        &self,
        options: Option<ListContainersOptions>,
    ) -> impl Future<Output = Result<Vec<ContainerSummary>, DockerError>> + Send;
}

impl DockerListContainers for Docker {
    async fn list_containers(
        &self,
        options: Option<ListContainersOptions>,
    ) -> Result<Vec<ContainerSummary>, DockerError> {
        self.list_containers(options)
            .await
            .map_err(DockerError::from)
    }
}

pub trait DockerPullImage {
    fn pull_image(
        &self,
        image: &str,
        tag: &str,
    ) -> impl Future<Output = Result<(), DockerError>> + Send;
}

impl DockerPullImage for Docker {
    async fn pull_image(&self, image: &str, tag: &str) -> Result<(), DockerError> {
        let create_image_options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .tag(tag)
            .build();

        let mut stream = self.create_image(Some(create_image_options), None, None);

        while let Some(result) = stream.next().await {
            result.map_err(DockerError::from)?;
        }

        Ok(())
    }
}

pub trait DockerStopContainer {
    fn stop_container(
        &self,
        container_id: &str,
        options: Option<StopContainerOptions>,
    ) -> impl Future<Output = Result<(), DockerError>> + Send;
}

impl DockerStopContainer for Docker {
    async fn stop_container(
        &self,
        container_id: &str,
        options: Option<StopContainerOptions>,
    ) -> Result<(), DockerError> {
        self.stop_container(container_id, options)
            .await
            .map_err(DockerError::from)
    }
}

pub trait DockerRemoveContainer {
    fn remove_container(
        &self,
        container_id: &str,
        options: Option<RemoveContainerOptions>,
    ) -> impl Future<Output = Result<(), DockerError>> + Send;
}

impl DockerRemoveContainer for Docker {
    async fn remove_container(
        &self,
        container_id: &str,
        options: Option<RemoveContainerOptions>,
    ) -> Result<(), DockerError> {
        self.remove_container(container_id, options)
            .await
            .map_err(DockerError::from)
    }
}

pub trait DockerCreateContainer {
    fn create_container(
        &self,
        options: Option<CreateContainerOptions>,
        config: ContainerCreateBody,
    ) -> impl Future<Output = Result<ContainerCreateResponse, DockerError>> + Send;
}

impl DockerCreateContainer for Docker {
    async fn create_container(
        &self,
        options: Option<CreateContainerOptions>,
        config: ContainerCreateBody,
    ) -> Result<ContainerCreateResponse, DockerError> {
        self.create_container(options, config)
            .await
            .map_err(DockerError::from)
    }
}

pub trait DockerStartContainer {
    fn start_container(
        &self,
        container_id: &str,
        options: Option<StartContainerOptions>,
    ) -> impl Future<Output = Result<(), DockerError>> + Send;
}

impl DockerStartContainer for Docker {
    async fn start_container(
        &self,
        container_id: &str,
        options: Option<StartContainerOptions>,
    ) -> Result<(), DockerError> {
        self.start_container(container_id, options)
            .await
            .map_err(DockerError::from)
    }
}

pub trait DockerPauseContainer {
    fn pause_container(
        &self,
        container_id: &str,
    ) -> impl Future<Output = Result<(), DockerError>> + Send;
}

impl DockerPauseContainer for Docker {
    async fn pause_container(&self, container_id: &str) -> Result<(), DockerError> {
        self.pause_container(container_id)
            .await
            .map_err(DockerError::from)
    }
}

pub trait DockerUnpauseContainer {
    fn unpause_container(
        &self,
        container_id: &str,
    ) -> impl Future<Output = Result<(), DockerError>> + Send;
}

impl DockerUnpauseContainer for Docker {
    async fn unpause_container(&self, container_id: &str) -> Result<(), DockerError> {
        self.unpause_container(container_id)
            .await
            .map_err(DockerError::from)
    }
}

pub trait RunCommandInContainer {
    fn run_command_in_container(
        &self,
        container_id: &str,
        command: Vec<String>,
    ) -> impl Future<Output = Result<CommandOutput, RunCommandInContainerError>> + Send;
}

pub struct CommandOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RunCommandInContainerError {
    #[error("Failed to create exec: {0}")]
    CreateExec(DockerError),
    #[error("Failed to start exec: {0}")]
    StartExec(DockerError),
    #[error("Failed to get output, output was not attached")]
    GetOutput,
    #[error("Failed to get output: {0}")]
    GetOutputError(DockerError),
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
            .map_err(|e| RunCommandInContainerError::CreateExec(DockerError::from(e)))?;

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
            .map_err(|e| RunCommandInContainerError::StartExec(DockerError::from(e)))?;

        let StartExecResults::Attached { mut output, .. } = exec else {
            return Err(RunCommandInContainerError::GetOutput);
        };

        let mut stdout = String::new();
        let mut stderr = String::new();

        while let Some(result) = output.next().await {
            let log_ouput = result
                .map_err(|e| RunCommandInContainerError::GetOutputError(DockerError::from(e)))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_error_from_bollard_not_modified() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 304,
            message: "Not Modified".to_string(),
        };
        assert_eq!(DockerError::from(err), DockerError::NotModified);
    }

    #[test]
    fn test_docker_error_from_bollard_bad_request() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 400,
            message: "Bad Request".to_string(),
        };
        assert_eq!(DockerError::from(err), DockerError::BadRequest);
    }

    #[test]
    fn test_docker_error_from_bollard_unauthorized() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 401,
            message: "Unauthorized".to_string(),
        };
        assert_eq!(DockerError::from(err), DockerError::Unauthorized);
    }

    #[test]
    fn test_docker_error_from_bollard_forbidden() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 403,
            message: "Forbidden".to_string(),
        };
        assert_eq!(DockerError::from(err), DockerError::Forbidden);
    }

    #[test]
    fn test_docker_error_from_bollard_not_found() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(DockerError::from(err), DockerError::NotFound);
    }

    #[test]
    fn test_docker_error_from_bollard_conflict() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 409,
            message: "Conflict".to_string(),
        };
        assert_eq!(DockerError::from(err), DockerError::Conflict);
    }

    #[test]
    fn test_docker_error_from_bollard_server_error() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 500,
            message: "Internal Server Error".to_string(),
        };
        assert_eq!(DockerError::from(err), DockerError::ServerError);
    }

    #[test]
    fn test_docker_error_from_bollard_other_status_code() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 503,
            message: "Service Unavailable".to_string(),
        };
        assert_eq!(
            DockerError::from(err),
            DockerError::Other {
                status_code: Some(503),
                message: "Service Unavailable".to_string(),
            }
        );
    }

    #[test]
    fn test_docker_error_from_bollard_non_server_error() {
        let err = bollard::errors::Error::RequestTimeoutError;
        let result = DockerError::from(err);
        assert!(matches!(
            result,
            DockerError::Other {
                status_code: None,
                ..
            }
        ));
    }

    #[test]
    fn test_docker_error_display() {
        assert_eq!(DockerError::NotModified.to_string(), "resource not modified");
        assert_eq!(DockerError::BadRequest.to_string(), "bad request");
        assert_eq!(DockerError::Unauthorized.to_string(), "unauthorized");
        assert_eq!(DockerError::Forbidden.to_string(), "forbidden");
        assert_eq!(DockerError::NotFound.to_string(), "not found");
        assert_eq!(DockerError::Conflict.to_string(), "conflict");
        assert_eq!(
            DockerError::ServerError.to_string(),
            "internal server error"
        );
        assert_eq!(
            DockerError::Other {
                status_code: Some(503),
                message: "oops".to_string(),
            }
            .to_string(),
            "docker error (status Some(503)): oops"
        );
    }

    #[test]
    fn test_container_health_status_from_bollard_empty() {
        assert_eq!(
            ContainerHealthStatus::from(bollard::secret::HealthStatusEnum::EMPTY),
            ContainerHealthStatus::Empty
        );
    }

    #[test]
    fn test_container_health_status_from_bollard_healthy() {
        assert_eq!(
            ContainerHealthStatus::from(bollard::secret::HealthStatusEnum::HEALTHY),
            ContainerHealthStatus::Healthy
        );
    }

    #[test]
    fn test_container_health_status_from_bollard_unhealthy() {
        assert_eq!(
            ContainerHealthStatus::from(bollard::secret::HealthStatusEnum::UNHEALTHY),
            ContainerHealthStatus::Unhealthy
        );
    }

    #[test]
    fn test_container_health_status_from_bollard_none() {
        assert_eq!(
            ContainerHealthStatus::from(bollard::secret::HealthStatusEnum::NONE),
            ContainerHealthStatus::None
        );
    }

    #[test]
    fn test_container_health_status_from_bollard_starting() {
        assert_eq!(
            ContainerHealthStatus::from(bollard::secret::HealthStatusEnum::STARTING),
            ContainerHealthStatus::Starting
        );
    }
}

pub trait DockerLogContainer {
    fn logs<'a>(
        &'a self,
        container_id: &'a str,
        options: Option<LogsOptions>,
    ) -> impl Stream<Item = Result<LogOutput, String>> + 'a;
}

impl DockerLogContainer for Docker {
    fn logs<'a>(
        &'a self,
        container_id: &'a str,
        options: Option<LogsOptions>,
    ) -> impl Stream<Item = Result<LogOutput, String>> + 'a {
        self.logs(container_id, options).map_err(|e| e.to_string())
    }
}
