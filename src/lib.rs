#![doc = include_str!("../README.md")]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod client;
pub mod docker;
pub mod models;

#[cfg(test)]
pub mod test_utils;

// Re-export the main types for convenience
pub use client::{
    Client, CreateDeploymentError, DeleteDeploymentError, GetConnectionStringError,
    GetDeploymentError, GetDeploymentIdError, GetLogsError, PullImageError,
};
