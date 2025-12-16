use std::net::IpAddr;

use bollard::secret::{ContainerInspectResponse, PortBinding};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MongoDBPortBinding {
    pub port: Option<u16>,
    pub binding_type: BindingType,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingType {
    Loopback,         // 127.0.0.1
    AnyInterface,     // 0.0.0.0
    Specific(IpAddr), // Specific IP address
}

#[derive(Debug, thiserror::Error, PartialEq)]
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
    pub fn new(port: Option<u16>, binding_type: BindingType) -> Self {
        Self { port, binding_type }
    }

    pub fn try_from(
        value: &ContainerInspectResponse,
    ) -> Result<Option<MongoDBPortBinding>, GetMongoDBPortBindingError> {
        let Some(ports) = Self::get_mongodb_ports(value) else {
            return Ok(None);
        };

        let ports = ports
            .iter()
            .map(ParsedPortBinding::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        // Ensure we have the expected number of port bindings
        match ports.len() {
            // If there are no port bindings, we return None
            0 => return Ok(None),
            // If there is one port binding, we can proceed
            1 => {}
            // If there are multiple host IPs, they should all have the same port number and either all be loopback or all be any interface
            // Multiple specific host IPs are not supported
            _ => {
                // Ensure all port bindings have the same port number
                let port_number = ports
                    .first()
                    .ok_or(GetMongoDBPortBindingError::MissingPortNumber)?
                    .host_port;
                let all_the_same_port_number = ports.iter().all(|p| p.host_port == port_number);
                if !all_the_same_port_number {
                    return Err(GetMongoDBPortBindingError::MultiplePortsFound);
                }

                // Ensure all port bindings are either loopback or any interface
                let all_loopback = ports.iter().all(|p| p.host_ip.is_loopback());
                let all_any_interface = ports.iter().all(|p| p.host_ip.is_unspecified());

                // If the port bindings are not all loopback or all any interface, we return an error
                if !(all_loopback || all_any_interface) {
                    return Err(GetMongoDBPortBindingError::MultiplePortsFound);
                }
            }
        }

        // It's safe to unwrap because we checked the length above
        #[allow(clippy::unwrap_used)]
        let port = ports.first().unwrap();

        // Get the binding type (determine if it's any interface, loopback, or specific IP address)
        let binding_type = match port.host_ip {
            ip if ip.is_unspecified() => BindingType::AnyInterface,
            ip if ip.is_loopback() => BindingType::Loopback,
            ip => BindingType::Specific(ip),
        };

        Ok(Some(MongoDBPortBinding::new(
            Some(port.host_port),
            binding_type,
        )))
    }

    fn get_mongodb_ports(value: &ContainerInspectResponse) -> Option<&Vec<PortBinding>> {
        let network_settings = value.network_settings.as_ref()?;
        let port = network_settings.ports.as_ref()?;
        let ports = port.get("27017/tcp")?.as_ref()?;
        Some(ports)
    }
}

struct ParsedPortBinding {
    host_ip: IpAddr,
    host_port: u16,
}

impl TryFrom<&PortBinding> for ParsedPortBinding {
    type Error = GetMongoDBPortBindingError;

    fn try_from(value: &PortBinding) -> Result<Self, Self::Error> {
        // Get the port number (convert optional string to u16)
        let host_port = value
            .host_port
            .as_ref()
            .ok_or(GetMongoDBPortBindingError::MissingPortNumber)?
            .parse::<u16>()
            .map_err(GetMongoDBPortBindingError::InvalidPortNumber)?;

        // Get the host IP
        let host_ip = value
            .host_ip
            .as_ref()
            .ok_or(GetMongoDBPortBindingError::MissingHostIP)?
            .parse::<IpAddr>()
            .map_err(GetMongoDBPortBindingError::InvalidHostIP)?;

        Ok(ParsedPortBinding { host_ip, host_port })
    }
}

impl From<&MongoDBPortBinding> for PortBinding {
    fn from(mdb_port_binding: &MongoDBPortBinding) -> Self {
        let host_ip = match mdb_port_binding.binding_type {
            BindingType::AnyInterface => "0.0.0.0".to_string(),
            BindingType::Loopback => "127.0.0.1".to_string(),
            BindingType::Specific(ip) => ip.to_string(),
        };
        PortBinding {
            host_ip: Some(host_ip),
            host_port: mdb_port_binding.port.map(|port| port.to_string()),
        }
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
        assert_eq!(binding.port, Some(27017));
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
        assert_eq!(binding.port, Some(27017));
        assert_eq!(binding.binding_type, BindingType::AnyInterface);
    }

    #[test]
    fn test_try_from_successful_parse_two_any_interface() {
        let container = create_container_response_with_mongodb_ports(vec![
            create_port_binding("0.0.0.0", "27017"),
            create_port_binding("::", "27017"),
        ]);

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_some());

        let binding = result.unwrap();
        assert_eq!(binding.port, Some(27017));
        assert_eq!(binding.binding_type, BindingType::AnyInterface);
    }

    #[test]
    fn test_try_from_successful_parse_many_any_interface() {
        let container = create_container_response_with_mongodb_ports(vec![
            create_port_binding("0.0.0.0", "27017"),
            create_port_binding("::", "27017"),
            create_port_binding("0.0.0.0", "27017"),
            create_port_binding("::", "27017"),
        ]);

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_some());

        let binding = result.unwrap();
        assert_eq!(binding.port, Some(27017));
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
        assert_eq!(binding.port, Some(27017));
        assert_eq!(
            binding.binding_type,
            BindingType::Specific("192.168.1.100".parse().unwrap())
        );
    }

    #[test]
    fn test_try_from_successful_parse_loopback_ipv6() {
        let container =
            create_container_response_with_mongodb_ports(vec![create_port_binding("::1", "27017")]);

        let result = MongoDBPortBinding::try_from(&container).unwrap();
        assert!(result.is_some());

        let binding = result.unwrap();
        assert_eq!(binding.port, Some(27017));
        assert_eq!(binding.binding_type, BindingType::Loopback);
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
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_try_from_multiple_ports_different_port_number_found() {
        let container = create_container_response_with_mongodb_ports(vec![
            create_port_binding("127.0.0.1", "27017"),
            create_port_binding("0.0.0.0", "27018"),
        ]);

        let result = MongoDBPortBinding::try_from(&container);
        assert_eq!(result, Err(GetMongoDBPortBindingError::MultiplePortsFound));
    }

    #[test]
    fn test_try_from_multiple_ports_same_port_number_found() {
        let container = create_container_response_with_mongodb_ports(vec![
            create_port_binding("127.0.0.1", "27017"),
            create_port_binding("::1", "27017"),
        ]);

        let result = MongoDBPortBinding::try_from(&container);
        assert_eq!(
            result,
            Ok(Some(MongoDBPortBinding::new(
                Some(27017),
                BindingType::Loopback
            )))
        );
    }

    #[test]
    fn test_try_from_multiple_ports_ipv4_and_ipv6_found() {
        let container = create_container_response_with_mongodb_ports(vec![
            create_port_binding("127.0.0.1", "27017"),
            create_port_binding("::1", "27018"),
        ]);

        let result = MongoDBPortBinding::try_from(&container);
        assert_eq!(result, Err(GetMongoDBPortBindingError::MultiplePortsFound));
    }

    #[test]
    fn test_try_from_multiple_ports_different_addresses_found() {
        let container = create_container_response_with_mongodb_ports(vec![
            create_port_binding("127.0.0.1", "27017"),
            create_port_binding("192.168.1.100", "27017"),
        ]);

        let result = MongoDBPortBinding::try_from(&container);
        assert_eq!(result, Err(GetMongoDBPortBindingError::MultiplePortsFound));
    }

    #[test]
    fn test_try_from_missing_port_number() {
        let container = create_container_response_with_mongodb_ports(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: None,
        }]);

        let result = MongoDBPortBinding::try_from(&container);
        assert_eq!(result, Err(GetMongoDBPortBindingError::MissingPortNumber));
    }

    #[test]
    fn test_try_from_invalid_port_number() {
        let container = create_container_response_with_mongodb_ports(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some("invalid_port".to_string()),
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
        }]);

        let result = MongoDBPortBinding::try_from(&container);
        assert_eq!(result, Err(GetMongoDBPortBindingError::MissingHostIP));
    }

    #[test]
    fn test_try_from_invalid_host_ip() {
        let container = create_container_response_with_mongodb_ports(vec![PortBinding {
            host_ip: Some("invalid_ip".to_string()),
            host_port: Some("27017".to_string()),
        }]);

        let result = MongoDBPortBinding::try_from(&container);
        assert!(matches!(
            result,
            Err(GetMongoDBPortBindingError::InvalidHostIP(_))
        ));
    }

    #[test]
    fn test_loopback_into_port_binding_vec() {
        let mdb_port_binding = MongoDBPortBinding::new(Some(27017), BindingType::Loopback);
        let port_bindings: PortBinding = (&mdb_port_binding).into();

        assert_eq!(port_bindings.host_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(port_bindings.host_port.as_deref(), Some("27017"));
    }
    #[test]
    fn test_any_interface_into_port_binding_vec() {
        let mdb_port_binding = MongoDBPortBinding::new(Some(27017), BindingType::AnyInterface);
        let port_bindings: PortBinding = (&mdb_port_binding).into();

        assert_eq!(port_bindings.host_ip.as_deref(), Some("0.0.0.0"));
        assert_eq!(port_bindings.host_port.as_deref(), Some("27017"));
    }
    #[test]
    fn test_specific_ip_into_port_binding_vec() {
        let specific_ip: IpAddr = "128.128.128.128".parse().unwrap();
        let mdb_port_binding =
            MongoDBPortBinding::new(Some(27017), BindingType::Specific(specific_ip));
        let port_bindings: PortBinding = (&mdb_port_binding).into();

        assert_eq!(port_bindings.host_ip.as_deref(), Some("128.128.128.128"));
        assert_eq!(port_bindings.host_port.as_deref(), Some("27017"));
    }
    #[test]
    fn test_specific_ip_into_port_binding_vec_no_port() {
        let specific_ip: IpAddr = "128.128.128.128".parse().unwrap();
        let mdb_port_binding = MongoDBPortBinding::new(None, BindingType::Specific(specific_ip));
        let port_bindings: PortBinding = (&mdb_port_binding).into();

        assert_eq!(port_bindings.host_ip.as_deref(), Some("128.128.128.128"));
        assert_eq!(port_bindings.host_port.as_deref(), None);
    }
}
