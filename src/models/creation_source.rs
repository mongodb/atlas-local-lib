#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreationSource {
    AtlasCLI,
    Container,
    Unknown(String),
}

impl From<&str> for CreationSource {
    fn from(s: &str) -> Self {
        match s {
            "ATLASCLI" => CreationSource::AtlasCLI,
            "CONTAINER" => CreationSource::Container,
            unknown => CreationSource::Unknown(unknown.to_string()),
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
    fn test_creation_source_from_unknown() {
        let source = CreationSource::from("some_unknown_source");
        assert_eq!(
            source,
            CreationSource::Unknown("some_unknown_source".to_string())
        );
    }
}
