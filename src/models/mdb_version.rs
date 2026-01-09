use std::fmt::{Display, Formatter};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MongoDBVersion {
    Latest,
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

const PARSE_ERROR_MESSAGE: &str = "Invalid MongoDB version format. Expected format: <major>[.<minor>[.<patch>]] or 'latest'. Some examples: 8, 8.2, 8.2.1, latest";

/// Parse a MongoDB version string into a MongoDBVersion enum.
///
/// Expected format: <major>[.<minor>[.<patch>]] or 'latest'.
/// Some examples: 8, 8.2, 8.2.1, latest    
///
impl TryFrom<&str> for MongoDBVersion {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Special case for latest version.
        if s == "latest" {
            return Ok(MongoDBVersion::Latest);
        }

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
            MongoDBVersion::Latest => write!(f, "latest"),
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
