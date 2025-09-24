use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreationSource {
    AtlasCLI,
    Container,
    MCPServer,
    Unknown(String),
}

impl From<&str> for CreationSource {
    fn from(s: &str) -> Self {
        match s {
            "ATLASCLI" => CreationSource::AtlasCLI,
            "CONTAINER" => CreationSource::Container,
            "MCPSERVER" => CreationSource::MCPServer,
            unknown => CreationSource::Unknown(unknown.to_string()),
        }
    }
}

impl Display for CreationSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreationSource::AtlasCLI => write!(f, "ATLASCLI"),
            CreationSource::Container => write!(f, "CONTAINER"),
            CreationSource::MCPServer => write!(f, "MCPSERVER"),
            CreationSource::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation_source_from_atlascli() {
        let source = CreationSource::from("ATLASCLI");
        assert_eq!(source, CreationSource::AtlasCLI);
    }

    #[test]
    fn test_creation_source_from_container() {
        let source = CreationSource::from("CONTAINER");
        assert_eq!(source, CreationSource::Container);
    }

    #[test]
    fn test_creation_source_from_mcp_server() {
        let source = CreationSource::from("MCPSERVER");
        assert_eq!(source, CreationSource::MCPServer);
    }

    #[test]
    fn test_creation_source_from_unknown() {
        let source = CreationSource::from("some_unknown_source");
        assert_eq!(
            source,
            CreationSource::Unknown("some_unknown_source".to_string())
        );
    }

    #[test]
    fn test_creation_source_to_string_atlascli() {
        let source = CreationSource::AtlasCLI;
        assert_eq!(source.to_string(), "ATLASCLI");
    }

    #[test]
    fn test_creation_source_to_string_container() {
        let source = CreationSource::Container;
        assert_eq!(source.to_string(), "CONTAINER");
    }

    #[test]
    fn test_creation_source_to_string_mcp_server() {
        let source = CreationSource::MCPServer;
        assert_eq!(source.to_string(), "MCPSERVER");
    }

    #[test]
    fn test_creation_source_to_string_unknown() {
        let source = CreationSource::Unknown("custom_source".to_string());
        assert_eq!(source.to_string(), "custom_source");
    }
}
