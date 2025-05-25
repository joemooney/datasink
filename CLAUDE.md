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

4. **Schema System** (`src/schema/`)
   - `mod.rs` - Schema data structures
   - `parser.rs` - TOML schema file parsing and processing

5. **CLI Interface** (`src/cli/`)
   - `mod.rs` - Command definitions using clap
   - `commands.rs` - Command implementations

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
- `cargo run -- server create-from-schema schemas/example.schema` - Create database from schema file
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

## Development History

This project was developed interactively with Claude. Here's the chronological history of major features added:

### 1. Initial Setup
**Request**: "this is a bare bones rust application. I want to build this out to be a server that receives database operations and can create new tables, insert, update, delete, and query. The requests should be GRPC. I'm guessing for a query we should be able to stream responses. Initially, we will use sqlite database, but we will want to be flexible so that we can support other databases in the future."

**What was built**:
- gRPC service definition with all CRUD operations
- Database abstraction trait for multiple backend support
- SQLite implementation
- Streaming query support
- Basic project structure with proper module organization

### 2. CLI Interface
**Request**: "please add '--help'. I suggest using structopt or maybe clap, and have subcommands for server, query, insert, update, delete. The server subcommand should have subcommands to start, stop, create table, create database."

**What was added**:
- Comprehensive CLI using clap v4
- Server subcommands: start, stop, create-table, create-database
- Client commands: query, insert, update, delete
- Global options and help text
- Support for different output formats (json, csv, table)

### 3. Schema File Support
**Request**: "I would like to be able to create a database based on a .schema file. This file would have the details necessary to create a some tables and maybe populate some initial data. There could be a command to create a database providing a .schema file. Please create an example .schema file. Each database would have an associated .schema file."

**What was implemented**:
- TOML-based schema file format
- Schema parser for table definitions and initial data
- `create-from-schema` command
- Support for column constraints (primary key, unique, nullable, etc.)
- Foreign key definitions (documentation only)
- Default values and auto-increment support
- Example schemas: default.schema, example.schema, blog.schema

### 4. PostIt Schema
**Request**: "I want to add a schema file called 'PostIt.schema' to store post-it notes. There is a title, description, creation date, status, priority, URL, and a list of tags. The status is open, closed, working, archived. priority is low, medium, high, critical."

**What was created**:
- PostIt.schema with three tables (notes, tags, note_tags)
- Many-to-many relationship between notes and tags
- Enum-like fields for status and priority
- Sample data including notes and tag assignments
- Example queries for common operations

### 5. Help Examples
**Request**: "Please add examples of invoking commands to the help for each subcommand (except for --help itself)"

**What was added**:
- Examples in help text for every command
- 2-3 practical examples per command
- Shows different options and use cases
- Makes CLI self-documenting

### 6. Proto Documentation
**Request**: "Please update the .proto file with detailed comments"

**What was done**:
- Comprehensive comments for all service methods
- Detailed message and field documentation
- Usage examples and warnings
- Type information and constraints

### 7. GitHub Integration
**Request**: "I have a GitHub account the the 'gh' CLI. Please create a repo and push all the changes"

**Result**: Repository created at https://github.com/joemooney/datasink

### Key Design Decisions Made During Development

1. **Database Abstraction**: Used trait-based design to allow easy addition of new database backends
2. **Streaming Queries**: Implemented streaming for large result sets to avoid memory issues
3. **Schema Format**: Chose TOML for human-readable, version-controllable schema definitions
4. **CLI Structure**: Used subcommands to logically group operations
5. **Error Handling**: Comprehensive error types with helpful messages
6. **Direct Database Access**: Schema creation works without requiring a running server