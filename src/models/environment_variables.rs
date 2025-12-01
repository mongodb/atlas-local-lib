use std::collections::HashMap;

use bollard::secret::ContainerInspectResponse;

use crate::models::CreationSource;

pub const ENV_VAR_TOOL: &str = "TOOL";
pub const ENV_VAR_RUNNER_LOG_FILE: &str = "RUNNER_LOG_FILE";
pub const ENV_VAR_MONGODB_INITDB_ROOT_USERNAME: &str = "MONGODB_INITDB_ROOT_USERNAME";
pub const ENV_VAR_MONGODB_INITDB_ROOT_USERNAME_FILE: &str = "MONGODB_INITDB_ROOT_USERNAME_FILE";
pub const ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD: &str = "MONGODB_INITDB_ROOT_PASSWORD";
pub const ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD_FILE: &str = "MONGODB_INITDB_ROOT_PASSWORD_FILE";
pub const ENV_VAR_MONGODB_INITDB_DATABASE: &str = "MONGODB_INITDB_DATABASE";
pub const ENV_VAR_MONGOT_LOG_FILE: &str = "MONGOT_LOG_FILE";
pub const ENV_VAR_DO_NOT_TRACK: &str = "DO_NOT_TRACK";
pub const ENV_VAR_TELEMETRY_BASE_URL: &str = "TELEMETRY_BASE_URL";
pub const ENV_VAR_MONGODB_LOAD_SAMPLE_DATA: &str = "MONGODB_LOAD_SAMPLE_DATA";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct EnvironmentVariables {
    pub tool: Option<CreationSource>,

    pub runner_log_file: Option<String>,

    pub mongodb_initdb_root_username: Option<String>,
    pub mongodb_initdb_root_username_file: Option<String>,

    pub mongodb_initdb_root_password: Option<String>,
    pub mongodb_initdb_root_password_file: Option<String>,

    pub mongodb_initdb_database: Option<String>,

    pub mongot_log_file: Option<String>,

    pub do_not_track: Option<String>,

    pub telemetry_base_url: Option<String>,

    pub mongodb_load_sample_data: Option<String>,
}

impl From<&ContainerInspectResponse> for EnvironmentVariables {
    fn from(value: &ContainerInspectResponse) -> Self {
        // Create a default environment variables
        let mut environment_variables = EnvironmentVariables::default();

        // Get the environment variables from the container
        // If nothing is found, return the default environment variables (which is a struct with all the fields set to None)
        let Some(Some(container_environment_variables_vec)) = value.config.as_ref().map(|c| &c.env)
        else {
            return environment_variables;
        };

        // Convert the vector of strings to a hash map
        // The container environment variables is a vector of strings
        let env = container_environment_variables_vec
            .iter()
            .filter_map(|e| e.split_once("="))
            .collect::<HashMap<&str, &str>>();

        // Extract the environment variables from the hash map
        environment_variables.tool = get_value(&env, ENV_VAR_TOOL);
        environment_variables.runner_log_file = get_value(&env, ENV_VAR_RUNNER_LOG_FILE);
        environment_variables.mongodb_initdb_root_username =
            get_value(&env, ENV_VAR_MONGODB_INITDB_ROOT_USERNAME);
        environment_variables.mongodb_initdb_root_username_file =
            get_value(&env, ENV_VAR_MONGODB_INITDB_ROOT_USERNAME_FILE);
        environment_variables.mongodb_initdb_root_password =
            get_value(&env, ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD);
        environment_variables.mongodb_initdb_root_password_file =
            get_value(&env, ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD_FILE);
        environment_variables.mongodb_initdb_database =
            get_value(&env, ENV_VAR_MONGODB_INITDB_DATABASE);
        environment_variables.mongot_log_file = get_value(&env, ENV_VAR_MONGOT_LOG_FILE);
        environment_variables.do_not_track = get_value(&env, ENV_VAR_DO_NOT_TRACK);
        environment_variables.telemetry_base_url = get_value(&env, ENV_VAR_TELEMETRY_BASE_URL);
        environment_variables.mongodb_load_sample_data =
            get_value(&env, ENV_VAR_MONGODB_LOAD_SAMPLE_DATA);

        environment_variables
    }
}

fn get_value<T>(hash_map: &HashMap<&str, &str>, key: &str) -> Option<T>
where
    T: for<'a> From<&'a str>,
{
    hash_map.get(key).map(|v| T::from(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::secret::{ContainerConfig, ContainerInspectResponse};

    #[test]
    fn test_from_container_inspect_response_no_config() {
        // Test case: inspection response does not contain a config
        let container_response = ContainerInspectResponse {
            config: None,
            ..Default::default()
        };

        let env_vars = EnvironmentVariables::from(&container_response);

        // All fields should be None (default values)
        assert_eq!(env_vars.tool, None);
        assert_eq!(env_vars.runner_log_file, None);
        assert_eq!(env_vars.mongodb_initdb_root_username, None);
        assert_eq!(env_vars.mongodb_initdb_root_username_file, None);
        assert_eq!(env_vars.mongodb_initdb_root_password, None);
        assert_eq!(env_vars.mongodb_initdb_root_password_file, None);
        assert_eq!(env_vars.mongodb_initdb_database, None);
        assert_eq!(env_vars.mongot_log_file, None);
        assert_eq!(env_vars.do_not_track, None);
        assert_eq!(env_vars.telemetry_base_url, None);
        assert_eq!(env_vars.mongodb_load_sample_data, None);
    }

    #[test]
    fn test_from_container_inspect_response_no_env_variables() {
        // Test case: inspection response does not contain env variables on the config
        let container_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                env: None,
                ..Default::default()
            }),
            ..Default::default()
        };

        let env_vars = EnvironmentVariables::from(&container_response);

        // All fields should be None (default values)
        assert_eq!(env_vars.tool, None);
        assert_eq!(env_vars.runner_log_file, None);
        assert_eq!(env_vars.mongodb_initdb_root_username, None);
        assert_eq!(env_vars.mongodb_initdb_root_username_file, None);
        assert_eq!(env_vars.mongodb_initdb_root_password, None);
        assert_eq!(env_vars.mongodb_initdb_root_password_file, None);
        assert_eq!(env_vars.mongodb_initdb_database, None);
        assert_eq!(env_vars.mongot_log_file, None);
        assert_eq!(env_vars.do_not_track, None);
        assert_eq!(env_vars.telemetry_base_url, None);
        assert_eq!(env_vars.mongodb_load_sample_data, None);
    }

    #[test]
    fn test_from_container_inspect_response_with_all_env_variables() {
        // Test case: inspection response contains every single possible env variable
        let env_variables = vec![
            format!("{}=ATLASCLI", ENV_VAR_TOOL),
            format!("{}=/tmp/runner.log", ENV_VAR_RUNNER_LOG_FILE),
            format!("{}=admin", ENV_VAR_MONGODB_INITDB_ROOT_USERNAME),
            format!(
                "{}=/run/secrets/username",
                ENV_VAR_MONGODB_INITDB_ROOT_USERNAME_FILE
            ),
            format!("{}=password123", ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD),
            format!(
                "{}=/run/secrets/password",
                ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD_FILE
            ),
            format!("{}=testdb", ENV_VAR_MONGODB_INITDB_DATABASE),
            format!("{}=/tmp/mongot.log", ENV_VAR_MONGOT_LOG_FILE),
            format!("{}=true", ENV_VAR_DO_NOT_TRACK),
            format!(
                "{}=https://telemetry.example.com",
                ENV_VAR_TELEMETRY_BASE_URL
            ),
            format!("{}=true", ENV_VAR_MONGODB_LOAD_SAMPLE_DATA),
        ];

        let container_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                env: Some(env_variables),
                ..Default::default()
            }),
            ..Default::default()
        };

        let env_vars = EnvironmentVariables::from(&container_response);

        // Verify all fields are properly set
        assert_eq!(env_vars.tool, Some(CreationSource::AtlasCLI));
        assert_eq!(
            env_vars.runner_log_file,
            Some("/tmp/runner.log".to_string())
        );
        assert_eq!(
            env_vars.mongodb_initdb_root_username,
            Some("admin".to_string())
        );
        assert_eq!(
            env_vars.mongodb_initdb_root_username_file,
            Some("/run/secrets/username".to_string())
        );
        assert_eq!(
            env_vars.mongodb_initdb_root_password,
            Some("password123".to_string())
        );
        assert_eq!(
            env_vars.mongodb_initdb_root_password_file,
            Some("/run/secrets/password".to_string())
        );
        assert_eq!(env_vars.mongodb_initdb_database, Some("testdb".to_string()));
        assert_eq!(
            env_vars.mongot_log_file,
            Some("/tmp/mongot.log".to_string())
        );
        assert_eq!(env_vars.do_not_track, Some("true".to_string()));
        assert_eq!(
            env_vars.telemetry_base_url,
            Some("https://telemetry.example.com".to_string())
        );
        assert_eq!(env_vars.mongodb_load_sample_data, Some("true".to_string()));
    }

    #[test]
    fn test_from_container_inspect_response_partial_env_variables() {
        // Test case: inspection response contains only some env variables
        let env_variables = vec![
            format!("{}=CONTAINER", ENV_VAR_TOOL),
            format!("{}=testuser", ENV_VAR_MONGODB_INITDB_ROOT_USERNAME),
            format!("{}=false", ENV_VAR_DO_NOT_TRACK),
            // Some unrelated environment variables
            "PATH=/usr/bin:/bin".to_string(),
            "HOME=/root".to_string(),
        ];

        let container_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                env: Some(env_variables),
                ..Default::default()
            }),
            ..Default::default()
        };

        let env_vars = EnvironmentVariables::from(&container_response);

        // Verify only the expected fields are set
        assert_eq!(env_vars.tool, Some(CreationSource::Container));
        assert_eq!(
            env_vars.mongodb_initdb_root_username,
            Some("testuser".to_string())
        );
        assert_eq!(env_vars.do_not_track, Some("false".to_string()));

        // These should be None as they weren't provided
        assert_eq!(env_vars.runner_log_file, None);
        assert_eq!(env_vars.mongodb_initdb_root_username_file, None);
        assert_eq!(env_vars.mongodb_initdb_root_password, None);
        assert_eq!(env_vars.mongodb_initdb_root_password_file, None);
        assert_eq!(env_vars.mongodb_initdb_database, None);
        assert_eq!(env_vars.mongot_log_file, None);
        assert_eq!(env_vars.telemetry_base_url, None);
        assert_eq!(env_vars.mongodb_load_sample_data, None);
    }

    #[test]
    fn test_from_container_inspect_response_unknown_tool() {
        // Test case: tool env variable has unknown value
        let env_variables = vec![format!("{}=UNKNOWN_TOOL", ENV_VAR_TOOL)];

        let container_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                env: Some(env_variables),
                ..Default::default()
            }),
            ..Default::default()
        };

        let env_vars = EnvironmentVariables::from(&container_response);

        assert_eq!(
            env_vars.tool,
            Some(CreationSource::Unknown("UNKNOWN_TOOL".to_string()))
        );
    }
}
