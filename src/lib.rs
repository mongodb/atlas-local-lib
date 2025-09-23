#![doc = include_str!("../README.md")]

pub mod client;
pub mod docker;
pub mod models;
pub mod mongodb;

#[cfg(test)]
pub mod test_utils;

// Re-export the main types for convenience
pub use client::{
    Client, CreateDeploymentError, DeleteDeploymentError, GetConnectionStringError,
    GetDeploymentError, GetDeploymentIdError, PullImageError,
};
