use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::future::Fuse;
use futures_util::FutureExt;
use tokio::sync::oneshot::{self, Receiver, Sender, error::RecvError};

use crate::models::Deployment;

use super::CreateDeploymentError;

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
    ) -> impl std::future::Future<Output = Result<T, RecvError>> {
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

impl std::future::Future for CreateDeploymentProgress {
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

pub struct CreateDeploymentProgressSender {
    pub pull_image_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    pub create_container_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    pub start_container_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    pub wait_for_healthy_deployment_finished: Option<Sender<CreateDeploymentStepOutcome>>,
    pub deployment: Sender<Result<Deployment, CreateDeploymentError>>,
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

    pub async fn set_pull_image_finished(&mut self, outcome: CreateDeploymentStepOutcome) {
        Self::send_outcome(&mut self.pull_image_finished, outcome).await;
    }

    pub async fn set_create_container_finished(&mut self, outcome: CreateDeploymentStepOutcome) {
        Self::send_outcome(&mut self.create_container_finished, outcome).await;
    }

    pub async fn set_start_container_finished(&mut self, outcome: CreateDeploymentStepOutcome) {
        Self::send_outcome(&mut self.start_container_finished, outcome).await;
    }

    pub async fn set_wait_for_healthy_deployment_finished(
        &mut self,
        outcome: CreateDeploymentStepOutcome,
    ) {
        Self::send_outcome(&mut self.wait_for_healthy_deployment_finished, outcome).await;
    }

    /// Finalizes the deployment process by marking all remaining steps and sending the final result.
    ///
    /// This method completes the deployment workflow by:
    /// - Marking any uncompleted steps as skipped (or failure if an error occurred)
    /// - Sending the final deployment result to the receiver
    /// - Consuming the sender to signal completion
    pub async fn finalize_deployment(mut self, result: Result<Deployment, CreateDeploymentError>) {
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

pub fn create_progress_pairs() -> (CreateDeploymentProgressSender, CreateDeploymentProgress) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MongodbType, State};
    use semver::Version;

    fn create_test_deployment() -> Deployment {
        Deployment {
            container_id: "test_container_id".to_string(),
            name: Some("test-deployment".to_string()),
            state: State::Running,
            port_bindings: None,
            mongodb_type: MongodbType::Community,
            mongodb_version: Version::new(8, 0, 0),
            creation_source: None,
            local_seed_location: None,
            mongodb_initdb_database: None,
            mongodb_initdb_root_password_file: None,
            mongodb_initdb_root_password: None,
            mongodb_initdb_root_username_file: None,
            mongodb_initdb_root_username: None,
            mongodb_load_sample_data: None,
            voyage_api_key: None,
            mongot_log_file: None,
            runner_log_file: None,
            do_not_track: false,
            telemetry_base_url: None,
        }
    }

    async fn create_test_error() -> CreateDeploymentError {
        // Create a channel and drop the sender, then try to receive to get RecvError
        let (sender, receiver) = oneshot::channel::<Result<Deployment, CreateDeploymentError>>();
        drop(sender);
        // Awaiting on a dropped sender's receiver will give us RecvError
        CreateDeploymentError::ReceiveDeployment(receiver.await.unwrap_err())
    }

    #[tokio::test]
    async fn test_create_progress_pairs() {
        let (mut sender, mut progress) = create_progress_pairs();

        // Verify all senders are present
        assert!(sender.pull_image_finished.is_some());
        assert!(sender.create_container_finished.is_some());
        assert!(sender.start_container_finished.is_some());
        assert!(sender.wait_for_healthy_deployment_finished.is_some());

        // Verify we can send and receive
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;
        let outcome = progress.wait_for_pull_image_outcome().await.unwrap();
        assert_eq!(outcome, CreateDeploymentStepOutcome::Success);
    }

    #[tokio::test]
    async fn test_wait_for_pull_image_outcome() {
        let (mut sender, mut progress) = create_progress_pairs();

        // Test success
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress.wait_for_pull_image_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );

        // Test failure
        let (mut sender, mut progress) = create_progress_pairs();
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Failure)
            .await;
        assert_eq!(
            progress.wait_for_pull_image_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Failure
        );

        // Test skipped
        let (mut sender, mut progress) = create_progress_pairs();
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Skipped)
            .await;
        assert_eq!(
            progress.wait_for_pull_image_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Skipped
        );
    }

    #[tokio::test]
    async fn test_wait_for_create_container_outcome() {
        let (mut sender, mut progress) = create_progress_pairs();

        sender
            .set_create_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress.wait_for_create_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );
    }

    #[tokio::test]
    async fn test_wait_for_start_container_outcome() {
        let (mut sender, mut progress) = create_progress_pairs();

        sender
            .set_start_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress.wait_for_start_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );
    }

    #[tokio::test]
    async fn test_wait_for_wait_for_healthy_deployment_outcome() {
        let (mut sender, mut progress) = create_progress_pairs();

        sender
            .set_wait_for_healthy_deployment_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress
                .wait_for_wait_for_healthy_deployment_outcome()
                .await
                .unwrap(),
            CreateDeploymentStepOutcome::Success
        );
    }

    #[tokio::test]
    async fn test_wait_for_deployment_outcome_success() {
        let (sender, mut progress) = create_progress_pairs();

        let deployment = create_test_deployment();
        sender.finalize_deployment(Ok(deployment.clone())).await;

        let result = progress.wait_for_deployment_outcome().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), deployment);
    }

    #[tokio::test]
    async fn test_wait_for_deployment_outcome_error() {
        let (sender, mut progress) = create_progress_pairs();

        let error = create_test_error().await;
        sender.finalize_deployment(Err(error)).await;

        let result = progress.wait_for_deployment_outcome().await;
        assert!(result.is_err());
        // We can't directly compare errors, but we can check the error type
        match result.unwrap_err() {
            CreateDeploymentError::ReceiveDeployment(_) => {}
            _ => panic!("Expected ReceiveDeployment error"),
        }
    }

    #[tokio::test]
    async fn test_future_implementation_success() {
        let (sender, progress) = create_progress_pairs();

        let deployment = create_test_deployment();
        sender.finalize_deployment(Ok(deployment.clone())).await;

        // Use the Future implementation directly
        let result = progress.await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), deployment);
    }

    #[tokio::test]
    async fn test_future_implementation_error() {
        let (sender, progress) = create_progress_pairs();

        let error = create_test_error().await;
        sender.finalize_deployment(Err(error)).await;

        // Use the Future implementation directly
        let result = progress.await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::ReceiveDeployment(_) => {}
            _ => panic!("Expected ReceiveDeployment error"),
        }
    }

    #[tokio::test]
    async fn test_set_pull_image_finished() {
        let (mut sender, mut progress) = create_progress_pairs();

        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress.wait_for_pull_image_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );

        // Setting again should be a no-op (sender is consumed)
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Failure)
            .await;
        // The receiver already received the first message, so this won't affect it
    }

    #[tokio::test]
    async fn test_set_create_container_finished() {
        let (mut sender, mut progress) = create_progress_pairs();

        sender
            .set_create_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress.wait_for_create_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );
    }

    #[tokio::test]
    async fn test_set_start_container_finished() {
        let (mut sender, mut progress) = create_progress_pairs();

        sender
            .set_start_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress.wait_for_start_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );
    }

    #[tokio::test]
    async fn test_set_wait_for_healthy_deployment_finished() {
        let (mut sender, mut progress) = create_progress_pairs();

        sender
            .set_wait_for_healthy_deployment_finished(CreateDeploymentStepOutcome::Success)
            .await;
        assert_eq!(
            progress
                .wait_for_wait_for_healthy_deployment_outcome()
                .await
                .unwrap(),
            CreateDeploymentStepOutcome::Success
        );
    }

    #[tokio::test]
    async fn test_finalize_deployment_success_all_steps_completed() {
        let (mut sender, mut progress) = create_progress_pairs();

        // Mark all steps as successful
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;
        sender
            .set_create_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        sender
            .set_start_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        sender
            .set_wait_for_healthy_deployment_finished(CreateDeploymentStepOutcome::Success)
            .await;

        let deployment = create_test_deployment();
        sender.finalize_deployment(Ok(deployment.clone())).await;

        // All steps should already be marked as success, finalize shouldn't change them
        // But we can verify the deployment result
        let result = progress.wait_for_deployment_outcome().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), deployment);
    }

    #[tokio::test]
    async fn test_finalize_deployment_success_some_steps_skipped() {
        let (mut sender, mut progress) = create_progress_pairs();

        // Mark only some steps as successful
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;
        sender
            .set_create_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        // start_container and wait_for_healthy_deployment are not set

        let deployment = create_test_deployment();
        sender.finalize_deployment(Ok(deployment.clone())).await;

        // Verify the unset steps are marked as skipped
        assert_eq!(
            progress.wait_for_start_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Skipped
        );
        assert_eq!(
            progress
                .wait_for_wait_for_healthy_deployment_outcome()
                .await
                .unwrap(),
            CreateDeploymentStepOutcome::Skipped
        );

        // Verify deployment result
        let result = progress.wait_for_deployment_outcome().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), deployment);
    }

    #[tokio::test]
    async fn test_finalize_deployment_error_all_steps_uncompleted() {
        let (sender, mut progress) = create_progress_pairs();

        let error = create_test_error().await;
        sender.finalize_deployment(Err(error)).await;

        // All steps should be marked as failure (first) or skipped (rest)
        assert_eq!(
            progress.wait_for_pull_image_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Failure
        );
        assert_eq!(
            progress.wait_for_create_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Skipped
        );
        assert_eq!(
            progress.wait_for_start_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Skipped
        );
        assert_eq!(
            progress
                .wait_for_wait_for_healthy_deployment_outcome()
                .await
                .unwrap(),
            CreateDeploymentStepOutcome::Skipped
        );

        // Verify deployment error
        let result = progress.wait_for_deployment_outcome().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CreateDeploymentError::ReceiveDeployment(_) => {}
            _ => panic!("Expected ReceiveDeployment error"),
        }
    }

    #[tokio::test]
    async fn test_finalize_deployment_error_some_steps_completed() {
        let (mut sender, mut progress) = create_progress_pairs();

        // Mark some steps as successful before error
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;
        sender
            .set_create_container_finished(CreateDeploymentStepOutcome::Success)
            .await;
        // start_container fails, so it should be marked as failure
        // wait_for_healthy_deployment should be skipped

        let error = create_test_error().await;
        sender.finalize_deployment(Err(error)).await;

        // Already completed steps remain as success
        assert_eq!(
            progress.wait_for_pull_image_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );
        assert_eq!(
            progress.wait_for_create_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Success
        );

        // First uncompleted step should be failure
        assert_eq!(
            progress.wait_for_start_container_outcome().await.unwrap(),
            CreateDeploymentStepOutcome::Failure
        );

        // Remaining steps should be skipped
        assert_eq!(
            progress
                .wait_for_wait_for_healthy_deployment_outcome()
                .await
                .unwrap(),
            CreateDeploymentStepOutcome::Skipped
        );

        // Verify deployment error
        let result = progress.wait_for_deployment_outcome().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_outcome_when_sender_absent() {
        let (mut sender, _progress) = create_progress_pairs();

        // Consume the sender by sending
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;

        // Try to send again - should be a no-op (sender is None)
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Failure)
            .await;
        // This should not panic or error
    }

    #[tokio::test]
    async fn test_send_outcome_when_receiver_dropped() {
        let (mut sender, progress) = create_progress_pairs();

        // Drop the receiver
        drop(progress);

        // Try to send - should handle gracefully (send fails but doesn't panic)
        sender
            .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
            .await;
        // This should not panic
    }

    #[tokio::test]
    async fn test_finalize_deployment_when_receiver_dropped() {
        let (sender, progress) = create_progress_pairs();

        // Drop the receiver
        drop(progress);

        // Finalize should handle gracefully
        let deployment = create_test_deployment();
        sender.finalize_deployment(Ok(deployment)).await;
        // This should not panic
    }

    #[test]
    fn test_create_deployment_step_outcome_debug() {
        let outcome = CreateDeploymentStepOutcome::Success;
        let debug_str = format!("{:?}", outcome);
        assert!(debug_str.contains("Success"));
    }

    #[test]
    fn test_create_deployment_step_outcome_clone() {
        let outcome = CreateDeploymentStepOutcome::Success;
        let cloned = outcome;
        assert_eq!(outcome, cloned);
    }

    #[test]
    fn test_create_deployment_step_outcome_partial_eq() {
        assert_eq!(
            CreateDeploymentStepOutcome::Success,
            CreateDeploymentStepOutcome::Success
        );
        assert_eq!(
            CreateDeploymentStepOutcome::Skipped,
            CreateDeploymentStepOutcome::Skipped
        );
        assert_eq!(
            CreateDeploymentStepOutcome::Failure,
            CreateDeploymentStepOutcome::Failure
        );
        assert_ne!(
            CreateDeploymentStepOutcome::Success,
            CreateDeploymentStepOutcome::Failure
        );
        assert_ne!(
            CreateDeploymentStepOutcome::Success,
            CreateDeploymentStepOutcome::Skipped
        );
        assert_ne!(
            CreateDeploymentStepOutcome::Skipped,
            CreateDeploymentStepOutcome::Failure
        );
    }

    #[tokio::test]
    async fn test_await_receiver_pending() {
        let (mut sender, mut progress) = create_progress_pairs();

        // Create a task that will send after a delay
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            sender
                .set_pull_image_finished(CreateDeploymentStepOutcome::Success)
                .await;
        });

        // This should eventually complete
        let outcome = progress.wait_for_pull_image_outcome().await;
        assert!(outcome.is_ok());
        assert_eq!(outcome.unwrap(), CreateDeploymentStepOutcome::Success);
    }
}
