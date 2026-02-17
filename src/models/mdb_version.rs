use std::fmt::{Display, Formatter};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MongoDBVersion {
    Major(MongoDBVersionMajor),
    MajorMinor(MongoDBVersionMajorMinor),
    MajorMinorPatch(MongoDBVersionMajorMinorPatch),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoDBVersionMajor {
    pub major: u8,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoDBVersionMajorMinor {
    pub major: u8,
    pub minor: u8,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoDBVersionMajorMinorPatch {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

const PARSE_ERROR_MESSAGE: &str = "Invalid MongoDB version format. Expected format: <major>[.<minor>[.<patch>]]. Some examples: 8, 8.2, 8.2.1";

/// Parse a MongoDB version string into a MongoDBVersion enum.
///
/// Expected format: <major>[.<minor>[.<patch>]].
/// Some examples: 8, 8.2, 8.2.1
///
impl TryFrom<&str> for MongoDBVersion {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Split the version string by '.' and parse each part as a u8.
        // If that fails, return the PARSE_ERROR_MESSAGE.
        let parts = s
            .split('.')
            .map(|part| part.parse::<u8>())
            .collect::<Result<Vec<u8>, _>>()
            .map_err(|_| PARSE_ERROR_MESSAGE.to_string())?;

        // Match on the number of parts.
        match parts[..] {
            [major] => Ok(MongoDBVersion::Major(MongoDBVersionMajor { major })),
            [major, minor] => Ok(MongoDBVersion::MajorMinor(MongoDBVersionMajorMinor {
                major,
                minor,
            })),
            [major, minor, patch] => Ok(MongoDBVersion::MajorMinorPatch(
                MongoDBVersionMajorMinorPatch {
                    major,
                    minor,
                    patch,
                },
            )),
            // If we have no or more than 3 parts, return the PARSE_ERROR_MESSAGE.
            _ => Err(PARSE_ERROR_MESSAGE.to_string()),
        }
    }
}

impl Display for MongoDBVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MongoDBVersion::Major(major) => write!(f, "{}", major.major),
            MongoDBVersion::MajorMinor(major_minor) => {
                write!(f, "{}.{}", major_minor.major, major_minor.minor)
            }
            MongoDBVersion::MajorMinorPatch(major_minor_patch) => write!(
                f,
                "{}.{}.{}",
                major_minor_patch.major, major_minor_patch.minor, major_minor_patch.patch
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_major() {
        let version = MongoDBVersion::try_from("8").unwrap();
        assert_eq!(
            version,
            MongoDBVersion::Major(MongoDBVersionMajor { major: 8 })
        );
        assert_eq!(version.to_string(), "8");
    }

    #[test]
    fn test_parse_major_minor() {
        let version = MongoDBVersion::try_from("8.1").unwrap();
        assert_eq!(
            version,
            MongoDBVersion::MajorMinor(MongoDBVersionMajorMinor { major: 8, minor: 1 })
        );
        assert_eq!(version.to_string(), "8.1");
    }

    #[test]
    fn test_parse_major_minor_patch() {
        let version = MongoDBVersion::try_from("7.3.2").unwrap();
        assert_eq!(
            version,
            MongoDBVersion::MajorMinorPatch(MongoDBVersionMajorMinorPatch {
                major: 7,
                minor: 3,
                patch: 2
            })
        );
        assert_eq!(version.to_string(), "7.3.2");
    }

    #[test]
    fn test_parse_invalid() {
        let result = MongoDBVersion::try_from("invalid");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PARSE_ERROR_MESSAGE);
    }

    #[test]
    fn test_parse_too_many_parts() {
        let result = MongoDBVersion::try_from("1.2.3.4");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PARSE_ERROR_MESSAGE);
    }
}
