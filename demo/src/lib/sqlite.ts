import sqlite3InitModule, { type Database } from '@sqlite.org/sqlite-wasm'

import { cell, ENGINES, introspect, type Db, type QueryResult, type Row } from './engine'
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
    introspect: () => readSchema(db),
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

/** Read the schema via the core's SQLite introspection queries. */
function readSchema(db: DB) {
  const rows = (sql: string) =>
    db.exec({ sql, rowMode: 'object', returnValue: 'resultRows' }) as Row[]
  return introspect(ENGINES.sqlite.dialect, rows)
}
