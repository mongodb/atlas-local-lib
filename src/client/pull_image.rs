use crate::{
    client::Client,
    docker::{DockerError, DockerPullImage},
};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("Failed to pull image: {0}")]
pub struct PullImageError(#[from] DockerError);

impl<D: DockerPullImage> Client<D> {
    /// Pulls the Atlas Local image.
    ///
    /// # Arguments
    ///
    /// * `image` - The image to pull.
    /// * `tag` - The tag to pull.
    pub async fn pull_image(&self, image: &str, tag: &str) -> Result<(), PullImageError> {
        self.docker.pull_image(image, tag).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::DockerError;
    use mockall::mock;

    mock! {
        Docker {}

        impl DockerPullImage for Docker {
            async fn pull_image(&self, image: &str, tag: &str) -> Result<(), DockerError>;
        }
    }

    #[tokio::test]
    async fn test_pull_image() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq("mongodb/mongodb-atlas-local"),
                mockall::predicate::eq("8.0.0"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .pull_image("mongodb/mongodb-atlas-local", "8.0.0")
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pull_image_docker_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_pull_image()
            .with(
                mockall::predicate::eq("mongodb/mongodb-atlas-local"),
                mockall::predicate::eq("invalid-tag"),
            )
            .times(1)
            .returning(|_, _| Err(DockerError::NotFound));

        let client = Client::new(mock_docker);

        // Act
        let result = client
            .pull_image("mongodb/mongodb-atlas-local", "invalid-tag")
            .await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PullImageError(_)));
    }
}
