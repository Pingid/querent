import * as duckdb from '@duckdb/duckdb-wasm'

import mvp_worker from '@duckdb/duckdb-wasm/dist/duckdb-browser-mvp.worker.js?url'
import eh_worker from '@duckdb/duckdb-wasm/dist/duckdb-browser-eh.worker.js?url'
import duckdb_wasm_eh from '@duckdb/duckdb-wasm/dist/duckdb-eh.wasm?url'
import duckdb_wasm from '@duckdb/duckdb-wasm/dist/duckdb-mvp.wasm?url'

import { cell, ENGINES, introspect, type Db, type QueryResult, type Row } from './engine'
import { DUCKDB_SEED } from './seed'

const BUNDLES: duckdb.DuckDBBundles = {
  mvp: { mainModule: duckdb_wasm, mainWorker: mvp_worker },
  eh: { mainModule: duckdb_wasm_eh, mainWorker: eh_worker },
}

/** Boot DuckDB-wasm, seed the in-browser database, and return a handle. */
export const create = async (): Promise<Db> => {
  const conn = await initialize()
  return {
    kind: 'duckdb',
    dialect: ENGINES.duckdb.dialect,
    query: (sql) => runQuery(conn, sql),
    introspect: () => readSchema(conn),
    close: () => conn.close(),
  }
}

let db: duckdb.AsyncDuckDB | null = null
const initialize = async () => {
  if (db) return db.connect()
  const bundle = await duckdb.selectBundle(BUNDLES)
  const worker = new Worker(bundle.mainWorker!)
  db = new duckdb.AsyncDuckDB(new duckdb.ConsoleLogger(), worker)
  await db.instantiate(bundle.mainModule, bundle.pthreadWorker)
  const conn = await db.connect()
  await conn.query(DUCKDB_SEED)
  return conn
}

type Conn = duckdb.AsyncDuckDBConnection

/** Run a single statement and materialise an Arrow result into plain strings. */
async function runQuery(conn: Conn, sql: string): Promise<QueryResult> {
  const start = performance.now()
  const table = await conn.query(sql)
  const columns = table.schema.fields.map((f) => f.name)
  const rows = table.toArray().map((row: any) => columns.map((c) => cell(row[c])))
  return { columns, rows, rowCount: rows.length, elapsedMs: performance.now() - start }
}

/** Read the schema via the core's Postgres introspection queries (DuckDB speaks pg_catalog). */
function readSchema(conn: Conn) {
  const rows = async (sql: string): Promise<Row[]> =>
    (await conn.query(sql)).toArray().map((r: any) => ({ ...r }))
  return introspect(ENGINES.duckdb.dialect, rows)
}
