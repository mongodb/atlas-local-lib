use crate::{client::Client, docker::DockerPullImage};

#[derive(Debug, thiserror::Error)]
#[error("Failed to pull image: {0}")]
pub struct PullImageError(#[from] bollard::errors::Error);

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
