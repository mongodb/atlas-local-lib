use bollard::models::{ContainerCreateBody, HostConfig};

use super::CreateDeploymentError;

const DOCKER_PARAMS_ENV_VAR: &str = "MONGODB_ATLAS_LOCAL_DOCKER_PARAMS";

pub(super) fn docker_params_from_env() -> Result<Option<HostConfig>, CreateDeploymentError> {
    let params = match std::env::var(DOCKER_PARAMS_ENV_VAR) {
        Ok(params) => params,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => {
            return Err(CreateDeploymentError::InvalidDockerParams(
                error.to_string(),
            ));
        }
    };

    parse_docker_params(Some(&params))
}

fn parse_docker_params(params: Option<&str>) -> Result<Option<HostConfig>, CreateDeploymentError> {
    let Some(params) = params.filter(|params| !params.trim().is_empty()) else {
        return Ok(None);
    };

    let value: serde_json::Value = serde_json::from_str(params)
        .map_err(|error| CreateDeploymentError::InvalidDockerParams(error.to_string()))?;
    if !value.is_object() {
        return Err(CreateDeploymentError::InvalidDockerParams(
            "expected a JSON object describing Docker HostConfig options".to_string(),
        ));
    }

    serde_json::from_value(value)
        .map(Some)
        .map_err(|error| CreateDeploymentError::InvalidDockerParams(error.to_string()))
}

pub(super) fn apply_docker_params(
    container_config: &mut ContainerCreateBody,
    additional_host_config: HostConfig,
) {
    let atlas_local_host_config = container_config.host_config.take().unwrap_or_default();
    container_config.host_config = Some(HostConfig {
        // Atlas Local owns these fields for its database port and optional seed mount.
        port_bindings: atlas_local_host_config.port_bindings,
        binds: atlas_local_host_config.binds,
        ..additional_host_config
    });
}

#[cfg(test)]
mod tests {
    use bollard::models::ContainerCreateBody;

    use crate::models::{
        BindingType, CreateDeploymentOptions, LOCAL_SEED_LOCATION, MongoDBPortBinding,
    };

    use super::{apply_docker_params, parse_docker_params};

    #[test]
    fn parse_docker_params_unset_or_empty() {
        assert!(parse_docker_params(None).unwrap().is_none());
        assert!(parse_docker_params(Some("  ")).unwrap().is_none());
    }

    #[test]
    fn parse_docker_params_host_config() {
        let params = r#"{"Ulimits":[{"Name":"nofile","Soft":64000,"Hard":64000}]}"#;
        let host_config = parse_docker_params(Some(params)).unwrap().unwrap();
        assert!(host_config.ulimits.is_some());
    }

    #[test]
    fn parse_docker_params_rejects_invalid_values() {
        for params in ["not-json", "[]", "null"] {
            assert!(parse_docker_params(Some(params)).is_err(), "{params}");
        }
    }

    #[test]
    fn apply_docker_params_preserves_atlas_local_mounts_and_port_binding() {
        let options = CreateDeploymentOptions {
            local_seed_location: Some("/host/seed-data".to_string()),
            mongodb_port_binding: Some(MongoDBPortBinding::new(Some(27017), BindingType::Loopback)),
            ..Default::default()
        };
        let mut request = ContainerCreateBody::from(&options);
        let host_config = parse_docker_params(Some(
            r#"{"Ulimits":[{"Name":"nofile","Soft":64000,"Hard":64000}]}"#,
        ))
        .unwrap()
        .unwrap();

        apply_docker_params(&mut request, host_config);

        let host_config = request.host_config.unwrap();
        assert!(host_config.ulimits.is_some());
        assert!(host_config.port_bindings.is_some());
        assert_eq!(
            host_config.binds,
            Some(vec![format!("/host/seed-data:{LOCAL_SEED_LOCATION}:rw")])
        );
    }
}
