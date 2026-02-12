# MicroKit CLI

CLI tool for creating and managing MicroKit services.

## Installation

```bash
cargo install microkit-cli
```

Or build from source:

```bash
cargo install --path crates/microkit-cli
```

## Usage

The CLI is available as `mk` after installation.

### Create a new service

```bash
mk new <service-name>
```

Options:
- `-d, --description <DESCRIPTION>` - Description of the service
- `-p, --port-offset <PORT_OFFSET>` - Port offset for running multiple services (default: 0)
- `-t, --tag <TAG>` - MicroKit git tag to create the service from

### Setup environment

```bash
mk setup
```

### Run services with Dapr

Run all services:
```bash
mk run
```

Run a specific binary:
```bash
mk run <binary-name>
```

### Database commands

Generate entities from database schema:
```bash
mk db entity
```

Create a new migration:
```bash
mk db migrate <migration-name>
```

Drop and recreate database:
```bash
mk db fresh
```
