import * as vscode from 'vscode'

type BaseConnection = {
  id: string
  name: string
}

type PgConnection = BaseConnection & { type: 'pg'; uri: string }
type MySqlConnection = BaseConnection & { type: 'mysql'; uri: string }
type SqliteConnection = BaseConnection & { type: 'sqlite'; uri: string }
type RemoteConnection = BaseConnection & {
  type: 'remote'
  uri: string
  dialect: 'postgresql' | 'mysql' | 'sqlite' | 'duckdb'
}

type Connection = PgConnection | MySqlConnection | SqliteConnection | RemoteConnection

// State management
const getState = <T>(context: vscode.ExtensionContext, key: string, defaultValue: T): T =>
  context.globalState.get(key, defaultValue)

const setState = <T>(context: vscode.ExtensionContext, key: string, value: T) => context.globalState.update(key, value)

const getConnections = (context: vscode.ExtensionContext) => getState<Connection[]>(context, 'connections', [])

const getDefaultConnections = (context: vscode.ExtensionContext) =>
  getState<Record<string, string>>(context, 'defaultConnections', {})

const getLastSelected = (context: vscode.ExtensionContext) =>
  getState<string | undefined>(context, 'lastSelectedConnection', undefined)

const getDescription = (conn: Connection) => (conn.type === 'remote' ? `${conn.type} (${conn.dialect})` : conn.type)

const setConnectionForFile = async (context: vscode.ExtensionContext, connectionId: string, fileUri?: string) => {
  await setState(context, 'lastSelectedConnection', connectionId)
  if (fileUri) {
    const defaults = getDefaultConnections(context)
    defaults[fileUri] = connectionId
    await setState(context, 'defaultConnections', defaults)
  }
}

// Public API
export const getConnectionForFile = (context: vscode.ExtensionContext, fileUri: string): Connection | undefined => {
  const connections = getConnections(context)
  const connectionId = getDefaultConnections(context)[fileUri] || getLastSelected(context)
  return connections.find((conn) => conn.id === connectionId)
}

export const registerConnections = (context: vscode.ExtensionContext) => {
  const commands = {
    'querent.addConnection': () => addConnection(context),
    'querent.viewConnections': () => viewConnections(context),
    'querent.editConnection': () => editConnection(context),
    'querent.deleteConnection': () => deleteConnection(context),
  }
  context.subscriptions.push(...Object.entries(commands).map(([cmd, fn]) => vscode.commands.registerCommand(cmd, fn)))
}

// Input helpers
const input = (prompt: string, options?: Partial<vscode.InputBoxOptions>) =>
  vscode.window.showInputBox({ prompt, ...options })

const pick = <T extends vscode.QuickPickItem>(items: T[], placeHolder: string) =>
  vscode.window.showQuickPick(items, { placeHolder })

const pickConnection = async (context: vscode.ExtensionContext, placeHolder: string) => {
  const connections = getConnections(context)
  if (connections.length === 0) return undefined

  const items = connections.map((conn) => ({
    label: conn.name,
    description: getDescription(conn),
    connection: conn,
  }))

  return (await pick(items, placeHolder))?.connection
}

// Connection builders
const buildSqliteConnection = async (name: string): Promise<SqliteConnection | undefined> => {
  const files = await vscode.window.showOpenDialog({
    canSelectFiles: true,
    canSelectFolders: false,
    canSelectMany: false,
    filters: { Database: ['db', 'sqlite', 'sqlite3'] },
    openLabel: 'Select SQLite database',
  })
  if (!files?.[0]) return undefined

  return { id: Date.now().toString(), name, type: 'sqlite', uri: files[0].fsPath }
}

const buildRemoteConnection = async (name: string): Promise<RemoteConnection | undefined> => {
  const uri = await input('Remote connection URI', { placeHolder: 'ws://localhost:9001' })
  if (!uri) return undefined

  const dialect = await pick(
    ['postgresql', 'mysql', 'sqlite', 'duckdb'].map((d) => ({ label: d })),
    'Select SQL dialect',
  )
  if (!dialect) return undefined

  return { id: Date.now().toString(), name, type: 'remote', uri, dialect: dialect.label as RemoteConnection['dialect'] }
}

const buildDbConnection = async (
  name: string,
  type: 'pg' | 'mysql',
): Promise<PgConnection | MySqlConnection | undefined> => {
  const defaultPort = type === 'pg' ? '5432' : '3306'
  const host = await input('Host', { placeHolder: 'localhost', value: 'localhost' })
  if (!host) return undefined

  const port = await input('Port', { placeHolder: defaultPort, value: defaultPort })
  if (!port) return undefined

  const database = await input('Database name', { placeHolder: type === 'pg' ? 'postgres' : 'mysql' })
  if (!database) return undefined

  const username = await input('Username', { placeHolder: 'user' })
  if (!username) return undefined

  const password = await input('Password (optional)', { placeHolder: 'password', password: true })

  const protocol = type === 'pg' ? 'postgresql' : 'mysql'
  const auth = `${username}${password ? ':' + password : ''}`
  const uri = `${protocol}://${auth}@${host}:${port}/${database}`

  return { id: Date.now().toString(), name, type, uri }
}

// Commands
const addConnection = async (context: vscode.ExtensionContext) => {
  const name = await input('Connection name', { placeHolder: 'My Database' })
  if (!name) return

  const typeOptions = [
    { label: 'PostgreSQL', value: 'pg' as const },
    { label: 'MySQL', value: 'mysql' as const },
    { label: 'SQLite', value: 'sqlite' as const },
    { label: 'Remote', value: 'remote' as const },
  ]
  const type = await pick(typeOptions, 'Select connection type')
  if (!type) return

  let connection: Connection | undefined
  if (type.value === 'sqlite') connection = await buildSqliteConnection(name)
  else if (type.value === 'remote') connection = await buildRemoteConnection(name)
  else connection = await buildDbConnection(name, type.value)

  if (!connection) return

  const connections = getConnections(context)
  await setState(context, 'connections', [...connections, connection])

  const editor = vscode.window.activeTextEditor
  await setConnectionForFile(context, connection.id, editor?.document.uri.toString())

  const msg = editor
    ? `Connection "${name}" added and set as default for ${editor.document.fileName}`
    : `Connection "${name}" added successfully`
  vscode.window.showInformationMessage(msg)
}

const viewConnections = async (context: vscode.ExtensionContext) => {
  const connections = getConnections(context)
  if (connections.length === 0) {
    const result = await vscode.window.showInformationMessage('No connections configured', 'Add Connection')
    if (result) await addConnection(context)
    return
  }

  const editor = vscode.window.activeTextEditor
  if (!editor) {
    vscode.window.showWarningMessage('No active file to set connection for')
    return
  }

  const fileUri = editor.document.uri.toString()
  const fileDefaultId = getDefaultConnections(context)[fileUri]
  const lastSelectedId = getLastSelected(context)

  const items = connections.map((conn) => {
    const isFileDefault = conn.id === fileDefaultId
    const isLastSelected = conn.id === lastSelectedId && !isFileDefault
    return {
      label: `${isFileDefault ? '$(check) ' : isLastSelected ? '$(circle-outline) ' : ''}${conn.name}`,
      description: getDescription(conn),
      detail: isFileDefault ? 'Current default' : isLastSelected ? 'Last selected' : undefined,
      connection: conn,
    }
  })

  const selected = await pick(items, 'Select a connection for this file')
  if (!selected) return

  await setConnectionForFile(context, selected.connection.id, fileUri)
  vscode.window.showInformationMessage(
    `Connection "${selected.connection.name}" set as default for ${editor.document.fileName}`,
  )
}

const editConnection = async (context: vscode.ExtensionContext) => {
  const conn = await pickConnection(context, 'Select a connection to edit')
  if (!conn) {
    vscode.window.showInformationMessage('No connections to edit')
    return
  }

  const newName = await input('Connection name', { value: conn.name })
  if (!newName) return

  let updated: Connection | undefined

  if (conn.type === 'sqlite') {
    const change = await pick([{ label: 'Keep current file' }, { label: 'Select new file' }], 'Update SQLite file?')
    if (change?.label === 'Select new file') {
      updated = await buildSqliteConnection(newName)
      if (updated) updated = { ...updated, id: conn.id }
    } else {
      updated = { ...conn, name: newName }
    }
  } else if (conn.type === 'remote') {
    const uri = await input('Remote connection URI', { value: conn.uri })
    if (!uri) return
    const dialect = await pick(
      ['postgresql', 'mysql', 'sqlite', 'duckdb'].map((d) => ({ label: d })),
      'Select SQL dialect',
    )
    if (!dialect) return
    updated = { ...conn, name: newName, uri, dialect: dialect.label as RemoteConnection['dialect'] }
  } else {
    const uri = await input('Connection URI', { value: conn.uri })
    if (!uri) return
    updated = { ...conn, name: newName, uri }
  }

  if (!updated) return

  const connections = getConnections(context)
  await setState(
    context,
    'connections',
    connections.map((c) => (c.id === conn.id ? updated : c)),
  )
  vscode.window.showInformationMessage(`Connection "${newName}" updated successfully`)
}

const deleteConnection = async (context: vscode.ExtensionContext) => {
  const conn = await pickConnection(context, 'Select a connection to delete')
  if (!conn) {
    vscode.window.showInformationMessage('No connections to delete')
    return
  }

  const confirm = await vscode.window.showWarningMessage(
    `Are you sure you want to delete connection "${conn.name}"?`,
    { modal: true },
    'Delete',
  )
  if (confirm !== 'Delete') return

  const connections = getConnections(context)
  await setState(
    context,
    'connections',
    connections.filter((c) => c.id !== conn.id),
  )

  const defaults = getDefaultConnections(context)
  await setState(
    context,
    'defaultConnections',
    Object.fromEntries(Object.entries(defaults).filter(([_, id]) => id !== conn.id)),
  )

  vscode.window.showInformationMessage(`Connection "${conn.name}" deleted successfully`)
}
