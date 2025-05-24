# DataSink gRPC API Documentation

## Overview

DataSink provides a gRPC-based interface for database operations. The service is designed to be database-agnostic, currently supporting SQLite with the ability to add other backends.

## Service Definition

```protobuf
service DataSink {
    rpc CreateTable(CreateTableRequest) returns (CreateTableResponse);
    rpc DropTable(DropTableRequest) returns (DropTableResponse);
    rpc Insert(InsertRequest) returns (InsertResponse);
    rpc Update(UpdateRequest) returns (UpdateResponse);
    rpc Delete(DeleteRequest) returns (DeleteResponse);
    rpc Query(QueryRequest) returns (stream QueryResponse);
    rpc BatchInsert(BatchInsertRequest) returns (BatchInsertResponse);
}
```

## Data Types

The following data types are supported:

- `INTEGER` - 64-bit signed integer
- `REAL` - Double-precision floating point
- `TEXT` - Variable-length text string
- `BLOB` - Binary data
- `BOOLEAN` - Boolean value (stored as INTEGER 0 or 1)
- `TIMESTAMP` - Unix timestamp (stored as INTEGER)

## API Methods

### CreateTable

Creates a new table with the specified schema.

**Request:**
```json
{
  "table_name": "users",
  "columns": [
    {
      "name": "id",
      "type": "INTEGER",
      "nullable": false,
      "primary_key": true,
      "unique": true,
      "default_value": ""
    },
    {
      "name": "email",
      "type": "TEXT",
      "nullable": false,
      "primary_key": false,
      "unique": true,
      "default_value": ""
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "message": "Table 'users' created successfully"
}
```

### Insert

Inserts a single row into a table.

**Request:**
```json
{
  "table_name": "users",
  "values": {
    "id": {"int_value": 1},
    "email": {"text_value": "user@example.com"},
    "created_at": {"timestamp_value": 1700000000}
  }
}
```

**Response:**
```json
{
  "success": true,
  "message": "Insert successful",
  "inserted_id": 1
}
```

### Query

Executes a SQL query and returns results as a stream.

**Request:**
```json
{
  "sql": "SELECT * FROM users WHERE id > ?",
  "parameters": {
    "min_id": {"int_value": 10}
  }
}
```

**Response (Streamed):**
First message contains column metadata:
```json
{
  "result_set": {
    "columns": [
      {"name": "id", "type": "INTEGER"},
      {"name": "email", "type": "TEXT"}
    ],
    "rows": []
  }
}
```

Subsequent messages contain data rows:
```json
{
  "result_set": {
    "columns": [],
    "rows": [
      {
        "values": [
          {"int_value": 11},
          {"text_value": "user11@example.com"}
        ]
      }
    ]
  }
}
```

### Update

Updates existing rows that match the WHERE clause.

**Request:**
```json
{
  "table_name": "users",
  "values": {
    "email": {"text_value": "newemail@example.com"}
  },
  "where_clause": "id = 1"
}
```

**Response:**
```json
{
  "success": true,
  "message": "1 rows updated",
  "affected_rows": 1
}
```

### Delete

Deletes rows that match the WHERE clause.

**Request:**
```json
{
  "table_name": "users",
  "where_clause": "id > 100"
}
```

**Response:**
```json
{
  "success": true,
  "message": "5 rows deleted",
  "affected_rows": 5
}
```

### BatchInsert

Efficiently inserts multiple rows in a single transaction.

**Request:**
```json
{
  "table_name": "users",
  "rows": [
    {
      "values": {
        "id": {"int_value": 2},
        "email": {"text_value": "user2@example.com"}
      }
    },
    {
      "values": {
        "id": {"int_value": 3},
        "email": {"text_value": "user3@example.com"}
      }
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "message": "2 rows inserted",
  "inserted_count": 2
}
```

## Value Types

Values in DataSink use a union type to ensure type safety:

```protobuf
message Value {
    oneof value {
        int64 int_value = 1;
        double real_value = 2;
        string text_value = 3;
        bytes blob_value = 4;
        bool bool_value = 5;
        int64 timestamp_value = 6;
        bool null_value = 7;
    }
}
```

## Error Handling

All operations return appropriate gRPC status codes on failure:

- `ALREADY_EXISTS` - Table already exists
- `NOT_FOUND` - Table not found
- `INVALID_ARGUMENT` - Invalid query or parameters
- `UNAVAILABLE` - Database connection error
- `INTERNAL` - Other database errors

Query operations can also return errors in the response stream:

```json
{
  "error": {
    "code": "QUERY_ERROR",
    "message": "Syntax error in SQL query"
  }
}
```

## Best Practices

1. **Use Parameterized Queries**: Always use parameters for user input to prevent SQL injection
2. **Stream Large Results**: The Query RPC uses streaming to efficiently handle large result sets
3. **Batch Operations**: Use BatchInsert for inserting multiple rows efficiently
4. **WHERE Clauses**: Be careful with empty WHERE clauses as they affect/delete all rows
5. **Connection Management**: The gRPC client handles connection pooling automatically