# MicroKit Template

This is the MicroKit template for building microservices. When you create a new service with `mk new <service-name>`, this template is used to generate a standalone project with all the necessary structure and configuration.

## Project Structure

```
.
├── crates/
│   ├── api/           # Main API service
│   ├── entities/      # Database entities (SeaORM models)
│   └── migrations/    # Database migrations
├── dapr/              # Dapr component configurations
├── microkit.yml       # MicroKit service configuration
├── dapr.yaml          # Dapr multi-app run configuration
└── docker-compose.yml # Local development infrastructure services (PostgreSQL, RabbitMQ, Aspire Dashboard)
```

## Configuration: microkit.yml

The `microkit.yml` file is the central configuration for your MicroKit service. All configuration options correspond to the `Config` struct in the MicroKit library.

### Core Settings

```yaml
service_name: AwesomeService  # Required: Name of your service
service_desc: The most awesome service  # Optional: Description for API documentation
host: localhost  # Optional: Host to bind to (default: localhost)
log_level: info  # Optional: Logging level (trace, debug, info, warn, error)
port_offset: 0   # Optional: Port offset for when you are running multiple services
```

### Database Configuration

When using the `database` feature:

```yaml
database_url: postgres://postgres:Developer01@localhost  # Optional: Database connection URL
database_name: awesomeservice  # Optional: Database name to connect to
database_drop: false  # Optional: Drop and recreate database on startup (useful for development)
```

### OpenTelemetry Configuration

When using the `otel` feature:

```yaml
otel:
  url: http://localhost:4317  # Required: OTLP gRPC endpoint URL
  token: null  # Optional: Authentication token for OTLP endpoint
```

### Authentication Configuration

When using the `auth` feature, add OIDC configuration:

```yaml
auth:
  issuer: https://cognito-idp.{region}.amazonaws.com/{userPoolId}  # Required: OIDC issuer URL
  jwks_uri: https://cognito-idp.{region}.amazonaws.com/{userPoolId}/.well-known/jwks.json  # Required: JWKS endpoint
  audience: your-client-id  # Optional: Expected audience/client ID for token validation
  client_id: your-client-id  # Optional: Client ID for documentation
  client_secret: your-secret  # Optional: Client secret (store in config-private.yml)
  scopes:  # Optional: Default scopes for documentation
    - openid
    - profile
    - email
```

**Note:** For sensitive values like `client_secret`, create a separate `config-private.yml` file that is not committed to version control.

## Port Configuration

MicroKit uses predefined port bases for different service types:

- **API**: Base port 50000
- **Client**: Base port 60000

> Client not yet implemented

The `port_offset` configuration allows you to run multiple services simultaneously. For example, with `port_offset: 0`, the API runs on port 9000, and with `port_offset: 1`, it runs on port 9001. When omitted it'll default to port `80` for when hosting on infrastructure.
Ideally you'd have a reverse proxy dealing with TLS to expose a https endpoint.

## Getting Started

### Prerequisites

1. Install the MicroKit CLI:

   ```bash
   cargo install microkit-cli
   ```

2. Start infrastructure services:

   ```bash
   mk setup
   ```

   This starts:
   - PostgreSQL on port 5432
   - RabbitMQ on ports 5672 (AMQP) and 15672 (UI)
   - Aspire Dashboard on ports 18888 (UI) and 4317 (OTLP)

### Running the Service

Run all services with Dapr:

```bash
mk all
```

Or run a specific binary:

```bash
mk run api
```

### Database Operations

Generate entities from database schema:

```bash
mk db entity
```

Create a new migration:

```bash
mk db migrate add_users_table
```

Drop and recreate database:

```bash
mk db fresh
```

## Building Your Service

The template uses the MicroKit builder pattern. See `crates/api/src/lib.rs` for the default configuration:

> As you enable and disable features, some of these will become unavailable

```rust
MicroKit::builder()
    .await?
    .with_logging()                            // Enable structured logging
    .with_database()                           // Enable database connection
    .with_router()                             // Enable HTTP router
    .with_dapr()                               // Enable Dapr integration
    .with_auth()                               // Enable OIDC authentication
    .with_health_checks()                      // Add /status/ready and /status/live
    .with_otel()                               // Enable OpenTelemetry
    .with_migrations::<migrations::Migrator>() // Run migrations on startup
    .with_endpoints(endpoints::init_endpoints) // Register endpoints
    .build()
    .await?
    .start(ServicePort::Api)                   // Sets the base port
    .await
```

## Features

The template includes all MicroKit features by default:

- `tracing` - Structured logging with tracing
- `database` - SeaORM database integration
- `auth` - OIDC authentication support
- `dapr` - Dapr integration for microservices
- `health-checks` - Health check endpoints
- `swagger` - Swagger UI documentation at `/swagger`
- `otel` - OpenTelemetry support for metrics and tracing

## Testing

Run tests with database mocking:

```bash
cd crates/api
cargo test --features mock
```

## Documentation

When running the service, API documentation is available at:

- Swagger UI: `http://localhost:50000/swagger`
- Health checks: `http://localhost:50000/status/ready` and `http://localhost:50000/status/live`

Add the port offset to the port number to calculate the correct one.

## Observability

The Aspire Dashboard provides:

- Distributed tracing visualization
- Metrics and logs
- Service dependencies

Access it at: `http://localhost:18888`

## Dapr Integration

The template includes Dapr configuration in `dapr.yaml` for multi-app runs. You can add more services or configure Dapr components in the `dapr/` directory.

## Learn More

- [MicroKit Documentation](https://github.com/mbwilding/microkit/tree/main/crates/microkit/README.md)
- [MicroKit CLI](https://github.com/mbwilding/microkit/tree/main/crates/microkit-cli/README.md)
