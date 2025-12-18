use bytes::Bytes;

/// Represents a single log output entry from a container.
///
/// Container logs can come from different streams (stdout, stderr, stdin)
/// and this enum represents which stream the log entry came from along
/// with its content.
///
/// # Accessing Log Content
///
/// You can access the log content via pattern matching or helper methods:
///
/// ```rust
/// use atlas_local::models::LogOutput;
/// use bytes::Bytes;
///
/// let log = LogOutput::StdOut {
///     message: Bytes::from("Hello, world!\n"),
/// };
///
/// // Pattern matching
/// match log {
///     LogOutput::StdOut { message } => {
///         println!("stdout: {:?}", message);
///     }
///     LogOutput::StdErr { message } => {
///         println!("stderr: {:?}", message);
///     }
///     _ => {}
/// }
///
/// // Or use helper methods
/// let log = LogOutput::StdOut {
///     message: Bytes::from("Hello, world!\n"),
/// };
/// println!("Message: {}", log.as_str_lossy());
/// println!("Bytes: {:?}", log.as_bytes());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogOutput {
    /// Standard output log entry
    StdOut {
        /// The log message content
        message: Bytes,
    },
    /// Standard error log entry
    StdErr {
        /// The log message content
        message: Bytes,
    },
    /// Standard input log entry
    StdIn {
        /// The log message content
        message: Bytes,
    },
    /// Console log entry
    Console {
        /// The log message content
        message: Bytes,
    },
}

impl LogOutput {
    /// Returns the message content as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            LogOutput::StdOut { message } => message.as_ref(),
            LogOutput::StdErr { message } => message.as_ref(),
            LogOutput::StdIn { message } => message.as_ref(),
            LogOutput::Console { message } => message.as_ref(),
        }
    }

    /// Returns the message content as a UTF-8 string, replacing invalid sequences.
    pub fn as_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }

    /// Returns true if this is a stdout log entry.
    pub fn is_stdout(&self) -> bool {
        matches!(self, LogOutput::StdOut { .. })
    }

    /// Returns true if this is a stderr log entry.
    pub fn is_stderr(&self) -> bool {
        matches!(self, LogOutput::StdErr { .. })
    }

    /// Returns true if this is a stdin log entry.
    pub fn is_stdin(&self) -> bool {
        matches!(self, LogOutput::StdIn { .. })
    }

    /// Returns true if this is a console log entry.
    pub fn is_console(&self) -> bool {
        matches!(self, LogOutput::Console { .. })
    }
}

impl From<bollard::container::LogOutput> for LogOutput {
    fn from(output: bollard::container::LogOutput) -> Self {
        match output {
            bollard::container::LogOutput::StdOut { message } => LogOutput::StdOut { message },
            bollard::container::LogOutput::StdErr { message } => LogOutput::StdErr { message },
            bollard::container::LogOutput::StdIn { message } => LogOutput::StdIn { message },
            bollard::container::LogOutput::Console { message } => LogOutput::Console { message },
        }
    }
}

impl std::fmt::Display for LogOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_output_stdout() {
        let output = LogOutput::StdOut {
            message: Bytes::from("test message\n"),
        };

        assert!(output.is_stdout());
        assert!(!output.is_stderr());
        assert!(!output.is_stdin());
        assert!(!output.is_console());
        assert_eq!(output.as_bytes(), b"test message\n");
        assert_eq!(output.as_str_lossy(), "test message\n");
    }

    #[test]
    fn test_log_output_stderr() {
        let output = LogOutput::StdErr {
            message: Bytes::from("error message\n"),
        };

        assert!(!output.is_stdout());
        assert!(output.is_stderr());
        assert!(!output.is_stdin());
        assert!(!output.is_console());
        assert_eq!(output.as_bytes(), b"error message\n");
        assert_eq!(output.as_str_lossy(), "error message\n");
    }

    #[test]
    fn test_log_output_stdin() {
        let output = LogOutput::StdIn {
            message: Bytes::from("input message\n"),
        };

        assert!(!output.is_stdout());
        assert!(!output.is_stderr());
        assert!(output.is_stdin());
        assert!(!output.is_console());
    }

    #[test]
    fn test_log_output_console() {
        let output = LogOutput::Console {
            message: Bytes::from("console message\n"),
        };

        assert!(!output.is_stdout());
        assert!(!output.is_stderr());
        assert!(!output.is_stdin());
        assert!(output.is_console());
    }

    #[test]
    fn test_log_output_display() {
        let output = LogOutput::StdOut {
            message: Bytes::from("display test\n"),
        };

        assert_eq!(format!("{}", output), "display test\n");
    }

    #[test]
    fn test_log_output_from_bollard() {
        let bollard_output = bollard::container::LogOutput::StdOut {
            message: Bytes::from("test\n"),
        };

        let output: LogOutput = bollard_output.into();
        assert!(output.is_stdout());
        assert_eq!(output.as_str_lossy(), "test\n");
    }

    #[test]
    fn test_log_output_lossy_conversion() {
        // Test with invalid UTF-8
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let output = LogOutput::StdOut {
            message: Bytes::from(invalid_utf8),
        };

        // Should not panic, should use replacement characters
        let lossy = output.as_str_lossy();
        assert!(!lossy.is_empty());
    }
}
