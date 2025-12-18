use futures_util::Stream;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::LogOutput;
use crate::GetLogsError;

/// A stream of log output from a container.
///
/// This type wraps the underlying stream of log entries and provides
/// automatic conversion from bollard's `LogOutput` to our internal
/// `LogOutput` type.
#[pin_project]
pub struct LogOutputStream<'a> {
    #[pin]
    inner: Pin<
        Box<dyn Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>> + 'a>,
    >,
}

impl<'a> LogOutputStream<'a> {
    /// Creates a new `LogOutputStream` wrapping the given stream.
    pub fn new(
        inner: impl Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>> + 'a,
    ) -> Self {
        Self {
            inner: Box::pin(inner),
        }
    }
}

impl<'a> Stream for LogOutputStream<'a> {
    type Item = Result<LogOutput, GetLogsError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx).map(|option| {
            option.map(|result| {
                result
                    .map(LogOutput::from)
                    .map_err(GetLogsError::ContainerLogs)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures_util::{StreamExt, stream};

    #[tokio::test]
    async fn test_log_output_stream_success() {
        let bollard_stream = stream::iter(vec![
            Ok(bollard::container::LogOutput::StdOut {
                message: Bytes::from("line 1\n"),
            }),
            Ok(bollard::container::LogOutput::StdErr {
                message: Bytes::from("error line\n"),
            }),
        ]);

        let mut log_stream = LogOutputStream::new(bollard_stream);

        // First item
        let item = log_stream.next().await.unwrap().unwrap();
        assert!(item.is_stdout());
        assert_eq!(item.as_str_lossy(), "line 1\n");

        // Second item
        let item = log_stream.next().await.unwrap().unwrap();
        assert!(item.is_stderr());
        assert_eq!(item.as_str_lossy(), "error line\n");

        // Stream ends
        assert!(log_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_log_output_stream_error() {
        let bollard_stream = stream::iter(vec![
            Ok(bollard::container::LogOutput::StdOut {
                message: Bytes::from("line 1\n"),
            }),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 500,
                message: "Internal error".to_string(),
            }),
        ]);

        let mut log_stream = LogOutputStream::new(bollard_stream);

        // First item succeeds
        assert!(log_stream.next().await.unwrap().is_ok());

        // Second item is error
        let result = log_stream.next().await.unwrap();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetLogsError::ContainerLogs(_)
        ));
    }
}
