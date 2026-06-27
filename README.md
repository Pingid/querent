# SQL Completion Engine

A lightweight, context-aware SQL autocompletion engine. It is built from composable traits and modular building blocks, and works with either live database connections or in-memory schema definitions.

## Scope

Completion support is largely restricted to read queries. `SELECT` statements are the focus: the engine resolves tables, columns, aliases, and scope across the full clause set — `WITH`/CTEs (including recursive), `FROM`, joins, `WHERE`, `GROUP BY`/`HAVING`, window functions, `ORDER BY`, and `LIMIT`/`OFFSET` — along with functions, operators, subqueries, and set operations.

`INSERT`, `UPDATE`, and `DELETE` parse, but completion coverage inside them is limited (for example, column-list and `SET` completions are not yet offered). DDL is not a target.

## Core Goals

- **Multi-dialect support** – completion suggestions across major SQL dialects
- **Portable** – runs in the browser via WASM
- **Efficient** – optimized for performance and low memory usage
