import { PGlite } from '@electric-sql/pglite'

import { cell, ENGINES, introspect, type Db, type QueryResult, type Row } from './engine'
import { PGLITE_SEED } from './seed'

/** Boot PGlite (in-memory Postgres), seed it, and return a handle. */
export const create = async (): Promise<Db> => {
  const pg = new PGlite()
  await pg.exec(PGLITE_SEED)
  return {
    kind: 'pglite',
    dialect: ENGINES.pglite.dialect,
    query: (sql) => runQuery(pg, sql),
    introspect: () => readSchema(pg),
    close: () => pg.close(),
  }
}

/** Run a single statement and flatten its result into display strings. */
async function runQuery(pg: PGlite, sql: string): Promise<QueryResult> {
  const start = performance.now()
  const res = await pg.query<Record<string, unknown>>(sql)
  const columns = res.fields.map((f) => f.name)
  const rows = res.rows.map((row) => columns.map((c) => cell(row[c])))
  return { columns, rows, rowCount: rows.length, elapsedMs: performance.now() - start }
}

/** Read the schema via the core's Postgres `pg_catalog` introspection queries. */
function readSchema(pg: PGlite) {
  const rows = async (sql: string): Promise<Row[]> => (await pg.query<Row>(sql)).rows
  return introspect(ENGINES.pglite.dialect, rows)
}
