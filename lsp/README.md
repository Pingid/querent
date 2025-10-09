# Querent LSP Server

A WebSocket-based Language Server Protocol (LSP) server for SQL, powered by the querent-core engine.

## Features

- **WebSocket Server**: Lightweight WebSocket server listening on `ws://127.0.0.1:9001`
- **SQL Completions**: Intelligent SQL autocompletion including:
  - Keywords (SELECT, FROM, WHERE, etc.)
  - Tables from catalog
  - Columns from tables
  - Functions
  - Operators
- **Document Synchronization**: Tracks open documents and their changes
- **PostgreSQL Dialect**: Currently supports PostgreSQL SQL dialect

## Architecture

The LSP server integrates with `querent-core` to provide:
- **Engine**: SQL parsing and completion engine
- **Catalog**: In-memory catalog for database schema metadata
- **Doc**: Document model with cursor tracking
- **Dialect**: SQL dialect specifications (Postgres)

## Protocol Support

Implements the following LSP methods:
- `initialize` - Initialize the server with capabilities
- `textDocument/didOpen` - Handle document open events
- `textDocument/didChange` - Handle document change events
- `textDocument/completion` - Provide completions at cursor position
- `shutdown` - Gracefully shutdown the server

## Running the Server

```bash
cargo run -p querent-lsp
```

The server will start listening on `ws://127.0.0.1:9001`

## Message Format

The server uses JSON-RPC 2.0 over WebSocket. Example completion request:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "textDocument/completion",
  "params": {
    "textDocument": {
      "uri": "file:///path/to/query.sql"
    },
    "position": {
      "line": 0,
      "character": 10
    }
  }
}
```

## Dependencies

- `tungstenite` - Lightweight WebSocket implementation
- `serde` / `serde_json` - JSON serialization
- `futures` - Async runtime
- `querent-core` - SQL parsing and completion engine
