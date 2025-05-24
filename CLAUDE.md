# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

DataSink is a gRPC-based database service written in Rust that provides a flexible interface for database operations. It currently supports SQLite but is designed to easily support additional database backends.

## Architecture

### Core Components

1. **gRPC Service Layer** (`src/grpc/`)
   - `service.rs` - Implements the DataSink gRPC service handlers
   - `conversions.rs` - Handles conversions between protobuf and internal types

2. **Database Abstraction Layer** (`src/db/`)
   - `traits.rs` - Defines the `Database` trait for backend abstraction
   - `sqlite.rs` - SQLite implementation of the Database trait
   - `error.rs` - Database-specific error types

3. **Protocol Buffers** (`proto/datasink.proto`)
   - Defines the gRPC service interface and message types
   - Supports streaming responses for query operations

### Key Design Decisions

- **Database Abstraction**: The `Database` trait allows for easy addition of new database backends (PostgreSQL, MySQL, etc.)
- **Streaming Queries**: Query operations return a stream of results, allowing efficient handling of large result sets
- **Type Safety**: Strong typing throughout with conversions between proto and internal types

## Common Development Commands

### Build Commands
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build the project in release mode

### CLI Usage
The application now includes a comprehensive CLI with subcommands:

#### Server Commands
- `cargo run -- server start` - Start the gRPC server
- `cargo run -- server start -b 0.0.0.0:8080` - Start on custom address
- `cargo run -- server create-database mydb.db` - Create a new database
- `cargo run -- server create-table users '[{"name":"id","type":"INTEGER","primary_key":true}]'` - Create a table

#### Client Commands (require server to be running)
- `cargo run -- query "SELECT * FROM users"` - Query data
- `cargo run -- query "SELECT * FROM users" -f json` - Query with JSON output
- `cargo run -- insert users '{"id":1,"name":"Alice"}'` - Insert data
- `cargo run -- update users '{"name":"Alice Smith"}' -w "id = 1"` - Update data
- `cargo run -- delete users -w "id = 1"` - Delete data

#### Global Options
- `-d, --database-url` - Specify database URL (also respects DATABASE_URL env var)
- `-s, --server-address` - Server address for client commands (default: http://127.0.0.1:50051)
- `-v, --verbose` - Enable verbose logging
- `--help` - Show help for any command

### Test Commands
- `cargo test` - Run all tests
- `cargo test [test_name]` - Run a specific test by name
- `cargo test -- --nocapture` - Run tests with output shown

### Lint and Format Commands
- `cargo clippy` - Run the Rust linter
- `cargo fmt` - Format the code
- `cargo fmt -- --check` - Check formatting without making changes

### Other Useful Commands
- `cargo check` - Quickly check for compilation errors without building
- `cargo clean` - Remove build artifacts

## Environment Variables

- `DATABASE_URL` - Database connection string (default: `sqlite://datasink.db`)
- `SERVER_ADDRESS` - gRPC server address (default: `127.0.0.1:50051`)

## Adding New Database Backends

To add support for a new database:

1. Create a new module in `src/db/` (e.g., `postgres.rs`)
2. Implement the `Database` trait for your backend
3. Update `src/db/mod.rs` to export the new implementation
4. Update dependencies in `Cargo.toml`