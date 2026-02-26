# MicroKit

A framework for building microservices with Axum, providing database integration, secret fetching, authentication, observability, and API documentation.

WIP: Eventing/Messaging via dapr

## Development (NixOS)

A `flake.nix` and `.envrc` are included. With `direnv` installed, the dev shell
activates automatically on `cd`. To enter it manually:

```bash
nix develop
```

## Crates

- [MicroKit](crates/microkit/README.md) - Core library for building microservices
- [MicroKit CLI](crates/microkit-cli/README.md) - CLI tool for creating and managing services
- [MicroKit Macros](crates/microkit-macros/README.md) - Procedural macros for MicroKit

## Quick Start

Install the CLI:
```bash
cargo install microkit-cli
```

Create a new service:
```bash
mk new my-service
```

Run all services:
```bash
mk run
```

See the [MicroKit CLI](crates/microkit-cli/README.md) for more details.
