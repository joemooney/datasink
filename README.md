# DataSink

A gRPC-based database service written in Rust that provides a flexible interface for database operations with support for streaming query results.

## Features

- **gRPC API** for database operations
- **Streaming support** for query results
- **Database abstraction** layer for easy addition of new backends
- **SQLite** support (with room for PostgreSQL, MySQL, etc.)
- **Type-safe** conversions between protobuf and internal types
- **Batch operations** for efficient data insertion
- **Schema files** for defining database structure and initial data
- **Multi-database support** (coming soon)

## Quick Start

### Prerequisites

- Rust 1.70 or later
- Protocol Buffers compiler (`protoc`)

### Using the CLI

DataSink now includes a comprehensive command-line interface:

```bash
# Show help
datasink --help

# Start the server
datasink server start
datasink server start -b 0.0.0.0:8080  # Custom address

# Create a database
datasink server create-database mydb.db

# Create a database from a schema file
datasink server create-from-schema schemas/example.schema
datasink server create-from-schema schemas/blog.schema -d myblog

# Create a table (server must be running)
datasink server create-table users '[{"name":"id","type":"INTEGER","primary_key":true},{"name":"name","type":"TEXT"}]'

# Insert data
datasink insert users '{"id":1,"name":"Alice"}'

# Query data
datasink query "SELECT * FROM users"
datasink query "SELECT * FROM users" -f json  # JSON output
datasink query "SELECT * FROM users" -f csv   # CSV output

# Update data
datasink update users '{"name":"Alice Smith"}' -w "id = 1"

# Delete data
datasink delete users -w "id = 1"
```

### Running the Example Client

```bash
# In another terminal, run the example client
cargo run --example client
```

## API Operations

The service supports the following operations:

- **CreateTable**: Create a new table with specified columns
- **DropTable**: Drop an existing table
- **Insert**: Insert a single row
- **BatchInsert**: Insert multiple rows efficiently
- **Update**: Update rows matching a condition
- **Delete**: Delete rows matching a condition
- **Query**: Execute SQL queries with streaming results

## Architecture

```
src/
├── main.rs           # Entry point and server setup
├── db/               # Database abstraction layer
│   ├── mod.rs        # Module exports
│   ├── traits.rs     # Database trait definition
│   ├── sqlite.rs     # SQLite implementation
│   └── error.rs      # Database error types
├── grpc/             # gRPC service layer
│   ├── mod.rs        # Module exports
│   ├── service.rs    # Service implementation
│   └── conversions.rs # Proto <-> internal type conversions
└── proto/            # Protocol buffer definitions
    └── datasink.proto
```

## Schema Files

DataSink supports TOML-based schema files for defining database structure and initial data. Schema files allow you to:

- Define multiple tables with columns and constraints
- Specify initial seed data
- Version your database schema
- Create consistent development/test databases

Example schema file structure:

```toml
[database]
name = "myapp"
description = "My application database"
version = "1.0.0"

[[tables]]
name = "users"
description = "User accounts"

[[tables.columns]]
name = "id"
type = "INTEGER"
primary_key = true
auto_increment = true

[[tables.columns]]
name = "email"
type = "TEXT"
nullable = false
unique = true

# Initial data
[[data.users]]
email = "admin@example.com"
```

See the `schemas/` directory for complete examples:
- `default.schema` - Minimal default schema
- `example.schema` - E-commerce database with users, products, and orders
- `blog.schema` - Blog system with authors and posts
- `PostIt.schema` - Post-it notes system with tags and priorities

## Environment Variables

- `DATABASE_URL`: Database connection string (default: `sqlite://datasink.db`)
- `SERVER_ADDRESS`: gRPC server address (default: `127.0.0.1:50051`)

## Development

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy

# Build for release
cargo build --release
```

## Adding New Database Backends

1. Create a new module in `src/db/` (e.g., `postgres.rs`)
2. Implement the `Database` trait for your backend
3. Update `src/db/mod.rs` to export the new implementation
4. Add necessary dependencies to `Cargo.toml`

## License

This project is open source.