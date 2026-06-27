# querent example

A browser SQL playground that runs entirely client-side. It wires three pieces together:

- **In-browser database** — switchable between **SQLite WASM** (default) and **DuckDB WASM**, each pre-seeded with the same small e-commerce schema.
- **querent completion engine** — the `querent-lsp-wasm` crate compiled to WebAssembly, running in a Web Worker as an LSP server.
- **Monaco editor** — connected to the engine through [`monaco-lsp-bridge`](https://github.com/Pingid/monaco-lsp-bridge), with [Shiki](https://shiki.style) providing TextMate-grammar SQL syntax highlighting (`src/lib/monaco.ts`).

The engine switcher in the header swaps the active database. Each engine reports its own dialect to the completion engine (`sqlite` for SQLite, `postgres` for DuckDB), so keyword/function suggestions match the backend while table/column suggestions come from live introspection.

## How it works

```
 Monaco editor ──LSP/JSON-RPC──▶ completions.worker ──▶ querent (wasm)
      ▲                                  ▲
      │ run query                        │ schema (Cache)
      ▼                                  │
   DuckDB WASM ───── introspect ─────────┘
```

1. On load, `boot()` (`src/lib/app.ts`) starts the default engine (SQLite) and runs its seed (`src/lib/seed.ts`). `switchEngine()` tears the current one down and brings up the other.
2. The DB is introspected into the engine's `Cache` shape via a shared mapper (`src/lib/engine.ts`): DuckDB reads `information_schema` (`src/lib/duckdb.ts`); SQLite reads `sqlite_master` + `pragma_table_info` (`src/lib/sqlite.ts`).
3. That schema (and the engine's dialect) is posted to the worker (`src/lsp/completions.worker.ts`), which feeds it to the querent `EngineProvider` keyed by the document URI.
4. Monaco sends `didOpen`/`didChange`/`completion` over JSON-RPC; the worker forwards them to the wasm `WasmLspServer`, which returns context-aware completions.
5. `⌘↵` (or the Run button) executes the statement at the cursor against DuckDB; results render in the bottom panel. After each run the schema is re-introspected so DDL changes show up in completions.

## Develop

```bash
pnpm install
pnpm dev
```

## Rebuild the completion engine

The wasm package in `src/wasm/` is generated from the `querent-lsp-wasm` crate:

```bash
cd ../lsp-wasm
wasm-pack build --target web --out-dir ../example/src/wasm --release
```
