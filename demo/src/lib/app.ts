import { DUCKDB_DEFAULT, PGLITE_DEFAULT, SQLITE_DEFAULT } from './seed'
import type { Db, EngineKind, QueryResult } from './engine'
import type { Cache } from '../querent'
import { createStore } from './store'

import { provider } from './lsp'

const FACTORIES: Record<EngineKind, () => Promise<Db>> = {
  sqlite: () => import('./sqlite').then((m) => m.create()),
  duckdb: () => import('./duckdb').then((m) => m.create()),
  pglite: () => import('./pglite').then((m) => m.create()),
}
const PAGES: Record<EngineKind, { uri: string; initial: string }> = {
  sqlite: { uri: 'inmemory://querent/sqlite.sql', initial: SQLITE_DEFAULT },
  duckdb: { uri: 'inmemory://querent/duckdb.sql', initial: DUCKDB_DEFAULT },
  pglite: { uri: 'inmemory://querent/pglite.sql', initial: PGLITE_DEFAULT },
}

export type AppState = {
  engine: EngineKind
  status: 'booting' | 'ready' | 'error'
  bootError?: string
  schema?: Cache
  running: boolean
  result?: QueryResult
  queryError?: string
  page: { uri: string; initial: string }
  activeStatement: string | null
}

// SQLite is shown by default; the header lets you switch to DuckDB or PGlite.
export const store = createStore<AppState>({
  engine: 'sqlite',
  status: 'booting',
  running: false,
  page: { uri: 'inmemory://querent/sqlite.sql', initial: SQLITE_DEFAULT },
  activeStatement: null,
})

let db: Db | null = null
let booted = false

/** Boot the default engine once on startup. */
export async function boot() {
  if (booted) return
  booted = true
  await loadEngine(store.get().engine)
}

/** Tear down the current engine and bring up another (SQLite / DuckDB / PGlite). */
export async function switchEngine(engine: EngineKind) {
  if (engine === store.get().engine && db) return
  const previous = db
  db = null
  store.set({
    engine,
    page: PAGES[engine],
    status: 'booting',
    result: undefined,
    queryError: undefined,
    schema: undefined,
  })
  await previous?.close().catch(() => {})
  await loadEngine(engine)
}

async function loadEngine(engine: EngineKind) {
  try {
    const next = await FACTORIES[engine]()
    if (store.get().engine !== engine) {
      await next.close().catch(() => {})
    }
    db = next
    await syncSchema()
    store.set({ status: 'ready' })
  } catch (error) {
    store.set({ status: 'error', bootError: String(error) })
  }
}

export async function run(sql: string) {
  if (!db || !sql.trim()) return
  store.set({ running: true, queryError: undefined })
  try {
    const result = await db.query(sql)
    store.set({ running: false, result })
    void syncSchema()
  } catch (error) {
    store.set({ running: false, queryError: String(error) })
  }
}

/** Re-read the DB schema and hand it to both the UI and the completion engine. */
async function syncSchema() {
  if (!db) return
  const cache = await db.introspect()
  store.set({ schema: cache })
  await provider.setSchema(store.get().page.uri, cache)
}
