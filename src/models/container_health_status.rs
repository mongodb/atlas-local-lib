use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContainerHealthStatus {
    Empty,
    Healthy,
    Unhealthy,
    None,
    Starting,
}

impl fmt::Display for ContainerHealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerHealthStatus::Empty => write!(f, "empty"),
            ContainerHealthStatus::Healthy => write!(f, "healthy"),
            ContainerHealthStatus::Unhealthy => write!(f, "unhealthy"),
            ContainerHealthStatus::None => write!(f, "none"),
            ContainerHealthStatus::Starting => write!(f, "starting"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(ContainerHealthStatus::Empty.to_string(), "empty");
        assert_eq!(ContainerHealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(ContainerHealthStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(ContainerHealthStatus::None.to_string(), "none");
        assert_eq!(ContainerHealthStatus::Starting.to_string(), "starting");
    }
}
