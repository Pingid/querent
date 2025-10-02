# SQL Completion Engine

A lightweight, context-aware SQL autocompletion engine. It’s built from composable traits and modular building blocks, designed to work with either live database connections or in-memory schema definitions.

## Core Goals

- **Multi-dialect support** – completion suggestions across all major SQL dialects
- **Portable** – runs in the browser via WASM
- **Efficient** – optimized for performance and low memory usage

## Components

- **Dialect**
  Encapsulates SQL dialect specifics: keywords, operators, quoting rules
- **Tokenizer**
  Converts a SQL string into a token stream based on the dialect
- **Parser**
  Error-tolerant; builds an AST from partial or complete queries
- **Catalog**
  Traits and abstractions for accessing database schema metadata
- **Engine**
  The central component that combines everything to generate context-aware completions at a given cursor position.

## Note for agents

- Do not return diffs
