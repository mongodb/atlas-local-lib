use bollard::{
    query_parameters::CreateContainerOptions,
    secret::{ContainerCreateBody, HostConfig, PortBinding},
};
use maplit::hashmap;
use rand::Rng;
use semver::Version;
use std::{time::Duration, vec};

use crate::models::{
    CreationSource, ENV_VAR_DO_NOT_TRACK, ENV_VAR_MONGODB_INITDB_DATABASE,
    ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD, ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD_FILE,
    ENV_VAR_MONGODB_INITDB_ROOT_USERNAME, ENV_VAR_MONGODB_INITDB_ROOT_USERNAME_FILE,
    ENV_VAR_MONGOT_LOG_FILE, ENV_VAR_RUNNER_LOG_FILE, ENV_VAR_TELEMETRY_BASE_URL, ENV_VAR_TOOL,
    LOCAL_DEPLOYMENT_LABEL_KEY, LOCAL_DEPLOYMENT_LABEL_VALUE,
};
use crate::models::{MongoDBPortBinding, deployment::LOCAL_SEED_LOCATION};
pub const ATLAS_LOCAL_IMAGE: &str = "mongodb/mongodb-atlas-local";
pub const ATLAS_LOCAL_VERSION_TAG: Version = Version::new(8, 0, 0);

#[derive(Debug, Default)]
pub struct CreateDeploymentOptions {
    // Identifiers
    pub name: Option<String>,

    // Image details
    pub image: Option<String>,
    pub mongodb_version: Option<Version>,

    // Creation Options
    pub wait_until_healthy: Option<bool>,
    pub wait_until_healthy_timeout: Option<Duration>,
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
    pub do_not_track: Option<bool>,
    pub telemetry_base_url: Option<String>,

    // Port configuration
    pub mongodb_port_binding: Option<MongoDBPortBinding>,
}

impl From<&CreateDeploymentOptions> for CreateContainerOptions {
    fn from(deployment_options: &CreateDeploymentOptions) -> Self {
        let name = deployment_options
            .name
            .clone()
            .unwrap_or_else(|| format!("local{}", rand::rng().random_range(0..10000)));

        CreateContainerOptions {
            name: Some(name),
            ..Default::default()
        }
    }
}

impl From<&CreateDeploymentOptions> for ContainerCreateBody {
    fn from(deployment_options: &CreateDeploymentOptions) -> Self {
        // Get the port bindings if available, otherwise default to binding a random avaiable port on 127.0.0.1
        let port_binding = deployment_options
            .mongodb_port_binding
            .as_ref()
            .map(PortBinding::from)
            .unwrap_or_else(|| PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: None,
            });

        let port_bindings_map = Some(hashmap! {
            "27017/tcp".to_string() => Some(vec![port_binding])
        });

        // Set up volume bindings if a local seed location is provided
        let volume_bindings_map =
            deployment_options
                .local_seed_location
                .as_ref()
                .map(|local_seed_location| {
                    vec![format!("{local_seed_location}:{LOCAL_SEED_LOCATION}:rw")]
                });

        // Set environment variables if they are provided in the deployment options
        let mut env_vars = [
            (
                ENV_VAR_RUNNER_LOG_FILE,
                deployment_options.runner_log_file.as_ref(),
            ),
            (
                ENV_VAR_MONGODB_INITDB_ROOT_USERNAME,
                deployment_options.mongodb_initdb_root_username.as_ref(),
            ),
            (
                ENV_VAR_MONGODB_INITDB_ROOT_USERNAME_FILE,
                deployment_options
                    .mongodb_initdb_root_username_file
                    .as_ref(),
            ),
            (
                ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD,
                deployment_options.mongodb_initdb_root_password.as_ref(),
            ),
            (
                ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD_FILE,
                deployment_options
                    .mongodb_initdb_root_password_file
                    .as_ref(),
            ),
            (
                ENV_VAR_MONGODB_INITDB_DATABASE,
                deployment_options.mongodb_initdb_database.as_ref(),
            ),
            (
                ENV_VAR_MONGOT_LOG_FILE,
                deployment_options.mongot_log_file.as_ref(),
            ),
            (
                ENV_VAR_DO_NOT_TRACK,
                deployment_options
                    .do_not_track
                    .as_ref()
                    .map(|b| b.to_string())
                    .as_ref(),
            ),
            (
                ENV_VAR_TELEMETRY_BASE_URL,
                deployment_options.telemetry_base_url.as_ref(),
            ),
        ]
        .into_iter()
        .filter_map(|(env_key, value_opt)| {
            value_opt.map(|env_value| format!("{env_key}={env_value}"))
        })
        .collect::<Vec<String>>();

        match deployment_options.creation_source {
            Some(CreationSource::AtlasCLI) => env_vars.push(format!("{}=ATLASCLI", ENV_VAR_TOOL)),
            Some(CreationSource::Container) => env_vars.push(format!("{}=CONTAINER", ENV_VAR_TOOL)),
            Some(CreationSource::MCPServer) => env_vars.push(format!("{}=MCPSERVER", ENV_VAR_TOOL)),
            Some(CreationSource::Unknown(ref s)) => {
                env_vars.push(format!("{}={}", ENV_VAR_TOOL, s))
            }
            None => {}
        }

        // Only set env if we have any to set, otherwise leave it as None
        let env = if env_vars.is_empty() {
            None
        } else {
            Some(env_vars)
        };

        // Get the image and tag
        let image_string = deployment_options
            .image
            .clone()
            .unwrap_or(ATLAS_LOCAL_IMAGE.to_string());

        let tag = deployment_options
            .mongodb_version
            .as_ref()
            .map_or_else(|| "latest".to_string(), |version| version.to_string());

        let image = Some(format!("{image_string}:{tag}"));

        // Get labels
        let labels = Some(hashmap! {
            LOCAL_DEPLOYMENT_LABEL_KEY.to_string() => LOCAL_DEPLOYMENT_LABEL_VALUE.to_string(),
        });

        ContainerCreateBody {
            image,
            labels,
            env,
            host_config: Some(HostConfig {
                port_bindings: port_bindings_map,
                binds: volume_bindings_map,
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::models::BindingType;

    use super::*;

    #[test]
    fn test_into_container_create_body_full() {
        // Create a full CreateDeploymentOptions with all fields set
        let create_deployment_options = CreateDeploymentOptions {
            name: Some("deployment_name".to_string()),
            image: Some(ATLAS_LOCAL_IMAGE.to_string()),
            mongodb_version: Some(ATLAS_LOCAL_VERSION_TAG),
            wait_until_healthy: Some(true),
            wait_until_healthy_timeout: Some(Duration::from_secs(60)),
            creation_source: Some(CreationSource::Container),
            local_seed_location: Some("/host/seed-data".to_string()),
            mongodb_initdb_database: Some("testdb".to_string()),
            mongodb_initdb_root_password_file: Some("/run/secrets/password".to_string()),
            mongodb_initdb_root_password: Some("password123".to_string()),
            mongodb_initdb_root_username_file: Some("/run/secrets/username".to_string()),
            mongodb_initdb_root_username: Some("admin".to_string()),
            mongot_log_file: Some("/tmp/mongot.log".to_string()),
            runner_log_file: Some("/tmp/runner.log".to_string()),
            do_not_track: Some(false),
            telemetry_base_url: Some("https://telemetry.example.com".to_string()),
            mongodb_port_binding: Some(MongoDBPortBinding::new(Some(50000), BindingType::Loopback)),
        };

        // Convert to ContainerCreateBody
        let container_create_body: ContainerCreateBody =
            ContainerCreateBody::from(&create_deployment_options);

        // Assert all fields are set correctly
        assert_eq!(
            container_create_body.image,
            Some("mongodb/mongodb-atlas-local:8.0.0".to_string())
        );
        assert_eq!(
            container_create_body
                .labels
                .unwrap()
                .get(LOCAL_DEPLOYMENT_LABEL_KEY),
            Some(&LOCAL_DEPLOYMENT_LABEL_VALUE.to_string())
        );

        // Check Creation Options
        assert_eq!(create_deployment_options.wait_until_healthy, Some(true));
        assert_eq!(
            create_deployment_options.creation_source,
            Some(CreationSource::Container)
        );

        // Check environment variables
        let env_vars = container_create_body.env.unwrap();
        assert!(env_vars.contains(&format!("{}=CONTAINER", ENV_VAR_TOOL)));
        assert!(env_vars.contains(&format!("{}=/tmp/runner.log", ENV_VAR_RUNNER_LOG_FILE)));
        assert!(env_vars.contains(&format!("{}=admin", ENV_VAR_MONGODB_INITDB_ROOT_USERNAME)));
        assert!(env_vars.contains(&format!(
            "{}=/run/secrets/username",
            ENV_VAR_MONGODB_INITDB_ROOT_USERNAME_FILE
        )));
        assert!(env_vars.contains(&format!(
            "{}=password123",
            ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD
        )));
        assert!(env_vars.contains(&format!(
            "{}=/run/secrets/password",
            ENV_VAR_MONGODB_INITDB_ROOT_PASSWORD_FILE
        )));
        assert!(env_vars.contains(&format!("{}=testdb", ENV_VAR_MONGODB_INITDB_DATABASE)));
        assert!(env_vars.contains(&format!("{}=/tmp/mongot.log", ENV_VAR_MONGOT_LOG_FILE)));
        assert!(env_vars.contains(&format!("{}=false", ENV_VAR_DO_NOT_TRACK)));
        assert!(env_vars.contains(&format!(
            "{}=https://telemetry.example.com",
            ENV_VAR_TELEMETRY_BASE_URL
        )));
        assert_eq!(env_vars.len(), 10);

        let host_config = container_create_body.host_config.unwrap();
        let port_bindings = host_config.port_bindings.unwrap();
        let port_binding = port_bindings
            .get("27017/tcp")
            .unwrap()
            .as_ref()
            .unwrap()
            .first()
            .unwrap();
        assert_eq!(port_binding.host_ip, Some("127.0.0.1".to_string()));
        assert_eq!(port_binding.host_port, Some("50000".to_string()));

        let volumn_binds = host_config.binds.unwrap();
        assert_eq!(volumn_binds.len(), 1);
        assert_eq!(
            volumn_binds[0],
            format!("/host/seed-data:{}:rw", LOCAL_SEED_LOCATION)
        );
    }

    #[test]
    fn test_into_container_create_body_minimal() {
        // Create a minimal CreateDeploymentOptions with only required fields set through defaults
        let create_deployment_options = CreateDeploymentOptions::default();

        // Convert to ContainerCreateBody
        let container_create_body: ContainerCreateBody =
            ContainerCreateBody::from(&create_deployment_options);

        // Assert default fields are set correctly and optional fields are None
        assert_eq!(
            container_create_body.image,
            Some(format!("{ATLAS_LOCAL_IMAGE}:latest"))
        );

        assert!(container_create_body.env.is_none());

        let host_config = container_create_body.host_config.unwrap();
        let port_bindings = host_config.port_bindings.unwrap();
        let port_binding = port_bindings
            .get("27017/tcp")
            .unwrap()
            .as_ref()
            .unwrap()
            .first()
            .unwrap();

        assert_eq!(port_binding.host_ip, Some("127.0.0.1".to_string()));
        assert!(port_binding.host_port.is_none());

        assert_eq!(
            container_create_body
                .labels
                .unwrap()
                .get(LOCAL_DEPLOYMENT_LABEL_KEY),
            Some(&LOCAL_DEPLOYMENT_LABEL_VALUE.to_string())
        );
        assert!(container_create_body.exposed_ports.is_none());
    }

    #[test]
    fn test_into_create_container_options_minimal() {
        // Create a minimal CreateDeploymentOptions with only name set
        let create_deployment_options = CreateDeploymentOptions {
            name: Some("deployment_name".to_string()),
            ..Default::default()
        };

        let create_container_options: CreateContainerOptions =
            CreateContainerOptions::from(&create_deployment_options);

        // Assert the name is set correctly
        assert_eq!(
            create_container_options.name,
            Some("deployment_name".to_string())
        );
    }

    #[test]
    fn test_into_create_container_options_default() {
        // Create a default CreateDeploymentOptions
        let options: CreateDeploymentOptions = CreateDeploymentOptions::default();
        let create_container_options: CreateContainerOptions =
            CreateContainerOptions::from(&options);

        // Name should start with "local" followed by random numbers
        assert!(create_container_options.name.unwrap().starts_with("local"));
    }

    #[test]
    fn test_create_deployment_options_default() {
        let options = CreateDeploymentOptions::default();

        // All fields should be None by default
        assert!(options.name.is_none());
        assert!(options.image.is_none());
        assert!(options.mongodb_version.is_none());
        assert!(options.wait_until_healthy.is_none());
        assert!(options.wait_until_healthy_timeout.is_none());
        assert!(options.creation_source.is_none());
        assert!(options.local_seed_location.is_none());
        assert!(options.mongodb_initdb_database.is_none());
        assert!(options.mongodb_initdb_root_password_file.is_none());
        assert!(options.mongodb_initdb_root_password.is_none());
        assert!(options.mongodb_initdb_root_username_file.is_none());
        assert!(options.mongodb_initdb_root_username.is_none());
        assert!(options.mongot_log_file.is_none());
        assert!(options.runner_log_file.is_none());
        assert!(options.do_not_track.is_none());
        assert!(options.telemetry_base_url.is_none());
        assert!(options.mongodb_port_binding.is_none());
    }
}
