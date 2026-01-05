use tokio::time;

/// Options for waiting for a deployment to become healthy.
///
/// This struct provides configuration options for waiting for a container
/// to reach a healthy state, including setting a maximum timeout duration.
///
/// # Examples
///
/// ```
/// use atlas_local::models::WatchOptions;
/// use std::time::Duration;
///
/// let options = WatchOptions::builder()
///     .timeout_duration(Duration::from_secs(300))
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, typed_builder::TypedBuilder)]
#[builder(doc)]
pub struct WatchOptions {
    /// Maximum duration to wait for the deployment to become healthy.
    #[builder(default, setter(strip_option))]
    pub timeout_duration: Option<time::Duration>,

    /// Indicates that the initial state of the deployment is allowed to be unhealthy.
    #[builder(default = false)]
    pub allow_unhealthy_initial_state: bool,
}
