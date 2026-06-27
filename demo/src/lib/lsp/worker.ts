/// <reference lib="webworker" />
import { expose } from 'comlink'

import init, { LspProvider, LspServer, type Cache, type DialectKind, type IntrospectionQueries } from '@/querent'

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

let provider: LspProvider
let server: LspServer

const ready = init().then(() => {
  provider = new LspProvider()
  server = new LspServer(provider, { capabilities: {} })
})

export const API = {
  getIntrospectionQueries: async (dialect: DialectKind): Promise<IntrospectionQueries> => {
    await ready
    return await provider.getIntrospectionQueries(dialect)
  },
  setDialect: async (uri: string, dialect: DialectKind) => {
    await ready
    return provider.setDialect(uri, dialect)
  },
  setSchema: async (uri: string, schema: Cache) => {
    await ready
    return provider.setSchema(uri, schema)
  },
}
expose(API)
export type API = typeof API

self.onmessage = async (event: MessageEvent) => {
  await ready
  const msg = event.data

  if ('jsonrpc' in msg) {
    try {
      const response = await server.handle(msg)
      if (response != null) self.postMessage(response)
    } catch (err) {
      // Keep the worker alive: a malformed frame shouldn't kill completions.
      console.error('completion worker failed to handle message', err)
    }
  }
}
