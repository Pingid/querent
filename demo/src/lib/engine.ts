import { provider } from './lsp'
import type { Cache, DataType, DialectKind, IntrospectionQueries } from '../querent'

/** Which in-browser database backs the playground. */
export type EngineKind = 'sqlite' | 'duckdb' | 'pglite'

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
  pglite: { label: 'PGlite', dialect: 'postgres' },
}

/** A result row keyed by column alias, as returned by an engine's driver. */
export type Row = Record<string, unknown>

/** Per-dialect introspection SQL from the querent core, memoised by dialect. */
const QUERIES = new Map<DialectKind, Promise<IntrospectionQueries>>()
function introspectionQueries(dialect: DialectKind): Promise<IntrospectionQueries> {
  let q = QUERIES.get(dialect)
  if (!q) {
    q = provider.getIntrospectionQueries(dialect)
    QUERIES.set(dialect, q)
  }
  return q
}

/**
 * Read a database's schema into the cache using the querent core's per-dialect
 * introspection queries (`getIntrospectionQueries`) — the single source of truth
 * shared by every engine. `rows` runs one statement and hands back its result
 * rows as plain objects keyed by the query's column aliases.
 */
export async function introspect(dialect: DialectKind, rows: (sql: string) => Row[] | Promise<Row[]>): Promise<Cache> {
  const q = await introspectionQueries(dialect)
  const tables = q.tables ? await rows(q.tables) : []
  const columns = q.columns ? await rows(q.columns) : []
  return {
    tables: tables.map((r) => ({
      table_name: String(r.table_name),
      schema_name: str(r.schema_name),
      database_name: str(r.database_name),
      table_type: str(r.table_type)?.toLowerCase() === 'view' ? 'view' : 'table',
    })),
    columns: columns.map((r) => ({
      column_name: String(r.column_name),
      table_name: str(r.table_name),
      schema_name: str(r.schema_name),
      database_name: str(r.database_name),
      data_type: dataType(String(r.data_type)),
      is_nullable: r.is_nullable == null ? undefined : Boolean(r.is_nullable),
    })),
    functions: [],
  }
}

/** Coerce a nullable cell to an optional string (NULL schema/database columns). */
const str = (value: unknown): string | undefined => (value == null ? undefined : String(value))

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
  // Postgres `format_type` base names (see pglite engine introspection).
  'character varying': 'varchar',
  character: 'varchar',
  bpchar: 'varchar',
  bool: 'boolean',
  boolean: 'boolean',
  date: 'date',
  datetime: 'timestamp',
  timestamp: 'timestamp',
  time: 'time',
  blob: 'bytes',
  bytes: 'bytes',
  bytea: 'bytes',
  json: 'json',
  jsonb: 'json',
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
