/// <reference lib="webworker" />
import init, { EngineProvider, WasmLspServer } from '../querent/index.js'
import type { Cache, DialectKind } from '../querent/index.js'

/**
 * Web Worker hosting the querent SQL completion engine as an LSP server.
 *
 * It speaks two message shapes:
 *  - JSON-RPC frames (from monaco-lsp-bridge) -> forwarded to the wasm server.
 *  - `SchemaMessage` control frames -> update the engine's per-document schema.
 */
export type SchemaMessage = {
  control: 'schema'
  uri: string
  dialect: DialectKind
  cache: Cache
}

let provider: EngineProvider
let server: WasmLspServer

const ready = init().then(() => {
  provider = new EngineProvider()
  server = new WasmLspServer(provider)
})

self.onmessage = async (event: MessageEvent) => {
  await ready
  const msg = event.data

  if (isSchemaMessage(msg)) {
    provider.set_dialect(msg.uri, msg.dialect)
    provider.set_schema(msg.uri, msg.cache)
    return
  }

  try {
    const response = await server.handle(msg)
    if (response != null) self.postMessage(response)
  } catch (err) {
    // Keep the worker alive: a malformed frame shouldn't kill completions.
    console.error('completion worker failed to handle message', err)
  }
}

function isSchemaMessage(msg: unknown): msg is SchemaMessage {
  return !!msg && typeof msg === 'object' && (msg as any).control === 'schema'
}
