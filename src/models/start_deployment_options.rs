use std::time::Duration;

/// Options for starting an existing local Atlas deployment.
///
/// By default `start_deployment` waits for the deployment to become healthy
/// before returning, matching the behavior of `create_deployment`.
///
/// # Examples
///
/// ```
/// use atlas_local::models::StartDeploymentOptions;
/// use std::time::Duration;
///
/// // Wait for the deployment to be healthy, with a custom timeout.
/// let options = StartDeploymentOptions::builder()
///     .wait_until_healthy_timeout(Duration::from_secs(120))
///     .build();
///
/// // Return as soon as the container has been started.
/// let options = StartDeploymentOptions::builder()
///     .wait_until_healthy(false)
///     .build();
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, typed_builder::TypedBuilder)]
#[builder(doc)]
pub struct StartDeploymentOptions {
    /// Whether to wait for the deployment to become healthy before returning.
    /// Defaults to `true`.
    #[builder(default, setter(strip_option))]
    pub wait_until_healthy: Option<bool>,

    /// Maximum duration to wait for the deployment to become healthy.
    #[builder(default, setter(strip_option))]
    pub wait_until_healthy_timeout: Option<Duration>,
}
