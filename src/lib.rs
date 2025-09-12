#![doc = include_str!("../README.md")]

pub mod client;
pub mod docker;
pub mod models;

// Re-export the main types for convenience
pub use client::{
    Client, CreateDeploymentError, DeleteDeploymentError, GetConnectionStringError,
    GetDeploymentError, PullImageError,
};
