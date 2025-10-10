import { LanguageClient, LanguageClientOptions, StreamInfo } from 'vscode-languageclient/node'
import ReconnectingWebSocket from 'reconnecting-websocket'
import * as vscode from 'vscode'
import { Duplex } from 'stream'
import WebSocket from 'ws'

import { registerConnections, getConnectionForFile } from './connections'

let client: LanguageClient

export const activate = (context: vscode.ExtensionContext) => {
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'sql' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.sql'),
    },
  }

  client = new LanguageClient('querentSql', 'Querent SQL Language Server', serverOptions, clientOptions)

  client.start().catch((err) => {
    console.error('Failed to start Querent SQL Language Client:', err)
    vscode.window.showErrorMessage(`Failed to start SQL LSP: ${err.message}`)
  })

  // Set up document lifecycle handlers
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => {
      if (document.languageId === 'sql') {
        setEngineForDocument(context, document)
      }
    }),
  )

  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((document) => {
      if (document.languageId === 'sql') {
        removeEngineForDocument(document)
      }
    }),
  )

  // Set engine for already open documents after client starts
  setTimeout(() => {
    vscode.workspace.textDocuments.forEach((document) => {
      if (document.languageId === 'sql') {
        setEngineForDocument(context, document)
      }
    })
  }, 1000)

  // Register commands with callback for connection changes
  registerConnections(context)

  // Export a function to notify when connections change
  context.subscriptions.push(
    vscode.commands.registerCommand('querent.updateEngineForDocument', (documentUri: string) => {
      const document = vscode.workspace.textDocuments.find((doc) => doc.uri.toString() === documentUri)
      if (document) {
        setEngineForDocument(context, document)
      }
    }),
  )
}

const setEngineForDocument = async (context: vscode.ExtensionContext, document: vscode.TextDocument) => {
  const connection = getConnectionForFile(context, document.uri.toString())
  if (!connection) {
    return
  }

  try {
    const response = await client.sendRequest('engine/set', {
      document_uri: document.uri.toString(),
      uri: connection.uri,
      kind: { type: 'postgres', uri: connection.uri },
    })

    if (response && typeof response === 'object' && 'result' in response) {
      const result = (response as any).result
      if (result && typeof result === 'object' && 'status' in result) {
        if (result.status === 'success') {
          vscode.window.showInformationMessage(
            `✓ Connected to ${connection.name} for ${document.fileName.split('/').pop()}`,
          )
        } else if (result.status === 'error') {
          const errorMsg = result.message || 'Unknown error'
          vscode.window.showErrorMessage(`✗ Failed to connect to ${connection.name}: ${errorMsg}`)
        }
      }
    }
  } catch (err: any) {
    vscode.window.showErrorMessage(`Failed to connect to ${connection.name}: ${err.message || err}`)
  }
}

const removeEngineForDocument = (document: vscode.TextDocument) => {
  client.sendNotification('engine/remove', {
    document_uri: document.uri.toString(),
  })
}

export const deactivate = () => client?.stop()

const serverOptions = () =>
  new Promise<StreamInfo>((resolve, reject) => {
    const ws = new ReconnectingWebSocket('ws://127.0.0.1:9001', [], {
      WebSocket,
      connectionTimeout: 5000,
      maxRetries: Infinity,
      maxReconnectionDelay: 10000,
      minReconnectionDelay: 1000,
    })

    ws.addEventListener('open', () => {
      const stream = new WebSocketStream(ws)
      resolve({ writer: stream, reader: stream })
    })

    ws.addEventListener('error', (event) => console.error('Failed to connect to server:', event))

    setTimeout(() => {
      const { readyState } = ws
      if (readyState !== ReconnectingWebSocket.OPEN && readyState !== ReconnectingWebSocket.CONNECTING) {
        reject(new Error('Failed to establish initial connection'))
      }
    }, 10_000)
  })

class WebSocketStream extends Duplex {
  constructor(private ws: ReconnectingWebSocket) {
    super()
    ws.addEventListener('message', (event) => void this.push(event.data))
    ws.addEventListener('close', () => void this.push(null))
    ws.addEventListener('error', (event) => {
      console.error('WebSocket error:', event)
      this.destroy(new Error('WebSocket error'))
    })
  }
  _write(chunk: any, _encoding: BufferEncoding, callback: (error?: Error | null) => void): void {
    if (this.ws.readyState !== ReconnectingWebSocket.OPEN) {
      return callback(new Error('WebSocket is not open'))
    }
    this.ws.send(chunk)
    callback()
  }
  _read(_size: number): void {}
  _final(callback: (error?: Error | null) => void): void {
    this.ws.close()
    callback()
  }
}
