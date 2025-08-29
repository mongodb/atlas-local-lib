# Contributing to MongoDB Atlas Local Library

We welcome contributions to the MongoDB Atlas Local Library! This guide will help you get started with contributing to the project.

## Prerequisites

Before contributing, make sure you have:

- **Docker**: Docker must be installed and running on your system
- **Rust**: Rust 1.70 or later (edition 2024)

## Quick Start

1. **Fork the repository**
2. **Clone your fork**:
   ```bash
   git clone https://github.com/your-username/atlas-local-lib.git
   cd atlas-local-lib
   ```
3. **Create a feature branch**: `git checkout -b feature/amazing-feature`
4. **Set up your environment**:
   ```bash
   # Build the project
   cargo build
   
   # Verify Docker is running and run tests
   docker info
   cargo test
   ```

## Development Workflow

### Making Changes

1. Create a new branch for your feature or bug fix
2. Make your changes in small, logical commits
3. Add tests for any new functionality
4. Before committing, run the complete validation suite:

```bash
# Format your code
cargo fmt

# Run linting
cargo clippy

# Run all tests
cargo test

# Check that code compiles
cargo check
```

### Code Style

This project follows standard Rust conventions:

- Use `cargo fmt` to format code
- Use `cargo clippy` to catch common mistakes
- Write comprehensive tests for new functionality
- Document all public APIs

### Testing

Since this library manages Docker containers, ensure Docker is running before executing tests:

```bash
# Verify Docker is available
docker info

# Run the full test suite
cargo test
```

### Documentation

```bash
# Generate documentation
cargo doc

# Generate and open documentation in browser
cargo doc --open
```

## License and Security Scanning

We use additional tools to ensure license compliance and security:

### cargo-deny

We use [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) to scan for allowed licenses and security advisories:

```sh
cargo deny check
```

This verifies that all dependencies use acceptable licenses and checks for known security vulnerabilities.

### cargo-about

We use [`cargo-about`](https://github.com/EmbarkStudios/cargo-about) to generate our third-party licenses notice:

```sh
cargo about generate about.hbs > license.html
```

This generates an HTML file containing all third-party license information for our dependencies.

## Submitting Your Changes

1. **Commit your changes**: `git commit -m 'Add amazing feature'`
2. **Push to your branch**: `git push origin feature/amazing-feature`
3. **Submit a pull request** against the main branch
4. **Provide a clear description** of what your changes do
5. **Link to any relevant issues**

## Questions or Issues?

If you have questions about contributing, feel free to:

1. Check existing issues and discussions
2. Create a new issue for questions
3. Reach out to the maintainers

Thank you for contributing to MongoDB Atlas Local Library!
