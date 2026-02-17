use std::fmt::{Display, Formatter};

use crate::models::MongoDBVersion;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageTag {
    Preview,
    Latest,
    Semver(MongoDBVersion),
    /// Semver with datestamp suffix, e.g. `8.2.4-20260217T084055Z`
    SemverDatestamp(MongoDBVersion, String),
}

const PARSE_ERROR: &str = "Invalid image tag: expected 'preview', 'latest', semver (e.g. 8.2.4), or semver+datestamp (e.g. 8.2.4-20260217T084055Z)";

impl TryFrom<&str> for ImageTag {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let s = s.trim();
        if s == "preview" {
            return Ok(ImageTag::Preview);
        }
        if s == "latest" {
            return Ok(ImageTag::Latest);
        }
        // Plain semver (no hyphen)? (e.g. 8.2.4)
        if !s.contains('-') {
            return Ok(ImageTag::Semver(
                MongoDBVersion::try_from(s).map_err(|_| PARSE_ERROR.to_string())?,
            ));
        }
        // semver+datestamp: "X.Y.Z-datestamp"
        let (prefix, suffix) = s.split_once('-').ok_or_else(|| PARSE_ERROR.to_string())?;
        if prefix.is_empty() {
            return Err(PARSE_ERROR.to_string());
        }
        let version = MongoDBVersion::try_from(prefix).map_err(|_| PARSE_ERROR.to_string())?;
        Ok(ImageTag::SemverDatestamp(version, suffix.to_string()))
    }
}

impl Display for ImageTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageTag::Preview => write!(f, "preview"),
            ImageTag::Latest => write!(f, "latest"),
            ImageTag::Semver(v) => write!(f, "{}", v),
            ImageTag::SemverDatestamp(version, datestamp) => write!(f, "{}-{}", version, datestamp),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview() {
        let tag = ImageTag::try_from("preview").unwrap();
        assert_eq!(tag, ImageTag::Preview);
        assert_eq!(tag.to_string(), "preview");
    }

    #[test]
    fn latest() {
        let tag = ImageTag::try_from("latest").unwrap();
        assert_eq!(tag, ImageTag::Latest);
        assert_eq!(tag.to_string(), "latest");
    }

    #[test]
    fn semver() {
        let tag = ImageTag::try_from("8.0.0").unwrap();
        assert!(matches!(tag, ImageTag::Semver(_)));
        assert_eq!(tag.to_string(), "8.0.0");
    }

    #[test]
    fn semver_datestamp() {
        use crate::models::{MongoDBVersion, MongoDBVersionMajorMinorPatch};
        let tag = ImageTag::try_from("8.2.4-20260217T084055Z").unwrap();
        let expected_version = MongoDBVersion::MajorMinorPatch(MongoDBVersionMajorMinorPatch {
            major: 8,
            minor: 2,
            patch: 4,
        });
        assert!(matches!(&tag, ImageTag::SemverDatestamp(_, _)));
        assert_eq!(tag.to_string(), "8.2.4-20260217T084055Z");
        if let ImageTag::SemverDatestamp(v, d) = &tag {
            assert_eq!(v, &expected_version);
            assert_eq!(d, "20260217T084055Z");
        }
    }

    #[test]
    fn invalid() {
        assert!(ImageTag::try_from("invalid").is_err());
        assert!(ImageTag::try_from("1.2.3.4").is_err());
    }
}
