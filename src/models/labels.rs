use bollard::secret::ContainerInspectResponse;
use semver::Version;

use crate::models::{MongodbType, ParseMongodbTypeError};

pub const LOCAL_DEPLOYMENT_LABEL_KEY: &str = "mongodb-atlas-local";
pub const LOCAL_DEPLOYMENT_LABEL_VALUE: &str = "container";

pub const MONGODB_TYPE_LABEL_KEY: &str = "mongodb-type";
pub const MONGODB_VERSION_LABEL_KEY: &str = "version";

#[derive(Debug, PartialEq, Eq)]
pub struct LocalDeploymentLabels {
    pub mongodb_version: Version,
    pub mongodb_type: MongodbType,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GetLocalDeploymentLabelsError {
    #[error("Missing container labels")]
    MissingContainerLabels,
    #[error("Not a local deployment")]
    NotALocalDeployment,
    #[error("Missing mongodb version")]
    MissingMongodbVersion,
    #[error("Invalid mongodb version: {reason}")]
    InvalidMongodbVersion { reason: String },
    #[error("Missing mongodb type")]
    MissingMongodbType,
    #[error(transparent)]
    InvalidMongodbType(#[from] ParseMongodbTypeError),
}

// We're implementing From<T: Borrow<ContainerInspectResponse>> for EnvironmentVariables instead of From<ContainerInspectResponse>
// to allow using both ContainerInspectResponse and &ContainerInspectResponse, we only need a ref to the container inspect response
impl TryFrom<&ContainerInspectResponse> for LocalDeploymentLabels {
    type Error = GetLocalDeploymentLabelsError;

    fn try_from(value: &ContainerInspectResponse) -> Result<Self, Self::Error> {
        // Get the container labels,
        // Every local deployment has these, we should return an error if they are not set
        let container_config = value
            .config
            .as_ref()
            .ok_or(GetLocalDeploymentLabelsError::MissingContainerLabels)?
            .labels
            .as_ref()
            .ok_or(GetLocalDeploymentLabelsError::MissingContainerLabels)?;

        // Verify that the container has the mongodb-atlas-local=container label
        let atlas_local_marker_label = container_config
            .get(LOCAL_DEPLOYMENT_LABEL_KEY)
            .ok_or(GetLocalDeploymentLabelsError::NotALocalDeployment)?;
        if atlas_local_marker_label != LOCAL_DEPLOYMENT_LABEL_VALUE {
            return Err(GetLocalDeploymentLabelsError::NotALocalDeployment);
        }

        // Get the mongodb version (semver)
        let mongodb_version_string = container_config
            .get(MONGODB_VERSION_LABEL_KEY)
            .ok_or(GetLocalDeploymentLabelsError::MissingMongodbVersion)?;
        let mongodb_version = mongodb_version_string.parse::<Version>().map_err(|e| {
            GetLocalDeploymentLabelsError::InvalidMongodbVersion {
                reason: e.to_string(),
            }
        })?;

        // Get the mongodb type (community or enterprise)
        let mongodb_type_string = container_config
            .get(MONGODB_TYPE_LABEL_KEY)
            .ok_or(GetLocalDeploymentLabelsError::MissingMongodbType)?;
        let mongodb_type: MongodbType = mongodb_type_string.parse()?;

        Ok(LocalDeploymentLabels {
            mongodb_version,
            mongodb_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use bollard::secret::ContainerConfig;

    use super::*;

    #[test]
    fn missing_container_config() {
        let container_inspect_response = ContainerInspectResponse::default();
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert_eq!(
            result,
            Err(GetLocalDeploymentLabelsError::MissingContainerLabels)
        );
    }

    #[test]
    fn missing_container_labels() {
        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert_eq!(
            result,
            Err(GetLocalDeploymentLabelsError::MissingContainerLabels)
        );
    }

    #[test]
    fn missing_marker_label() {
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert("some-other-label".to_string(), "value".to_string());

        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert_eq!(
            result,
            Err(GetLocalDeploymentLabelsError::NotALocalDeployment)
        );
    }

    #[test]
    fn invalid_marker_label_value() {
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert(
            LOCAL_DEPLOYMENT_LABEL_KEY.to_string(),
            "wrong-value".to_string(),
        );

        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert_eq!(
            result,
            Err(GetLocalDeploymentLabelsError::NotALocalDeployment)
        );
    }

    #[test]
    fn missing_mongodb_version() {
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert(
            LOCAL_DEPLOYMENT_LABEL_KEY.to_string(),
            LOCAL_DEPLOYMENT_LABEL_VALUE.to_string(),
        );

        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert_eq!(
            result,
            Err(GetLocalDeploymentLabelsError::MissingMongodbVersion)
        );
    }

    #[test]
    fn invalid_mongodb_version() {
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert(
            LOCAL_DEPLOYMENT_LABEL_KEY.to_string(),
            LOCAL_DEPLOYMENT_LABEL_VALUE.to_string(),
        );
        labels.insert(
            MONGODB_VERSION_LABEL_KEY.to_string(),
            "not-a-semver".to_string(),
        );

        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert!(matches!(
            result,
            Err(GetLocalDeploymentLabelsError::InvalidMongodbVersion { .. })
        ));
    }

    #[test]
    fn missing_mongodb_type() {
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert(
            LOCAL_DEPLOYMENT_LABEL_KEY.to_string(),
            LOCAL_DEPLOYMENT_LABEL_VALUE.to_string(),
        );
        labels.insert(MONGODB_VERSION_LABEL_KEY.to_string(), "7.0.0".to_string());

        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert_eq!(
            result,
            Err(GetLocalDeploymentLabelsError::MissingMongodbType)
        );
    }

    #[test]
    fn invalid_mongodb_type() {
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert(
            LOCAL_DEPLOYMENT_LABEL_KEY.to_string(),
            LOCAL_DEPLOYMENT_LABEL_VALUE.to_string(),
        );
        labels.insert(MONGODB_VERSION_LABEL_KEY.to_string(), "7.0.0".to_string());
        labels.insert(
            MONGODB_TYPE_LABEL_KEY.to_string(),
            "invalid-type".to_string(),
        );

        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);
        assert!(matches!(
            result,
            Err(GetLocalDeploymentLabelsError::InvalidMongodbType(_))
        ));
    }

    #[test]
    fn successful_parse() {
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        labels.insert(
            LOCAL_DEPLOYMENT_LABEL_KEY.to_string(),
            LOCAL_DEPLOYMENT_LABEL_VALUE.to_string(),
        );
        labels.insert(MONGODB_VERSION_LABEL_KEY.to_string(), "7.0.0".to_string());
        labels.insert(MONGODB_TYPE_LABEL_KEY.to_string(), "community".to_string());

        let container_inspect_response = ContainerInspectResponse {
            config: Some(ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = LocalDeploymentLabels::try_from(&container_inspect_response);

        assert!(result.is_ok());
        let labels = result.unwrap();
        assert_eq!(labels.mongodb_version, Version::parse("7.0.0").unwrap());
        assert_eq!(labels.mongodb_type, MongodbType::Community);
    }
}
