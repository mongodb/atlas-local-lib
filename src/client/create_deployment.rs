use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bollard::{
    errors::Error::DockerResponseServerError,
    query_parameters::{CreateContainerOptions, StartContainerOptions},
    secret::ContainerCreateBody,
};
use futures::future::Fuse;
use futures_util::FutureExt;
use tokio::sync::oneshot::{self, Receiver, Sender, error::RecvError};

use crate::{
    GetDeploymentError,
    client::Client,
    docker::{
        DockerCreateContainer, DockerInspectContainer, DockerPullImage, DockerStartContainer,
    },
    models::{ATLAS_LOCAL_IMAGE, CreateDeploymentOptions, Deployment, WatchOptions},
};

use super::{PullImageError, WatchDeploymentError};

#[derive(Debug, thiserror::Error)]
pub enum CreateDeploymentError {
    #[error("Failed to create container: {0}")]
    CreateContainer(bollard::errors::Error),
    #[error(transparent)]
    PullImage(#[from] PullImageError),
    #[error("Container already exists: {0}")]
    ContainerAlreadyExists(String),
    #[error("Failed to check status of started container: {0}")]
    ContainerInspect(bollard::errors::Error),
    #[error("Created Deployment {0} is not healthy")]
    UnhealthyDeployment(String),
    #[error("Unable to get details for Deployment: {0}")]
    GetDeploymentError(GetDeploymentError),
    #[error("Error when waiting for deployment to become healthy: {0}")]
    WatchDeployment(#[from] WatchDeploymentError),
    #[error("Error when receiving deployment: {0}")]
    ReceiveDeployment(#[from] oneshot::error::RecvError),
}

impl<
    D: DockerPullImage
        + DockerCreateContainer
        + DockerStartContainer
        + DockerInspectContainer
        + Clone
        + Send
        + Sync
        + 'static,
> Client<D>
{
    /// Creates a local Atlas deployment.
    pub fn create_deployment(
        &self,
        deployment_options: CreateDeploymentOptions,
    ) -> CreateDeploymentProgress {
        let (sender, receiver) = create_progress_pairs();
        let client = self.clone();

        tokio::spawn(async move {
            let mut progress: CreateDeploymentProgressSender = sender;

            let result = client
                .create_deployment_inner(deployment_options, &mut progress)
                .await;

            progress.set_deployment(result).await;
        });

        receiver
    }

    async fn create_deployment_inner(
        &self,
        deployment_options: CreateDeploymentOptions,
        progress: &mut CreateDeploymentProgressSender,
    ) -> Result<Deployment, CreateDeploymentError> {
        // Pull the latest image for Atlas Local
        let will_pull_image = !deployment_options.skip_pull_image.unwrap_or(false);
        if will_pull_image {
            self.pull_image(
                deployment_options
                    .image
                    .as_ref()
                    .unwrap_or(&ATLAS_LOCAL_IMAGE.to_string()),
                deployment_options
                    .mongodb_version
                    .as_ref()
                    .map_or_else(|| "latest".to_string(), |version| version.to_string())
                    .as_ref(),
            )
            .await?;
        }

        progress
            .set_pull_image_finished(if will_pull_image {
                CreateDeploymentStepOutcome::Success
            } else {
                CreateDeploymentStepOutcome::Skipped
            })
            .await;

        // Create the container with the correct configuration
        let create_container_options: CreateContainerOptions = (&deployment_options).into();
        let create_container_config: ContainerCreateBody = (&deployment_options).into();

        // Get the cluster name
        // It is safe to unwrap because CreateContainerOptions::from will generate a random name if none is provided
        #[allow(clippy::expect_used)]
        let cluster_name = create_container_options
            .name
            .clone()
            .expect("Container name to be set by CreateContainerOptions::from");

        self.docker
            .create_container(Some(create_container_options), create_container_config)
            .await
            .map_err(|err| match err {
                DockerResponseServerError {
                    status_code: 409, ..
                } => CreateDeploymentError::ContainerAlreadyExists(cluster_name.to_string()),
                _ => CreateDeploymentError::CreateContainer(err),
            })?;

        progress
            .set_create_container_finished(CreateDeploymentStepOutcome::Success)
            .await;

        // Start the Atlas Local container
        self.docker
            .start_container(&cluster_name.to_string(), None::<StartContainerOptions>)
            .await
            .map_err(CreateDeploymentError::CreateContainer)?;

        progress
            .set_start_container_finished(CreateDeploymentStepOutcome::Success)
            .await;

        // Default to waiting for the deployment to be healthy
        let will_wait_for_healthy = deployment_options.wait_until_healthy.unwrap_or(true);
        if will_wait_for_healthy {
            let watch_options = WatchOptions {
                timeout_duration: deployment_options.wait_until_healthy_timeout,
                allow_unhealthy_initial_state: false,
            };
            self.wait_for_healthy_deployment(&cluster_name, watch_options)
                .await?;
        }

        progress
            .set_wait_for_healthy_deployment_finished(if will_wait_for_healthy {
                CreateDeploymentStepOutcome::Success
            } else {
                CreateDeploymentStepOutcome::Skipped
            })
            .await;

        // Return the deployment details
        self.get_deployment(&cluster_name)
            .await
            .map_err(CreateDeploymentError::GetDeploymentError)
    }
}

fn create_progress_pairs() -> (CreateDeploymentProgressSender, CreateDeploymentProgress) {
    let (pull_image_finished, pull_image_finished_receiver) = oneshot::channel();
    let (create_container_finished, create_container_finished_receiver) = oneshot::channel();
    let (start_container_finished, start_container_finished_receiver) = oneshot::channel();
    let (wait_for_healthy_deployment_finished, wait_for_healthy_deployment_finished_receiver) =
        oneshot::channel();
    let (deployment, deployment_receiver) = oneshot::channel();

    (
        CreateDeploymentProgressSender {
            pull_image_finished: Some(pull_image_finished),
            create_container_finished: Some(create_container_finished),
            start_container_finished: Some(start_container_finished),
            wait_for_healthy_deployment_finished: Some(wait_for_healthy_deployment_finished),
            deployment,
        },
        CreateDeploymentProgress {
            pull_image_finished: pull_image_finished_receiver.fuse(),
            create_container_finished: create_container_finished_receiver.fuse(),
            start_container_finished: start_container_finished_receiver.fuse(),
            wait_for_healthy_deployment_finished: wait_for_healthy_deployment_finished_receiver
                .fuse(),
            deployment: deployment_receiver.fuse(),
        },
    )
}

pub struct CreateDeploymentProgress {
    pub pull_image_finished: Fuse<Receiver<CreateDeploymentStepOutcome>>,
    pub create_container_finished: Fuse<Receiver<CreateDeploymentStepOutcome>>,
    pub start_container_finished: Fuse<Receiver<CreateDeploymentStepOutcome>>,
    pub wait_for_healthy_deployment_finished: Fuse<Receiver<CreateDeploymentStepOutcome>>,
    pub deployment: Fuse<Receiver<Result<Deployment, CreateDeploymentError>>>,
}

impl CreateDeploymentProgress {
    // Low level function to wait for a result from a receiver
    fn await_receiver<T>(
        receiver: &mut Fuse<Receiver<T>>,
    ) -> impl Future<Output = Result<T, RecvError>> {
        Pin::new(receiver).into_future()
    }

    pub async fn wait_for_pull_image_outcome(
        &mut self,
    ) -> Result<CreateDeploymentStepOutcome, RecvError> {
        Self::await_receiver(&mut self.pull_image_finished).await
    }

    pub async fn wait_for_create_container_outcome(
        &mut self,
    ) -> Result<CreateDeploymentStepOutcome, RecvError> {
        Self::await_receiver(&mut self.create_container_finished).await
    }

    pub async fn wait_for_start_container_outcome(
        &mut self,
    ) -> Result<CreateDeploymentStepOutcome, RecvError> {
        Self::await_receiver(&mut self.start_container_finished).await
    }

    pub async fn wait_for_wait_for_healthy_deployment_outcome(
        &mut self,
    ) -> Result<CreateDeploymentStepOutcome, RecvError> {
        Self::await_receiver(&mut self.wait_for_healthy_deployment_finished).await
    }

    pub async fn wait_for_deployment_outcome(
        &mut self,
    ) -> Result<Deployment, CreateDeploymentError> {
        // We use the Future implementation to wait for the deployment outcome
        self.await
    }
}

impl Future for CreateDeploymentProgress {
    type Output = Result<Deployment, CreateDeploymentError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.deployment).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(error)) => {
                Poll::Ready(Err(CreateDeploymentError::ReceiveDeployment(error)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

struct CreateDeploymentProgressSender {
    pull_image_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    create_container_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    start_container_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    wait_for_healthy_deployment_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    deployment: Sender<Result<Deployment, CreateDeploymentError>>,
}

impl CreateDeploymentProgressSender {
    // Send the outcome to a sender if present
    // Returns true if the outcome was sent, false if the sender was not present
    async fn send_outcome(
        sender: &mut Option<Sender<CreateDeploymentStepOutcome>>,
        outcome: CreateDeploymentStepOutcome,
    ) -> bool {
        if let Some(sender) = sender.take() {
            // An error occurs when there is not receiver, this is expected behavior that is safe to ignore
            if sender.send(outcome).is_ok() {
                return true;
            }
        }

        false
    }

    async fn set_pull_image_finished(&mut self, outcome: CreateDeploymentStepOutcome) {
        Self::send_outcome(&mut self.pull_image_finished, outcome).await;
    }

    async fn set_create_container_finished(&mut self, outcome: CreateDeploymentStepOutcome) {
        Self::send_outcome(&mut self.create_container_finished, outcome).await;
    }

    async fn set_start_container_finished(&mut self, outcome: CreateDeploymentStepOutcome) {
        Self::send_outcome(&mut self.start_container_finished, outcome).await;
    }

    async fn set_wait_for_healthy_deployment_finished(
        &mut self,
        outcome: CreateDeploymentStepOutcome,
    ) {
        Self::send_outcome(&mut self.wait_for_healthy_deployment_finished, outcome).await;
    }

    /// Set the deployment, this is the last step and consumes the sender
    async fn set_deployment(mut self, result: Result<Deployment, CreateDeploymentError>) {
        // To ensure that all steps are marked as either success, failure, or skipped
        // We loop through all steps and send skipped if no message was sent to the channel yet (we can only send one message to a channel, so it's safe to just send skipped to all channels)
        // In case of an error, we mark the first step that has not been marked as success as failure, mark the rest as skipped
        let mut outcome = if result.is_err() {
            CreateDeploymentStepOutcome::Failure
        } else {
            CreateDeploymentStepOutcome::Skipped
        };

        // Helper function to send the outcome to a sender if present and mark the next steps as skipped if the outcome was sent
        let send_failure_or_skipped =
            async |outcome: &mut CreateDeploymentStepOutcome,
                   sender: &mut Option<Sender<CreateDeploymentStepOutcome>>| {
                if Self::send_outcome(sender, *outcome).await {
                    // If the outcome was sent this means that the step was not successful
                    // So the next steps should be marked as skipped
                    *outcome = CreateDeploymentStepOutcome::Skipped;
                }
            };

        // All steps in order of execution
        send_failure_or_skipped(&mut outcome, &mut self.pull_image_finished).await;
        send_failure_or_skipped(&mut outcome, &mut self.create_container_finished).await;
        send_failure_or_skipped(&mut outcome, &mut self.start_container_finished).await;
        send_failure_or_skipped(&mut outcome, &mut self.wait_for_healthy_deployment_finished).await;

        // An error occurs when there is not receiver, this is expected behavior that is safe to ignore
        _ = self.deployment.send(result);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CreateDeploymentStepOutcome {
    Success,
    Skipped,
    Failure,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::WatchDeploymentError;
    use bollard::{
        errors::Error as BollardError,
        query_parameters::InspectContainerOptions,
        secret::{
            ContainerConfig, ContainerCreateResponse, ContainerInspectResponse, ContainerState,
            ContainerStateStatusEnum, HealthStatusEnum,
        },
    };
    use maplit::hashmap;
    use mockall::mock;
    use pretty_assertions::assert_eq;
    use tokio::time;

    mock! {
        Docker {}

        impl Clone for Docker {
            fn clone(&self) -> Self;
        }

        impl DockerPullImage for Docker {
            async fn pull_image(&self, image: &str, tag: &str) -> Result<(), BollardError>;
        }

        impl DockerCreateContainer for Docker {
            async fn create_container(
                &self,
                options: Option<CreateContainerOptions>,
                config: ContainerCreateBody,
            ) -> Result<ContainerCreateResponse, BollardError>;
        }

        impl DockerStartContainer for Docker {
            async fn start_container(
                &self,
                container_id: &str,
                options: Option<StartContainerOptions>,
            ) -> Result<(), BollardError>;
        }

        impl DockerInspectContainer for Docker {
            async fn inspect_container(
                &self,
                container_id: &str,
                options: Option<InspectContainerOptions>,
            ) -> Result<ContainerInspectResponse, BollardError>;
        }
    }

    fn create_test_container_inspect_response() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                health: Some(bollard::secret::Health {
                    status: Some(HealthStatusEnum::HEALTHY),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_unhealthy() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                health: Some(bollard::secret::Health {
                    status: Some(HealthStatusEnum::UNHEALTHY),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_starting() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                health: Some(bollard::secret::Health {
                    status: Some(HealthStatusEnum::STARTING),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_no_state() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: None,
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_no_health() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                health: None,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_test_container_inspect_response_no_health_status() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
                ..Default::default()
            }),
            state: Some(ContainerState {
                health: Some(bollard::secret::Health {
                    status: None,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_create_deployment() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(2)
            .returning(|_, _| Ok(create_test_container_inspect_response()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_deployment_pull_image_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker.expect_pull_image().times(1).returning(|_, _| {
            Err(BollardError::DockerResponseServerError {
                status_code: 500,
                message: "Internal Server Error".to_string(),
            })
        });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::PullImage(_)
        ));
    }

    #[tokio::test]
    async fn test_create_deployment_container_already_exists() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 409,
                    message: "Conflict".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::ContainerAlreadyExists(name) => {
                assert_eq!(name, "test-deployment");
            }
            _ => panic!("Expected ContainerAlreadyExists error"),
        }
    }

    #[tokio::test]
    async fn test_create_deployment_create_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::CreateContainer(_)
        ));
    }

    #[tokio::test]
    async fn test_create_deployment_start_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::CreateContainer(_)
        ));
    }

    #[tokio::test]
    async fn test_create_deployment_wait_for_healthy_deployment_unhealthy() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_unhealthy()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreateDeploymentError::WatchDeployment(
                WatchDeploymentError::UnhealthyDeployment { .. }
            )
        ));
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_retries() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_starting()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(2)
            .returning(|_, _| Ok(create_test_container_inspect_response()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_disabled() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            wait_until_healthy: Some(false),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_deployment_timeout() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            wait_until_healthy: Some(true),
            wait_until_healthy_timeout: Some(time::Duration::from_millis(1)),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        // Mock inspect_container to always return STARTING status, which will cause timeout
        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .returning(|_, _| Ok(create_test_container_inspect_response_starting()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::WatchDeployment(WatchDeploymentError::Timeout {
                deployment_name,
            }) => {
                assert_eq!(deployment_name, "test-deployment");
            }
            _ => panic!("Expected WatchDeployment Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_no_state() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_no_state()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::WatchDeployment(WatchDeploymentError::UnhealthyDeployment {
                deployment_name,
                status,
            }) => {
                assert_eq!(deployment_name, "test-deployment");
                assert_eq!(status, HealthStatusEnum::NONE);
            }
            _ => panic!("Expected WatchDeployment error"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_no_health() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_no_health()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::WatchDeployment(WatchDeploymentError::UnhealthyDeployment {
                deployment_name,
                status,
            }) => {
                assert_eq!(deployment_name, "test-deployment");
                assert_eq!(status, HealthStatusEnum::NONE);
            }
            _ => panic!("Expected WatchDeployment error"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_healthy_deployment_no_health_status() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let options = CreateDeploymentOptions {
            name: Some("test-deployment".to_string()),
            ..Default::default()
        };

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq(ATLAS_LOCAL_IMAGE),
                mockall::predicate::eq("latest"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_create_container()
            .times(1)
            .returning(|_, _| {
                Ok(ContainerCreateResponse {
                    id: "container_id".to_string(),
                    warnings: vec![],
                })
            });

        mock_docker
            .expect_start_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<StartContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("test-deployment"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| Ok(create_test_container_inspect_response_no_health_status()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.create_deployment(options).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::WatchDeployment(WatchDeploymentError::UnhealthyDeployment {
                deployment_name,
                status,
            }) => {
                assert_eq!(deployment_name, "test-deployment");
                assert_eq!(status, HealthStatusEnum::NONE);
            }
            _ => panic!("Expected WatchDeployment error"),
        }
    }
}
