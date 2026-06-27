import type { API } from './worker'
import { wrap } from 'comlink'

/** Worker hosting the querent completion engine (an LSP server over wasm). */
export const WORKER = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })

export const provider = wrap<API>(WORKER)
