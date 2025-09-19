use bollard::secret::{
    ContainerConfig, ContainerInspectResponse, ContainerState, ContainerStateStatusEnum,
};
use maplit::hashmap;

pub fn create_container_inspect_response_with_auth(port: u16) -> ContainerInspectResponse {
    ContainerInspectResponse {
        id: Some("test_container_id".to_string()),
        name: Some("/test-deployment".to_string()),
        config: Some(ContainerConfig {
            labels: Some(hashmap! {
                "mongodb-atlas-local".to_string() => "container".to_string(),
                "version".to_string() => "7.0.0".to_string(),
                "mongodb-type".to_string() => "community".to_string(),
            }),
            ..Default::default()
        }),
        state: Some(ContainerState {
            status: Some(ContainerStateStatusEnum::RUNNING),
            ..Default::default()
        }),
        network_settings: Some(bollard::secret::NetworkSettings {
            ports: Some(hashmap! {
                "27017/tcp".to_string() => Some(vec![
                    bollard::secret::PortBinding {
                        host_ip: Some("127.0.0.1".to_string()),
                        host_port: Some(port.to_string()),
                    }
                ])
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn create_container_inspect_response_no_auth(port: u16) -> ContainerInspectResponse {
    ContainerInspectResponse {
        id: Some("test_container_id".to_string()),
        name: Some("/test-deployment".to_string()),
        config: Some(ContainerConfig {
            labels: Some(hashmap! {
                "mongodb-atlas-local".to_string() => "container".to_string(),
                "version".to_string() => "7.0.0".to_string(),
                "mongodb-type".to_string() => "community".to_string(),
            }),
            ..Default::default()
        }),
        state: Some(ContainerState {
            status: Some(ContainerStateStatusEnum::RUNNING),
            ..Default::default()
        }),
        network_settings: Some(bollard::secret::NetworkSettings {
            ports: Some(hashmap! {
                "27017/tcp".to_string() => Some(vec![
                    bollard::secret::PortBinding {
                        host_ip: Some("127.0.0.1".to_string()),
                        host_port: Some(port.to_string()),
                    }
                ])
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}
