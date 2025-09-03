use bollard::secret::ContainerInspectResponse;
use semver::Version;

use crate::models::{
    CreationSource, EnvironmentVariables, GetLocalDeploymentLabelsError,
    GetMongoDBPortBindingError, GetStateError, LocalDeploymentLabels, MongoDBPortBinding,
    MongodbType, State,
};

const LOCAL_SEED_LOCATION: &str = "/docker-entrypoint-initdb.d";

#[derive(Debug)]
pub struct Deployment {
    // Identifiers
    pub container_id: String,
    pub name: Option<String>,

    // Docker specific
    pub state: State,
    pub port_bindings: Option<MongoDBPortBinding>,

    // MongoDB details (MongoD)
    pub mongodb_type: MongodbType,
    pub mongodb_version: Version,

    // Creation source
    pub creation_source: Option<CreationSource>,

    // Initial database configuration
    pub local_seed_location: Option<String>,
    pub mongodb_initdb_database: Option<String>,
    pub mongodb_initdb_root_password_file: Option<String>,
    pub mongodb_initdb_root_password: Option<String>,
    pub mongodb_initdb_root_username_file: Option<String>,
    pub mongodb_initdb_root_username: Option<String>,

    // Logging
    pub mongot_log_file: Option<String>,
    pub runner_log_file: Option<String>,

    // Telemetry
    pub do_not_track: Option<String>,
    pub telemetry_base_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum IntoDeploymentError {
    #[error("Container ID is missing")]
    MissingContainerID,
    #[error(transparent)]
    LocalDeploymentLabels(#[from] GetLocalDeploymentLabelsError),
    #[error(transparent)]
    MongoDBPortBinding(#[from] GetMongoDBPortBindingError),
    #[error(transparent)]
    State(#[from] GetStateError),
}

impl TryFrom<ContainerInspectResponse> for Deployment {
    type Error = IntoDeploymentError;

    fn try_from(value: ContainerInspectResponse) -> Result<Self, Self::Error> {
        // Extract the container ID from the response
        let container_id = value
            .id
            .as_ref()
            .ok_or(IntoDeploymentError::MissingContainerID)?
            .clone();

        // Extract the container name as the deployment name
        // Docker names have a leading slash, so we remove it
        let name = value
            .name
            .as_ref()
            .and_then(|n| n.strip_prefix('/'))
            .map(|n| n.to_string());

        // Get container labels, environment variables, and local seed location from the container inspect response
        let container_labels = LocalDeploymentLabels::try_from(&value)?;
        let container_environment_variables = EnvironmentVariables::from(&value);
        let local_seed_location = extract_local_seed_location(&value);
        let port_bindings = MongoDBPortBinding::try_from(&value)?;
        let state = State::try_from(&value)?;

        // Deconstruct the labels and environment variables
        let LocalDeploymentLabels {
            mongodb_version,
            mongodb_type,
        } = container_labels;

        let EnvironmentVariables {
            tool,
            runner_log_file,
            mongodb_initdb_root_username,
            mongodb_initdb_root_username_file,
            mongodb_initdb_root_password,
            mongodb_initdb_root_password_file,
            mongodb_initdb_database,
            mongot_log_file,
            do_not_track,
            telemetry_base_url,
        } = container_environment_variables;

        Ok(Self {
            // Identifiers
            name,
            container_id,

            // Docker specific
            state,
            port_bindings,

            // MongoDB details (MongoD)
            mongodb_type,
            mongodb_version,

            // Creation source
            creation_source: tool,

            // Initial database configuration
            local_seed_location,
            mongodb_initdb_database,
            mongodb_initdb_root_password_file,
            mongodb_initdb_root_password,
            mongodb_initdb_root_username_file,
            mongodb_initdb_root_username,

            // Logging
            mongot_log_file,
            runner_log_file,

            // Telemetry
            do_not_track,
            telemetry_base_url,
        })
    }
}

fn extract_local_seed_location(
    container_inspect_response: &ContainerInspectResponse,
) -> Option<String> {
    // Go through the mounts and find the one that has the local seed location (mounted at /docker-entrypoint-initdb.d)
    let mount = container_inspect_response
        .mounts
        .as_ref()?
        .iter()
        .find(|m| m.destination.as_deref() == Some(LOCAL_SEED_LOCATION))?;

    // Return the source of the mount
    mount.source.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::secret::{
        ContainerConfig, ContainerState, ContainerStateStatusEnum, MountPoint, NetworkSettings,
        PortBinding,
    };
    use std::collections::HashMap;

    #[test]
    fn test_into_deployment() {
        // Create required labels for LocalDeploymentLabels
        let mut labels = HashMap::new();
        labels.insert("mongodb-atlas-local".to_string(), "container".to_string());
        labels.insert("version".to_string(), "8.0.0".to_string());
        labels.insert("mongodb-type".to_string(), "community".to_string());

        // Create environment variables
        let env_vars = vec![
            "TOOL=ATLASCLI".to_string(),
            "MONGODB_INITDB_ROOT_USERNAME=admin".to_string(),
            "MONGODB_INITDB_ROOT_USERNAME_FILE=/run/secrets/username".to_string(),
            "MONGODB_INITDB_ROOT_PASSWORD=password123".to_string(),
            "MONGODB_INITDB_ROOT_PASSWORD_FILE=/run/secrets/password".to_string(),
            "MONGODB_INITDB_DATABASE=testdb".to_string(),
            "RUNNER_LOG_FILE=/tmp/runner.log".to_string(),
            "MONGOT_LOG_FILE=/tmp/mongot.log".to_string(),
            "DO_NOT_TRACK=false".to_string(),
            "TELEMETRY_BASE_URL=https://telemetry.example.com".to_string(),
        ];

        // Create a mount for local seed location
        let mount = MountPoint {
            destination: Some("/docker-entrypoint-initdb.d".to_string()),
            source: Some("/host/seed-data".to_string()),
            ..Default::default()
        };

        // Create state for the container
        let container_state = ContainerState {
            status: Some(ContainerStateStatusEnum::RUNNING),
            ..Default::default()
        };

        // Create network settings with port bindings
        let port_binding = PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some("27017".to_string()),
        };
        let mut port_map = HashMap::new();
        port_map.insert("27017/tcp".to_string(), Some(vec![port_binding]));
        let network_settings = NetworkSettings {
            ports: Some(port_map),
            ..Default::default()
        };

        let container_inspect_response = ContainerInspectResponse {
            id: Some("container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                env: Some(env_vars),
                labels: Some(labels),
                ..Default::default()
            }),
            mounts: Some(vec![mount]),
            state: Some(container_state),
            network_settings: Some(network_settings),
            ..Default::default()
        };

        let deployment = Deployment::try_from(container_inspect_response).unwrap();

        // Test all the fields to ensure proper parsing
        assert_eq!(deployment.container_id, "container_id");
        assert_eq!(deployment.name, Some("test-deployment".to_string()));
        assert_eq!(deployment.state, State::Running);
        assert!(deployment.port_bindings.is_some());
        let port_binding = deployment.port_bindings.unwrap();
        assert_eq!(port_binding.port, 27017);
        assert_eq!(
            port_binding.binding_type,
            crate::models::BindingType::Loopback
        );
        assert_eq!(deployment.creation_source, Some(CreationSource::AtlasCLI));
        assert_eq!(deployment.mongodb_type, MongodbType::Community);
        assert_eq!(deployment.mongodb_version, Version::new(8, 0, 0));
        assert_eq!(
            deployment.local_seed_location,
            Some("/host/seed-data".to_string())
        );
        assert_eq!(
            deployment.mongodb_initdb_database,
            Some("testdb".to_string())
        );
        assert_eq!(
            deployment.mongodb_initdb_root_username,
            Some("admin".to_string())
        );
        assert_eq!(
            deployment.mongodb_initdb_root_username_file,
            Some("/run/secrets/username".to_string())
        );
        assert_eq!(
            deployment.mongodb_initdb_root_password,
            Some("password123".to_string())
        );
        assert_eq!(
            deployment.mongodb_initdb_root_password_file,
            Some("/run/secrets/password".to_string())
        );
        assert_eq!(
            deployment.runner_log_file,
            Some("/tmp/runner.log".to_string())
        );
        assert_eq!(
            deployment.mongot_log_file,
            Some("/tmp/mongot.log".to_string())
        );
        assert_eq!(deployment.do_not_track, Some("false".to_string()));
        assert_eq!(
            deployment.telemetry_base_url,
            Some("https://telemetry.example.com".to_string())
        );
    }

    #[test]
    fn test_extract_local_seed_location_no_mounts() {
        let container_inspect_response = ContainerInspectResponse {
            mounts: None,
            ..Default::default()
        };

        let result = extract_local_seed_location(&container_inspect_response);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_local_seed_location_empty_mounts() {
        let container_inspect_response = ContainerInspectResponse {
            mounts: Some(vec![]),
            ..Default::default()
        };

        let result = extract_local_seed_location(&container_inspect_response);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_local_seed_location_no_matching_mount() {
        let mount1 = MountPoint {
            destination: Some("/var/log".to_string()),
            source: Some("/host/logs".to_string()),
            ..Default::default()
        };
        let mount2 = MountPoint {
            destination: Some("/app/data".to_string()),
            source: Some("/host/data".to_string()),
            ..Default::default()
        };

        let container_inspect_response = ContainerInspectResponse {
            mounts: Some(vec![mount1, mount2]),
            ..Default::default()
        };

        let result = extract_local_seed_location(&container_inspect_response);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_local_seed_location_matching_mount() {
        let mount = MountPoint {
            destination: Some(LOCAL_SEED_LOCATION.to_string()),
            source: Some("/host/seed-data".to_string()),
            ..Default::default()
        };

        let container_inspect_response = ContainerInspectResponse {
            mounts: Some(vec![mount]),
            ..Default::default()
        };

        let result = extract_local_seed_location(&container_inspect_response);
        assert_eq!(result, Some("/host/seed-data".to_string()));
    }
}
