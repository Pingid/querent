import sqlite3InitModule, { type Database } from '@sqlite.org/sqlite-wasm'

import { cacheFromRows, cell, ENGINES, type Db, type QueryResult } from './engine'
import { SQLITE_SEED } from './seed'

// The wasm runtime is initialised once and shared across (re)connections.
const runtime = sqlite3InitModule()

let db: Database | null = null
const initialize = async () => {
  if (db) return db
  const sqlite3 = await runtime
  db = new sqlite3.oo1.DB(':memory:', 'c')
  db.exec(SQLITE_SEED)
  return db
}

/** Boot SQLite-wasm (in-memory), seed it, and return a handle. */
export const create = async (): Promise<Db> => {
  const db = await initialize()
  return {
    kind: 'sqlite',
    dialect: ENGINES.sqlite.dialect,
    query: async (sql) => runQuery(db, sql),
    introspect: async () => introspect(db),
    close: async () => {},
  }
}

type DB = Awaited<ReturnType<typeof sqlite3InitModule>>['oo1']['DB']['prototype']

/** Execute one statement, capturing column names even for empty result sets. */
function runQuery(db: DB, sql: string): QueryResult {
  const start = performance.now()
  const columns: string[] = []
  const rows = db.exec({
    sql,
    rowMode: 'array',
    returnValue: 'resultRows',
    columnNames: columns,
  }) as unknown[][]
  return {
    columns,
    rows: rows.map((row) => row.map(cell)),
    rowCount: rows.length,
    elapsedMs: performance.now() - start,
  }
}

/** Read `sqlite_master` + `pragma_table_info` into the schema cache. */
function introspect(db: DB) {
  const rows = db.exec({
    sql: QUERY_SCHEMA,
    rowMode: 'object',
    returnValue: 'resultRows',
  }) as any[]
  return cacheFromRows(rows.map((r) => ({ ...r })))
}

const QUERY_SCHEMA = `
SELECT
  'memory' AS table_catalog,
  'main'   AS table_schema,
  m.name   AS table_name,
  CASE WHEN m.type = 'view' THEN 'view' ELSE 'table' END AS table_type,
  p.name   AS column_name,
  p.type   AS data_type
FROM sqlite_master AS m
JOIN pragma_table_info(m.name) AS p
WHERE m.type IN ('table', 'view')
  AND m.name NOT LIKE 'sqlite_%'
ORDER BY m.name, p.cid`
