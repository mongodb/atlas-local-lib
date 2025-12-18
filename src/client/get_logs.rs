use crate::{
    client::Client,
    docker::DockerLogContainer,
    models::{LogOutput, LogsOptions},
};
use futures_util::{pin_mut, StreamExt};

#[derive(Debug, thiserror::Error)]
pub enum GetLogsError {
    #[error("Failed to get container logs: {0}")]
    ContainerLogs(#[from] bollard::errors::Error),
}

impl<D: DockerLogContainer> Client<D> {
    /// Gets the logs from a container.
    ///
    /// # Arguments
    ///
    /// * `container_id_or_name` - The ID or name of the container to get logs from.
    /// * `options` - Optional logging options (e.g., tail, timestamps, etc.)
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of log entries from the container, or an error if the logs could not be retrieved.
    ///
    /// # Examples
    ///
    /// See the complete working example:
    ///
    /// ```sh
    /// cargo run --example get_logs
    /// ```
    ///
    #[doc = "Example code:\n\n"]
    #[doc = "```rust,no_run"]
    #[doc = include_str!("../../examples/get_logs.rs")]
    #[doc = "```"]
    pub async fn get_logs(
        &self,
        container_id_or_name: &str,
        options: Option<LogsOptions>,
    ) -> Result<Vec<LogOutput>, GetLogsError> {
        let bollard_options = options.map(bollard::query_parameters::LogsOptions::from);
        let stream = self.docker.logs(container_id_or_name, bollard_options);
        pin_mut!(stream);
        
        let mut logs = Vec::new();
        while let Some(result) = stream.next().await {
            let log_output = result.map_err(GetLogsError::ContainerLogs)?;
            logs.push(LogOutput::from(log_output));
        }
        
        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LogsOptions;
    use bollard::errors::Error as BollardError;
    use futures_util::{Stream, StreamExt, stream};
    use mockall::mock;

    mock! {
        Docker {}

        impl DockerLogContainer for Docker {
            fn logs<'a>(
                &'a self,
                container_id: &str,
                options: Option<bollard::query_parameters::LogsOptions>,
            ) -> impl Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>>;
        }
    }

    #[tokio::test]
    async fn test_get_logs_success() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_logs()
            .withf(|container_id, options| container_id == "test-container" && options.is_some())
            .times(1)
            .returning(|_, _| {
                Box::pin(stream::iter(vec![
                    Ok(bollard::container::LogOutput::StdOut {
                        message: "Log line 1\n".into(),
                    }),
                    Ok(bollard::container::LogOutput::StdOut {
                        message: "Log line 2\n".into(),
                    }),
                ]))
            });

        let client = Client::new(mock_docker);
        let options = LogsOptions::builder().stdout(true).stderr(true).build();

        // Act
        let logs = client
            .get_logs("test-container", Some(options))
            .await
            .expect("get_logs should succeed");

        // Assert
        assert_eq!(logs.len(), 2);
        assert!(logs[0].is_stdout());
        assert!(logs[1].is_stdout());
    }

    #[tokio::test]
    async fn test_get_logs_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_logs()
            .withf(|container_id, options| {
                container_id == "nonexistent-container" && options.is_none()
            })
            .times(1)
            .returning(|_, _| {
                Box::pin(stream::iter(vec![Err(
                    BollardError::DockerResponseServerError {
                        status_code: 404,
                        message: "No such container".to_string(),
                    },
                )]))
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .get_logs("nonexistent-container", None)
            .await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.as_ref().unwrap_err(),
            GetLogsError::ContainerLogs(_)
        ));
    }

    #[tokio::test]
    async fn test_get_logs_mixed_stdout_stderr() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_logs()
            .withf(|container_id, _| container_id == "test-container")
            .times(1)
            .returning(|_, _| {
                Box::pin(stream::iter(vec![
                    Ok(bollard::container::LogOutput::StdOut {
                        message: "stdout line\n".into(),
                    }),
                    Ok(bollard::container::LogOutput::StdErr {
                        message: "stderr line\n".into(),
                    }),
                    Ok(bollard::container::LogOutput::StdOut {
                        message: "another stdout line\n".into(),
                    }),
                ]))
            });

        let client = Client::new(mock_docker);
        let options = LogsOptions::builder().stdout(true).stderr(true).build();

        // Act
        let logs = client
            .get_logs("test-container", Some(options))
            .await
            .expect("get_logs should succeed");

        // Assert
        assert_eq!(logs.len(), 3);

        // Verify first is stdout
        assert!(logs[0].is_stdout());

        // Verify second is stderr
        assert!(logs[1].is_stderr());

        // Verify third is stdout
        assert!(logs[2].is_stdout());
    }
}
