# SQL Completion Engine

A lightweight, context-aware SQL autocompletion engine. It’s built from composable traits and modular building blocks, designed to work with either live database connections or in-memory schema definitions.

## Core Goals

- **Multi-dialect support** – completion suggestions across all major SQL dialects
- **Portable** – runs in the browser via WASM
- **Efficient** – optimized for performance and low memory usage

## Modules

- **ast**
  Generic sql ast types with visiter implementation
- **complete**
  The central component that combines everything to generate context-aware completions at a given cursor position. It is built from the following parts:
  - **completion**
    Completion and Completion result types with rank and sorting
  - **context**
    Constructs a context from a sql ast and cursor position
  - **provider**
    Implementations for varias completion types including: columns, tables, keywords
- **dialect**
  Encapsulates SQL dialect specifics: keywords, operators, quoting rules
  > keywords are defined in build.rs
- **lex**
  Converts a SQL string into a token stream based on the dialect
- **parse**
  Builds error-tolerant ast from partial or complete queries
- **schema**
  Types for database schema
