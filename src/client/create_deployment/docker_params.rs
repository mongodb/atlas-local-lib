use bollard::models::HostConfig;

use crate::models::CreationSource;

use super::CreateDeploymentError;

const DOCKER_PARAMS_ENV_VAR: &str = "MONGODB_ATLAS_LOCAL_DOCKER_PARAMS";

pub(super) fn docker_params_from_env(
    creation_source: Option<&CreationSource>,
) -> Result<Option<HostConfig>, CreateDeploymentError> {
    if !matches!(creation_source, Some(&CreationSource::AtlasLocal)) {
        return Ok(None);
    }

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

#[cfg(test)]
mod tests {
    use crate::models::{
        BindingType, CreateDeploymentOptions, CreationSource, LOCAL_SEED_LOCATION,
        MongoDBPortBinding,
    };

    use super::{docker_params_from_env, parse_docker_params};

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
    fn docker_params_are_ignored_for_other_creation_sources() {
        for source in [
            CreationSource::Container,
            CreationSource::Unknown("PYTHON_LIB".to_string()),
        ] {
            assert!(docker_params_from_env(Some(&source)).unwrap().is_none());
        }
        assert!(docker_params_from_env(None).unwrap().is_none());
    }

    #[test]
    fn to_container_create_body_merges_options_and_preserves_atlas_local_settings() {
        let options = CreateDeploymentOptions {
            local_seed_location: Some("/host/seed-data".to_string()),
            mongodb_port_binding: Some(MongoDBPortBinding::new(Some(27017), BindingType::Loopback)),
            ..Default::default()
        };
        let host_config = parse_docker_params(Some(
            r#"{"Ulimits":[{"Name":"nofile","Soft":64000,"Hard":64000}]}"#,
        ))
        .unwrap()
        .unwrap();

        let request = options.to_container_create_body(Some(host_config));

        let host_config = request.host_config.unwrap();
        assert!(host_config.ulimits.is_some());
        assert!(host_config.port_bindings.is_some());
        assert_eq!(
            host_config.binds,
            Some(vec![format!("/host/seed-data:{LOCAL_SEED_LOCATION}:rw")])
        );
    }
}
