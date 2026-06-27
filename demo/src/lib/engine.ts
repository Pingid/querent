import type { Cache, Column, DataType, DialectKind, Table } from '../querent'

/** Which in-browser database backs the playground. */
export type EngineKind = 'sqlite' | 'duckdb'

export type QueryResult = {
  columns: string[]
  rows: string[][]
  rowCount: number
  elapsedMs: number
}

/** A connected, pre-seeded database with query + introspection + teardown. */
export type Db = {
  kind: EngineKind
  /** Completion dialect to hand the querent engine for this database. */
  dialect: DialectKind
  query: (sql: string) => Promise<QueryResult>
  introspect: () => Promise<Cache>
  close: () => Promise<void>
}

/** Static metadata per engine, used for UI labels and the default selection. */
export const ENGINES: Record<EngineKind, { label: string; dialect: DialectKind }> = {
  sqlite: { label: 'SQLite', dialect: 'sqlite' },
  duckdb: { label: 'DuckDB', dialect: 'postgres' },
}

/** One flattened introspection row (a column, with its owning table's info). */
export type IntrospectRow = {
  table_catalog: string
  table_schema: string
  table_name: string
  table_type: string
  column_name: string
  data_type: string
}

/** Fold flat introspection rows into the engine's schema cache shape. */
export function cacheFromRows(rows: IntrospectRow[]): Cache {
  const tables = new Map<string, Table>()
  const columns: Column[] = []

  for (const r of rows) {
    const key = `${r.table_schema}.${r.table_name}`
    if (!tables.has(key)) {
      tables.set(key, {
        table_name: r.table_name,
        schema_name: r.table_schema,
        database_name: r.table_catalog,
        table_type: r.table_type.toLowerCase() === 'view' ? 'view' : 'table',
      })
    }
    columns.push({
      column_name: r.column_name,
      table_name: r.table_name,
      schema_name: r.table_schema,
      database_name: r.table_catalog,
      data_type: dataType(r.data_type),
    })
  }

  return { tables: [...tables.values()], columns, functions: [] }
}

/** Render any DB cell as a display string; keeps BigInt/Date/JSON readable. */
export function cell(value: unknown): string {
  if (value == null) return ''
  if (typeof value === 'bigint') return value.toString()
  if (value instanceof Date) return value.toISOString()
  if (value instanceof Uint8Array) return `[${value.length} bytes]`
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

const ALIASES: Record<string, DataType> = {
  int: 'integer',
  integer: 'integer',
  int4: 'integer',
  bigint: 'bigint',
  int8: 'bigint',
  real: 'double',
  double: 'double',
  float8: 'double',
  'double precision': 'double',
  float: 'float',
  float4: 'float',
  decimal: 'decimal',
  numeric: 'decimal',
  text: 'text',
  clob: 'text',
  varchar: 'varchar',
  char: 'varchar',
  string: 'varchar',
  bool: 'boolean',
  boolean: 'boolean',
  date: 'date',
  datetime: 'timestamp',
  timestamp: 'timestamp',
  time: 'time',
  blob: 'bytes',
  bytes: 'bytes',
  json: 'json',
  uuid: 'uuid',
}

const PREFIXES: DataType[] = [
  'timestamp',
  'boolean',
  'integer',
  'bigint',
  'varchar',
  'decimal',
  'double',
  'float',
  'text',
  'date',
  'time',
  'json',
  'bytes',
  'uuid',
]

/** Map a DB type name onto the engine's coarse DataType union. */
export function dataType(raw: string): DataType {
  const t = raw.toLowerCase().trim()
  if (ALIASES[t]) return ALIASES[t]
  return PREFIXES.find((p) => t.startsWith(p)) ?? 'unknown'
}
