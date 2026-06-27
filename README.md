# Querent

Querent is a local SQL completion engine for editor integrations.

It analyzes the query around the cursor, resolves visible schema and aliases, and returns ranked completions for keywords, tables, columns, functions, and operators.

The goal is to provide useful SQL completion without requiring a direct database connection. Integrations provide a dialect and a schema cache, either from live database introspection or from an in-memory definition. This keeps the engine portable, simple to embed, and practical to run in WebAssembly.

## What It Supports

Querent is optimized for read queries. `SELECT` is the primary target, including:

- `WITH` and recursive CTEs
- `FROM`, joins, aliases, and table functions
- `WHERE`, `GROUP BY`, `HAVING`, and window expressions
- `ORDER BY`, `LIMIT`, `OFFSET`, subqueries, and set operations
- context-aware function, operator, table, and column suggestions

`INSERT`, `UPDATE`, and `DELETE` parse, but completion coverage inside them is limited. DDL is not a target.

Built-in dialects currently cover ANSI SQL, PostgreSQL, and SQLite.

## Core Architecture

Querent is split into small layers that can be reused independently:

- **Dialect specs** define keyword, operator, quoting, casing, function, and follow-rule behavior.
- **Lexer and parser** build a cursor-aware SQL statement without requiring a full database connection.
- **Context analysis** identifies the cursor token, active clause, visible tables, aliases, CTEs, columns, and expected data type.
- **Providers** generate candidate completions from the context: keywords, tables, columns, functions, and operators.
- **Rankers** score candidates using exact, prefix, fuzzy, semantic, source, qualifier, join-key, and type-compatibility signals.
- **Adapters** expose the engine directly, through a small LSP server, or through a WASM LSP wrapper.

The default completion flow is:

```text
Content + cursor
  -> lex
  -> parse statement at cursor
  -> build completion context
  -> collect provider candidates
  -> rank candidates
  -> return replacement-aware completions
```

## Workspace

- `core/` — dialects, lexer, parser, schema cache, completion context, providers, rankers, and the default engine.
- `lsp/` — JSON-RPC/LSP request handling around the core completion provider trait.
- `lsp-wasm/` — WebAssembly bindings for browser-hosted LSP completion.
- `demo/` — Monaco-based browser demo backed by SQLite WASM or DuckDB WASM.
- `pat/` — internal pattern utilities used by the parser.
