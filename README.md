# MongoDB Atlas Local Library
[![CI](https://github.com/mongodb/atlas-local-lib/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/mongodb/atlas-local-lib/actions/workflows/ci.yml)
[![Coverage Status](https://coveralls.io/repos/github/mongodb/atlas-local-lib/badge.svg?branch=main)](https://coveralls.io/github/mongodb/atlas-local-lib?branch=main)
[![Security Audit](https://github.com/mongodb/atlas-local-lib/actions/workflows/security-audit.yml/badge.svg)](https://github.com/mongodb/atlas-local-lib/actions/workflows/security-audit.yml)

A Rust library for managing MongoDB Atlas Local deployments using Docker. This library provides a high-level interface to interact with MongoDB Atlas Local deployments, making it easy to develop and test applications against a local MongoDB Atlas environment.

## Overview

MongoDB Atlas Local Library simplifies the process of managing MongoDB Atlas Local deployments by providing a Rust-native interface that abstracts away the complexity of Docker container management. Whether you're developing applications that will eventually run against MongoDB Atlas or testing Atlas-specific features locally, this library provides the tools you need.

## Features

- Docker Integration: Seamlessly manages MongoDB Atlas Local deployments through Docker
- Rust Native: Built specifically for Rust applications with idiomatic APIs
- Simple Setup: Easy to integrate into existing Rust projects
- Development Ready: Perfect for local development and testing workflows

## Installation

Add this library to your `Cargo.toml`:

```toml
[dependencies]
atlas-local = "0.0.1"
```

For development and testing, you may also want to include tokio:

```toml
[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
```

## Prerequisites

Before using this library, make sure you have:

- **Docker**: Docker must be installed and running on your system
- **Rust**: Rust 1.85 or later (for edition 2024)

## Feature Flags

This library provides optional feature flags to enable additional functionality:

### `serde`

Enables serialization and deserialization support for the library's models using [serde](https://serde.rs/).

**When to use:** Enable this feature if you need to serialize or deserialize deployment configurations, responses, or other model types (e.g., for storing configurations as JSON or passing data across API boundaries).

```toml
[dependencies]
atlas-local = { version = "0.0.1", features = ["serde"] }
```

### `e2e-tests` ⚠️

**For internal use only.** This feature is used by the maintainers for end-to-end testing and is not part of the public API. It may change or be removed without notice.

## Quick Start

Here's a simple example to get you started:

```rust,no_run
use bollard::Docker;
use atlas_local::Client;
use atlas_local::models::CreateDeploymentOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Docker daemon
    let docker = Docker::connect_with_socket_defaults()?;

    // Create a new MongoDB Atlas Local client
    let client = Client::new(docker);

    // Create a deployment
    client.create_deployment(CreateDeploymentOptions::default()).await?;

    // List the running deployments
    let deployments = client.list_deployments().await.unwrap();

    // Print the deployments
    for deployment in deployments {
        println!("[{}] \t{}", deployment.mongodb_version, deployment.name.unwrap_or_default());
    }

    // Delete the new deployment
    client.delete_deployment("local1234").await?;

    Ok(())
}
```

More examples can be found in `examples/*` and ran using `cargo run --example [example-name]`

## Development

For building, testing, and generating documentation, see the [CONTRIBUTING.md](CONTRIBUTING.md) file which contains detailed instructions for all development commands.

## API Documentation

The complete API documentation is available in the generated docs. Key components include:

- **`Client`**: The main entry point for managing MongoDB Atlas Local deployments
- **Error Types**: Comprehensive error handling for Docker and Atlas operations

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contributing guidelines.

## License

This project is licensed under the Apache License, Version 2.0 (see `LICENSE.txt`). It also makes use of third-party libraries, which are distributed under their own respective licenses (see `LICENSE-3RD-PARTY.txt`).

## Related Projects

- [Bollard](https://crates.io/crates/bollard) - Docker API client for Rust
- [MongoDB Atlas](https://www.mongodb.com/atlas) - MongoDB's cloud database service

## Support

For issues and questions:

1. Check the [documentation](#documentation)
2. Review [existing issues](../../issues)
3. Create a [new issue](../../issues/new) if needed

---
