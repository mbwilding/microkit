# microkit

Core library for building microservices with Axum, providing database integration, secret fetching, authentication, observability, and API documentation.

WIP: Eventing/Messaging via dapr

## Installation

```toml
[dependencies]
microkit = "*"
```

## Features

- `tracing` - Structured logging with tracing (enabled by default)
- `database` - SeaORM database integration (enabled by default)
- `auth` - OIDC authentication support (enabled by default)
- `dapr` - Dapr integration for microservices (enabled by default)
- `health-checks` - Health check endpoints at `/status/ready` and `/status/live` (enabled by default)
- `swagger` - Swagger UI documentation (enabled by default)
- `redoc` - Redoc documentation (opt-in)
- `rapidoc` - Rapidoc documentation (opt-in)
- `scalar` - Scalar documentation (opt-in)
- `otel` - OpenTelemetry support for metrics and tracing (enabled by default)

## Basic Usage

See the [Template API](../../template/crates/api/src/lib.rs).

## Configuration

See the [Template Config](../../template/microkit.yml).

## Tooling

See the [MicroKit CLI](../microkit-cli/README.md) for scaffolding tools.
