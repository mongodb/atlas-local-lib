use chrono::{DateTime, Utc};

/// Specifies how many lines to retrieve from the tail of the logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tail {
    /// Return all log lines
    All,
    /// Return a specific number of lines from the end
    Number(u64),
}

/// Error type for parsing `Tail` from a string.
#[derive(Debug, thiserror::Error)]
pub enum TailParseError {
    /// The string is not a valid tail value (must be "all" or a positive number)
    #[error("Invalid tail value: '{0}'. Expected 'all' or a positive number")]
    InvalidValue(String),
}

impl From<u64> for Tail {
    fn from(n: u64) -> Self {
        Tail::Number(n)
    }
}

impl TryFrom<&str> for Tail {
    type Error = TailParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "all" => Ok(Tail::All),
            _ => s
                .parse::<u64>()
                .map(Tail::Number)
                .map_err(|_| TailParseError::InvalidValue(s.to_string())),
        }
    }
}

impl TryFrom<String> for Tail {
    type Error = TailParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.as_str().try_into()
    }
}

impl std::fmt::Display for Tail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tail::All => write!(f, "all"),
            Tail::Number(n) => write!(f, "{}", n),
        }
    }
}

/// Options for retrieving logs from a container.
///
/// This struct provides configuration options for fetching container logs,
/// including filtering by stream type (stdout/stderr), limiting the number
/// of lines, and adding timestamps.
///
/// # Examples
///
/// ```
/// use atlas_local::models::{LogsOptions, Tail};
///
/// let options = LogsOptions::builder()
///     .stdout(true)
///     .stderr(true)
///     .tail(Tail::Number(100))
///     .timestamps(true)
///     .build();
/// ```
#[derive(Debug, Clone, Default, PartialEq, typed_builder::TypedBuilder)]
#[builder(doc)]
pub struct LogsOptions {
    /// Return logs from stdout
    #[builder(default = false)]
    pub stdout: bool,
    /// Return logs from stderr
    #[builder(default = false)]
    pub stderr: bool,
    /// Return logs from the given timestamp
    #[builder(default, setter(strip_option))]
    pub since: Option<DateTime<Utc>>,
    /// Return logs before the given timestamp
    #[builder(default, setter(strip_option))]
    pub until: Option<DateTime<Utc>>,
    /// Add timestamps to every log line
    #[builder(default = false)]
    pub timestamps: bool,
    /// Return this number of lines at the tail of the logs
    #[builder(default, setter(strip_option, into))]
    pub tail: Option<Tail>,
}

impl From<LogsOptions> for bollard::query_parameters::LogsOptions {
    fn from(options: LogsOptions) -> Self {
        bollard::query_parameters::LogsOptions {
            follow: false,
            stdout: options.stdout,
            stderr: options.stderr,
            since: options.since.map(|t| t.timestamp() as i32).unwrap_or(0),
            until: options.until.map(|t| t.timestamp() as i32).unwrap_or(0),
            timestamps: options.timestamps,
            tail: options.tail.map(|t| t.to_string()).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logs_options_into_bollard() {
        let options = LogsOptions::builder()
            .stdout(true)
            .stderr(true)
            .since(DateTime::from_timestamp(1234567890, 0).unwrap())
            .until(DateTime::from_timestamp(1234567900, 0).unwrap())
            .timestamps(true)
            .tail(Tail::Number(100))
            .build();

        let bollard_options: bollard::query_parameters::LogsOptions = options.into();

        assert!(bollard_options.stdout);
        assert!(bollard_options.stderr);
        assert_eq!(bollard_options.since, 1234567890);
        assert_eq!(bollard_options.until, 1234567900);
        assert!(bollard_options.timestamps);
        assert_eq!(bollard_options.tail, "100");
        assert!(!bollard_options.follow);
    }

    #[test]
    fn test_tail_display() {
        assert_eq!(Tail::All.to_string(), "all");
        assert_eq!(Tail::Number(100).to_string(), "100");
        assert_eq!(Tail::Number(0).to_string(), "0");
    }

    #[test]
    fn test_tail_try_from_str() {
        // Valid values
        assert_eq!(Tail::try_from("all").unwrap(), Tail::All);
        assert_eq!(Tail::try_from("100").unwrap(), Tail::Number(100));
        assert_eq!(Tail::try_from("0").unwrap(), Tail::Number(0));

        // Test invalid values
        let result = Tail::try_from("ALL");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TailParseError::InvalidValue(_)
        ));

        let result = Tail::try_from("invalid");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TailParseError::InvalidValue(_)
        ));

        let result = Tail::try_from("-1");
        assert!(result.is_err());

        let result = Tail::try_from("");
        assert!(result.is_err());
    }

    #[test]
    fn test_tail_try_from_string() {
        assert_eq!(Tail::try_from("all".to_string()).unwrap(), Tail::All);
        assert_eq!(
            Tail::try_from("100".to_string()).unwrap(),
            Tail::Number(100)
        );

        let result = Tail::try_from("invalid".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_tail_from_u64() {
        assert_eq!(Tail::from(100u64), Tail::Number(100));
        assert_eq!(Tail::from(0u64), Tail::Number(0));
    }

    #[test]
    fn test_logs_options_into_bollard_with_tail_all() {
        let options = LogsOptions {
            stdout: true,
            stderr: false,
            since: None,
            until: None,
            timestamps: false,
            tail: Some(Tail::All),
        };

        let bollard_options: bollard::query_parameters::LogsOptions = options.into();
        assert_eq!(bollard_options.tail, "all");
    }

    #[test]
    fn test_logs_options_builder_with_u64_tail() {
        // Test that builder accepts u64 directly
        let options = LogsOptions::builder().stdout(true).tail(100u64).build();

        assert_eq!(options.tail, Some(Tail::Number(100)));
    }

    #[test]
    fn test_tail_parse_error_display() {
        let err = TailParseError::InvalidValue("bad_value".to_string());
        let error_msg = err.to_string();
        assert!(error_msg.contains("Invalid tail value"));
        assert!(error_msg.contains("bad_value"));
    }
}
