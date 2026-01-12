use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreationSource {
    AtlasCLI,
    Container,
    MCPServer,
    AtlasLocal,
    Unknown(String),
}

impl From<&str> for CreationSource {
    fn from(s: &str) -> Self {
        match s {
            "ATLASCLI" => CreationSource::AtlasCLI,
            "CONTAINER" => CreationSource::Container,
            "MCPSERVER" => CreationSource::MCPServer,
            "ATLAS_LOCAL" => CreationSource::AtlasLocal,
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
            CreationSource::AtlasLocal => write!(f, "ATLAS_LOCAL"),
            CreationSource::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CreationSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CreationSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(CreationSource::from(s.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

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
    fn test_creation_source_from_atlas_local() {
        let source = CreationSource::from("ATLAS_LOCAL");
        assert_eq!(source, CreationSource::AtlasLocal);
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
    fn test_creation_source_to_string_atlas_local() {
        let source = CreationSource::AtlasLocal;
        assert_eq!(source.to_string(), "ATLAS_LOCAL");
    }

    #[test]
    fn test_creation_source_to_string_unknown() {
        let source = CreationSource::Unknown("custom_source".to_string());
        assert_eq!(source.to_string(), "custom_source");
    }

    #[test]
    fn test_json_serialization() {
        #[derive(Serialize)]
        struct Test {
            source: CreationSource,
        }
        let json = serde_json::to_value(&Test {
            source: CreationSource::AtlasCLI,
        })
        .unwrap();
        assert_eq!(json, json!({"source": "ATLASCLI"}));
    }

    #[test]
    fn test_json_deserialization() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Test {
            source: CreationSource,
        }
        let json = json!({"source": "OTHER"});
        let source = serde_json::from_value::<Test>(json).unwrap();
        assert_eq!(
            source,
            Test {
                source: CreationSource::Unknown("OTHER".to_string()),
            }
        );
    }
}
