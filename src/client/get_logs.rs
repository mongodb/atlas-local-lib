use crate::{
    client::Client,
    docker::DockerLogContainer,
    models::{LogOutputStream, LogsOptions},
};

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
    /// * `options` - Optional logging options (e.g., follow, tail, timestamps, etc.)
    ///
    /// # Returns
    ///
    /// A [`LogOutputStream`] that yields log entries from the container.
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
    pub fn get_logs<'a>(
        &'a self,
        container_id_or_name: &'a str,
        options: Option<LogsOptions>,
    ) -> LogOutputStream<'a> {
        let bollard_options = options.map(bollard::query_parameters::LogsOptions::from);
        LogOutputStream::new(self.docker.logs(container_id_or_name, bollard_options))
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
        let logs: Vec<_> = client
            .get_logs("test-container", Some(options))
            .collect()
            .await;

        // Assert
        assert_eq!(logs.len(), 2);
        assert!(logs[0].is_ok());
        assert!(logs[1].is_ok());
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
        let logs: Vec<_> = client
            .get_logs("nonexistent-container", None)
            .collect()
            .await;

        // Assert
        assert_eq!(logs.len(), 1);
        assert!(logs[0].is_err());
        assert!(matches!(
            logs[0].as_ref().unwrap_err(),
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
        let logs: Vec<_> = client
            .get_logs("test-container", Some(options))
            .collect()
            .await;

        // Assert
        assert_eq!(logs.len(), 3);
        assert!(logs.iter().all(|log| log.is_ok()));

        // Verify first is stdout
        if let Ok(log) = &logs[0] {
            assert!(log.is_stdout());
        } else {
            panic!("Expected stdout log");
        }

        // Verify second is stderr
        if let Ok(log) = &logs[1] {
            assert!(log.is_stderr());
        } else {
            panic!("Expected stderr log");
        }

        // Verify third is stdout
        if let Ok(log) = &logs[2] {
            assert!(log.is_stdout());
        } else {
            panic!("Expected stdout log");
        }
    }
}
