use std::{net::IpAddr, ops::Deref};

use bollard::secret::{ContainerInspectResponse, PortBinding};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoDBPortBinding {
    pub port: u16,
    pub binding_type: BindingType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingType {
    Loopback,         // 127.0.0.1
    AnyInterface,     // 0.0.0.0
    Specific(IpAddr), // Specific IP address
}

#[derive(Debug, thiserror::Error)]
pub enum GetMongoDBPortBindingError {
    #[error("Multiple MongoDB ports found")]
    MultiplePortsFound,
    #[error("Missing port number")]
    MissingPortNumber,
    #[error("Invalid port number: {0}")]
    InvalidPortNumber(std::num::ParseIntError),
    #[error("Missing host IP")]
    MissingHostIP,
    #[error("Invalid host IP: {0}")]
    InvalidHostIP(std::net::AddrParseError),
}

impl MongoDBPortBinding {
    pub fn new(port: u16, binding_type: BindingType) -> Self {
        Self { port, binding_type }
    }

    pub fn try_from(
        value: &ContainerInspectResponse,
    ) -> Result<Option<MongoDBPortBinding>, GetMongoDBPortBindingError> {
        let Some(ports) = Self::get_mongodb_ports(value) else {
            return Ok(None);
        };

        if ports.len() != 1 {
            return Err(GetMongoDBPortBindingError::MultiplePortsFound);
        }

        // It's safe to unwrap because we checked the length above
        let port = ports.first().unwrap();

        // Get the port number (convert optional string to u16)
        let port_number = port
            .host_port
            .as_ref()
            .ok_or(GetMongoDBPortBindingError::MissingPortNumber)?
            .parse::<u16>()
            .map_err(GetMongoDBPortBindingError::InvalidPortNumber)?;

        // Get the binding type (determine if it's any interface, loopback, or specific IP address)
        let binding_type = match port
            .host_ip
            .as_ref()
            .ok_or(GetMongoDBPortBindingError::MissingHostIP)?
            .deref()
        {
            "0.0.0.0" => BindingType::AnyInterface,
            "127.0.0.1" => BindingType::Loopback,
            ip => BindingType::Specific(
                ip.parse::<IpAddr>()
                    .map_err(GetMongoDBPortBindingError::InvalidHostIP)?,
            ),
        };

        Ok(Some(MongoDBPortBinding::new(port_number, binding_type)))
    }

    fn get_mongodb_ports(value: &ContainerInspectResponse) -> Option<&Vec<PortBinding>> {
        let network_settings = value.network_settings.as_ref()?;
        let port = network_settings.ports.as_ref()?;
        let ports = port.get("27017/tcp")?.as_ref()?;
        Some(ports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::secret::NetworkSettings;
    use std::collections::HashMap;

    fn create_container_response_with_mongodb_ports(
        ports: Vec<PortBinding>,
    ) -> ContainerInspectResponse {
        let mut port_map = HashMap::new();
        port_map.insert("27017/tcp".to_string(), Some(ports));

        ContainerInspectResponse {
            network_settings: Some(NetworkSettings {
                ports: Some(port_map),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn create_port_binding(host_ip: &str, host_port: &str) -> PortBinding {
        PortBinding {
            host_ip: Some(host_ip.to_string()),
            host_port: Some(host_port.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_try_from_successful_parse_loopback() {
        let container = create_container_response_with_mongodb_ports(vec![create_port_binding(
            "127.0.0.1",
            "27017",
        )]);

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_some());

        let binding = result.unwrap();
        assert_eq!(binding.port, 27017);
        assert_eq!(binding.binding_type, BindingType::Loopback);
    }

    #[test]
    fn test_try_from_successful_parse_any_interface() {
        let container = create_container_response_with_mongodb_ports(vec![create_port_binding(
            "0.0.0.0", "27017",
        )]);

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_some());

        let binding = result.unwrap();
        assert_eq!(binding.port, 27017);
        assert_eq!(binding.binding_type, BindingType::AnyInterface);
    }

    #[test]
    fn test_try_from_successful_parse_specific_ipv4() {
        let container = create_container_response_with_mongodb_ports(vec![create_port_binding(
            "192.168.1.100",
            "27017",
        )]);

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_some());

        let binding = result.unwrap();
        assert_eq!(binding.port, 27017);
        assert_eq!(
            binding.binding_type,
            BindingType::Specific("192.168.1.100".parse().unwrap())
        );
    }

    #[test]
    fn test_try_from_successful_parse_specific_ipv6() {
        let container =
            create_container_response_with_mongodb_ports(vec![create_port_binding("::1", "27017")]);

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_some());

        let binding = result.unwrap();
        assert_eq!(binding.port, 27017);
        assert_eq!(
            binding.binding_type,
            BindingType::Specific("::1".parse().unwrap())
        );
    }

    #[test]
    fn test_try_from_missing_network_settings() {
        let container = ContainerInspectResponse {
            network_settings: None,
            ..Default::default()
        };
        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_try_from_missing_ports() {
        let container = ContainerInspectResponse {
            network_settings: Some(NetworkSettings {
                ports: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_try_from_missing_mongodb_port() {
        let mut port_map = HashMap::new();
        port_map.insert(
            "3000/tcp".to_string(),
            Some(vec![create_port_binding("127.0.0.1", "3000")]),
        );

        let container = ContainerInspectResponse {
            network_settings: Some(NetworkSettings {
                ports: Some(port_map),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_try_from_empty_mongodb_ports() {
        let container = create_container_response_with_mongodb_ports(vec![]);
        let result = MongoDBPortBinding::try_from(&container);
        assert!(matches!(
            result,
            Err(GetMongoDBPortBindingError::MultiplePortsFound)
        ));
    }

    #[test]
    fn test_try_from_multiple_ports_found() {
        let container = create_container_response_with_mongodb_ports(vec![
            create_port_binding("127.0.0.1", "27017"),
            create_port_binding("0.0.0.0", "27018"),
        ]);

        let result = MongoDBPortBinding::try_from(&container);
        assert!(matches!(
            result,
            Err(GetMongoDBPortBindingError::MultiplePortsFound)
        ));
    }

    #[test]
    fn test_try_from_missing_port_number() {
        let container = create_container_response_with_mongodb_ports(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: None,
            ..Default::default()
        }]);

        let result = MongoDBPortBinding::try_from(&container);
        assert!(matches!(
            result,
            Err(GetMongoDBPortBindingError::MissingPortNumber)
        ));
    }

    #[test]
    fn test_try_from_invalid_port_number() {
        let container = create_container_response_with_mongodb_ports(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some("invalid_port".to_string()),
            ..Default::default()
        }]);

        let result = MongoDBPortBinding::try_from(&container);
        assert!(matches!(
            result,
            Err(GetMongoDBPortBindingError::InvalidPortNumber(_))
        ));
    }

    #[test]
    fn test_try_from_missing_host_ip() {
        let container = create_container_response_with_mongodb_ports(vec![PortBinding {
            host_ip: None,
            host_port: Some("27017".to_string()),
            ..Default::default()
        }]);

        let result = MongoDBPortBinding::try_from(&container);
        assert!(matches!(
            result,
            Err(GetMongoDBPortBindingError::MissingHostIP)
        ));
    }

    #[test]
    fn test_try_from_invalid_host_ip() {
        let container = create_container_response_with_mongodb_ports(vec![PortBinding {
            host_ip: Some("invalid_ip".to_string()),
            host_port: Some("27017".to_string()),
            ..Default::default()
        }]);

        let result = MongoDBPortBinding::try_from(&container);
        assert!(matches!(
            result,
            Err(GetMongoDBPortBindingError::InvalidHostIP(_))
        ));
    }
}
