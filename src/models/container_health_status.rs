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
