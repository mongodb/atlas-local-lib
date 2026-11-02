use std::fmt::{Display, Formatter};

use url::Url;

const INVALID_ENDPOINT: &str = "embedding_provider_endpoint must be an absolute HTTPS URL";

/// A validated HTTPS URL used by mongot for embedding requests.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingProviderEndpoint(String);

impl EmbeddingProviderEndpoint {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(value: &str) -> Result<(), String> {
        let url = Url::parse(value).map_err(|_| INVALID_ENDPOINT.to_owned())?;
        if url.scheme() != "https" {
            return Err(INVALID_ENDPOINT.to_owned());
        }

        Ok(())
    }
}

impl TryFrom<String> for EmbeddingProviderEndpoint {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for EmbeddingProviderEndpoint {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl From<EmbeddingProviderEndpoint> for String {
    fn from(endpoint: EmbeddingProviderEndpoint) -> Self {
        endpoint.0
    }
}

impl AsRef<str> for EmbeddingProviderEndpoint {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for EmbeddingProviderEndpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_embedding_endpoints() {
        let endpoint =
            EmbeddingProviderEndpoint::try_from("https://api.voyageai.com/v1/embeddings").unwrap();

        assert_eq!(endpoint.as_str(), "https://api.voyageai.com/v1/embeddings");
    }

    #[test]
    fn rejects_non_https_embedding_endpoints() {
        assert_eq!(
            EmbeddingProviderEndpoint::try_from("http://api.voyageai.com/v1/embeddings")
                .unwrap_err(),
            INVALID_ENDPOINT
        );
    }

    #[test]
    fn rejects_invalid_embedding_endpoints() {
        assert_eq!(
            EmbeddingProviderEndpoint::try_from("not a url").unwrap_err(),
            INVALID_ENDPOINT
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_deserialization_validates_the_scheme() {
        assert!(
            serde_json::from_str::<EmbeddingProviderEndpoint>(
                "\"http://api.voyageai.com/v1/embeddings\""
            )
            .is_err()
        );
    }
}
