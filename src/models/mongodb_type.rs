use std::str::FromStr;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MongodbType {
    Community,
    Enterprise,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
#[error("Invalid mongodb type: {0}")]
pub struct ParseMongodbTypeError(String);

impl FromStr for MongodbType {
    type Err = ParseMongodbTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "community" => Ok(MongodbType::Community),
            "enterprise" => Ok(MongodbType::Enterprise),
            _ => Err(ParseMongodbTypeError(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mongodb_type() {
        assert_eq!(
            MongodbType::from_str("community").unwrap(),
            MongodbType::Community
        );
        assert_eq!(
            MongodbType::from_str("enterprise").unwrap(),
            MongodbType::Enterprise
        );
        assert!(MongodbType::from_str("invalid").is_err());
    }
}
