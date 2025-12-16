use std::{fmt::Display, str::FromStr};

use bollard::secret::{ContainerInspectResponse, ContainerStateStatusEnum};

/// The state of the container (from the Docker API)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Created,
    Dead,
    Exited,
    Paused,
    Removing,
    Restarting,
    Running,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum GetStateError {
    #[error("Missing state")]
    MissingState,
    #[error(transparent)]
    FromContainerStateStatusEnum(#[from] FromContainerStateStatusEnumError),
}

impl TryFrom<&ContainerInspectResponse> for State {
    type Error = GetStateError;

    fn try_from(value: &ContainerInspectResponse) -> Result<Self, Self::Error> {
        let state = &value
            .state
            .as_ref()
            .and_then(|s| s.status)
            .ok_or(GetStateError::MissingState)?;
        Ok(state.try_into()?)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FromContainerStateStatusEnumError {
    #[error("Empty state")]
    EmptyState,
}

impl TryFrom<&ContainerStateStatusEnum> for State {
    type Error = FromContainerStateStatusEnumError;

    fn try_from(value: &ContainerStateStatusEnum) -> Result<Self, Self::Error> {
        Ok(match value {
            ContainerStateStatusEnum::CREATED => State::Created,
            ContainerStateStatusEnum::DEAD => State::Dead,
            ContainerStateStatusEnum::EXITED => State::Exited,
            ContainerStateStatusEnum::PAUSED => State::Paused,
            ContainerStateStatusEnum::REMOVING => State::Removing,
            ContainerStateStatusEnum::RESTARTING => State::Restarting,
            ContainerStateStatusEnum::RUNNING => State::Running,

            ContainerStateStatusEnum::EMPTY => {
                return Err(FromContainerStateStatusEnumError::EmptyState);
            }
        })
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Created => write!(f, "created"),
            State::Dead => write!(f, "dead"),
            State::Exited => write!(f, "exited"),
            State::Paused => write!(f, "paused"),
            State::Removing => write!(f, "removing"),
            State::Restarting => write!(f, "restarting"),
            State::Running => write!(f, "running"),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FromStrStateError {
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

impl FromStr for State {
    type Err = FromStrStateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "created" => Ok(State::Created),
            "dead" => Ok(State::Dead),
            "exited" => Ok(State::Exited),
            "paused" => Ok(State::Paused),
            "removing" => Ok(State::Removing),
            "restarting" => Ok(State::Restarting),
            "running" => Ok(State::Running),
            _ => Err(FromStrStateError::InvalidState(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_container_state_status_enum_success() {
        // Test all successful conversions
        assert_eq!(
            State::try_from(&ContainerStateStatusEnum::CREATED).unwrap(),
            State::Created
        );
        assert_eq!(
            State::try_from(&ContainerStateStatusEnum::DEAD).unwrap(),
            State::Dead
        );
        assert_eq!(
            State::try_from(&ContainerStateStatusEnum::EXITED).unwrap(),
            State::Exited
        );
        assert_eq!(
            State::try_from(&ContainerStateStatusEnum::PAUSED).unwrap(),
            State::Paused
        );
        assert_eq!(
            State::try_from(&ContainerStateStatusEnum::REMOVING).unwrap(),
            State::Removing
        );
        assert_eq!(
            State::try_from(&ContainerStateStatusEnum::RESTARTING).unwrap(),
            State::Restarting
        );
        assert_eq!(
            State::try_from(&ContainerStateStatusEnum::RUNNING).unwrap(),
            State::Running
        );
    }

    #[test]
    fn test_try_from_container_state_status_enum_empty_error() {
        // Test error case for EMPTY status
        let result = State::try_from(&ContainerStateStatusEnum::EMPTY);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FromContainerStateStatusEnumError::EmptyState
        ));
    }

    #[test]
    fn test_try_from_container_inspect_response_success() {
        use bollard::secret::ContainerState;

        // Test successful conversion with a valid state
        let container_state = ContainerState {
            status: Some(ContainerStateStatusEnum::RUNNING),
            ..Default::default()
        };
        let container_inspect = ContainerInspectResponse {
            state: Some(container_state),
            ..Default::default()
        };

        let result = State::try_from(&container_inspect);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), State::Running);
    }

    #[test]
    fn test_try_from_container_inspect_response_missing_state() {
        // Test error case when state is None
        let container_inspect = ContainerInspectResponse::default();

        let result = State::try_from(&container_inspect);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GetStateError::MissingState));
    }

    #[test]
    fn test_try_from_container_inspect_response_missing_status() {
        use bollard::secret::ContainerState;

        // Test error case when state exists but status is None
        let container_inspect = ContainerInspectResponse {
            state: Some(ContainerState::default()), // status is None by default
            ..Default::default()
        };

        let result = State::try_from(&container_inspect);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GetStateError::MissingState));
    }

    #[test]
    fn test_try_from_container_inspect_response_empty_status() {
        use bollard::secret::ContainerState;

        // Test error case when status is EMPTY
        let container_state = ContainerState {
            status: Some(ContainerStateStatusEnum::EMPTY),
            ..Default::default()
        };
        let container_inspect = ContainerInspectResponse {
            state: Some(container_state),
            ..Default::default()
        };

        let result = State::try_from(&container_inspect);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetStateError::FromContainerStateStatusEnum(_)
        ));
    }

    #[test]
    fn test_all_states_have_corresponding_enum_values() {
        // Ensure all State variants can be created from ContainerStateStatusEnum
        let test_cases = [
            (ContainerStateStatusEnum::CREATED, State::Created),
            (ContainerStateStatusEnum::DEAD, State::Dead),
            (ContainerStateStatusEnum::EXITED, State::Exited),
            (ContainerStateStatusEnum::PAUSED, State::Paused),
            (ContainerStateStatusEnum::REMOVING, State::Removing),
            (ContainerStateStatusEnum::RESTARTING, State::Restarting),
            (ContainerStateStatusEnum::RUNNING, State::Running),
        ];

        for (enum_value, expected_state) in test_cases {
            let result = State::try_from(&enum_value);
            assert!(result.is_ok(), "Failed to convert {:?}", enum_value);
            assert_eq!(result.unwrap(), expected_state);
        }
    }

    #[test]
    fn test_state_debug_and_clone() {
        // Test that State implements Debug and Clone
        let state = State::Running;
        let cloned_state = state;
        assert_eq!(state, cloned_state);

        // Test Debug implementation
        let debug_output = format!("{:?}", state);
        assert_eq!(debug_output, "Running");
    }

    #[test]
    fn test_error_types_debug() {
        // Test Debug implementation for error types
        let get_state_error = GetStateError::MissingState;
        let debug_output = format!("{:?}", get_state_error);
        assert!(debug_output.contains("MissingState"));

        let from_enum_error = FromContainerStateStatusEnumError::EmptyState;
        let debug_output = format!("{:?}", from_enum_error);
        assert!(debug_output.contains("EmptyState"));
    }

    #[test]
    fn test_display_all_states() {
        // Test Display implementation for all State variants
        assert_eq!(State::Created.to_string(), "created");
        assert_eq!(State::Dead.to_string(), "dead");
        assert_eq!(State::Exited.to_string(), "exited");
        assert_eq!(State::Paused.to_string(), "paused");
        assert_eq!(State::Removing.to_string(), "removing");
        assert_eq!(State::Restarting.to_string(), "restarting");
        assert_eq!(State::Running.to_string(), "running");
    }

    #[test]
    fn test_from_str_success() {
        // Test FromStr for all valid states
        assert_eq!("created".parse::<State>().unwrap(), State::Created);
        assert_eq!("dead".parse::<State>().unwrap(), State::Dead);
        assert_eq!("exited".parse::<State>().unwrap(), State::Exited);
        assert_eq!("paused".parse::<State>().unwrap(), State::Paused);
        assert_eq!("removing".parse::<State>().unwrap(), State::Removing);
        assert_eq!("restarting".parse::<State>().unwrap(), State::Restarting);
        assert_eq!("running".parse::<State>().unwrap(), State::Running);
    }

    #[test]
    fn test_from_str_invalid() {
        // Test FromStr error branch
        let result = "invalid".parse::<State>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FromStrStateError::InvalidState("invalid".to_string())
        );
    }
}
