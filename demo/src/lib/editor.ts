import { LspMonacoClient } from 'monaco-lsp-bridge'

import { monaco, THEMES } from '@/lib/monaco'
import { WORKER } from '@/lib/lsp'

export const createModel = (p: { uri: string; content: string; language?: string }) => {
  const uri = monaco.Uri.parse(p.uri)
  const current = monaco.editor.getModel(uri)
  const next = current ?? monaco.editor.createModel(p.content, p.language ?? 'sql', uri)
  return next
}

/** Monaco SQL editor wired to the querent completion engine via monaco-lsp-bridge. */
export const createEditor = (props: {
  el: HTMLElement
  theme: 'dark' | 'light'
  select: (stmt: string) => void
  run: (stmt: string) => void
}) => {
  const disposables = new Set<monaco.IDisposable>()

  const editor = monaco.editor.create(props.el, {
    theme: props.theme === 'dark' ? THEMES.dark : THEMES.light,
    automaticLayout: true,
    fontSize: 13,
    lineHeight: 20,
    fontFamily: "'Geist Mono', 'JetBrains Mono', ui-monospace, SFMono-Regular, monospace",
    minimap: { enabled: false },
    padding: { top: 14, bottom: 14 },
    scrollBeyondLastLine: false,
    renderLineHighlight: 'none',
    lineNumbersMinChars: 3,
    glyphMargin: false,
    folding: false,
    fixedOverflowWidgets: true,
    smoothScrolling: true,
    cursorBlinking: 'smooth',
    // Suggestions as you type (not just on Ctrl+Space / trigger characters).
    quickSuggestions: { other: true, comments: false, strings: false },
    quickSuggestionsDelay: 0,
    suggestOnTriggerCharacters: true,
    tabCompletion: 'on',
    scrollbar: { verticalScrollbarSize: 10, horizontalScrollbarSize: 10 },
  })
  disposables.add(editor)

  editor.addAction({
    id: 'querent.run',
    label: 'Run Query',
    keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
    run: () => void props.run(activeSql(editor)),
  })

  // Highlight the statement the cursor sits in — exactly what ⌘↵ will run.
  const decorations = editor.createDecorationsCollection()
  const highlight = () => {
    const stmt = activeSql(editor)
    props.select(stmt)
    highlightActiveStatement(editor, decorations)
  }
  highlight()

  disposables.add(editor.onDidChangeCursorSelection(highlight))
  disposables.add(editor.onDidChangeModelContent(highlight))

  const client = new LspMonacoClient(WORKER, { languageSelector: { language: 'sql' } })
  client.connect(monaco, editor)
  disposables.add(client)

  return { editor, dispose: () => disposables.forEach((d) => d.dispose()) }
}

/** The SQL to execute: the current selection, else the statement at the cursor. */
const activeSql = (editor: monaco.editor.IStandaloneCodeEditor): string => {
  const model = editor.getModel()
  if (!model) return ''

  const selection = editor.getSelection()
  if (selection && !selection.isEmpty()) return model.getValueInRange(selection)

  const bounds = statementBounds(model, editor.getPosition())
  const text = bounds ? model.getValue().slice(bounds[0], bounds[1]) : ''
  return text
}

/** Decorate the active statement, unless the user has an explicit selection. */
const highlightActiveStatement = (
  editor: monaco.editor.IStandaloneCodeEditor,
  decorations: monaco.editor.IEditorDecorationsCollection,
) => {
  const model = editor.getModel()
  const selection = editor.getSelection()
  if (!model || (selection && !selection.isEmpty())) return decorations.clear()

  const bounds = statementBounds(model, editor.getPosition())
  if (!bounds) return decorations.clear()

  const range = monaco.Range.fromPositions(model.getPositionAt(bounds[0]), model.getPositionAt(bounds[1]))
  decorations.set([
    { range, options: { isWholeLine: true, className: 'cur-stmt-bg', linesDecorationsClassName: 'cur-stmt-bar' } },
  ])
}

const SPACE = /\s/
const START = { lineNumber: 1, column: 1 }

/**
 * Bounds of the statement at the cursor.
 *
 * **rules**
 * - The statement starts at the first non-space character after the last semicolon before the cursor.
 * - The statement ends at the last non-space character before the next semicolon after the cursor.
 * - No selection when the cursor is between a semicolon and the first non-space character.
 */
const statementBounds = (model: monaco.editor.ITextModel, pos: monaco.Position | null): [number, number] | null => {
  const offset = model.getOffsetAt(pos ?? START)
  const text = model.getValue()

  // Find the semicolon before the cursor (or start of text).
  let startEdge = offset - 1
  while (startEdge > 0 && text[startEdge - 1] !== ';') {
    startEdge--
  }

  // Find the semicolon after the cursor (or end of text).
  let endEdge = offset - 1
  while (endEdge < text.length && text[endEdge] !== ';') {
    endEdge++
  }

  // Trim leading whitespace.
  let start = startEdge
  while (start < endEdge && SPACE.test(text[start]!)) {
    start++
  }

  // Trim trailing whitespace.
  let end = endEdge
  while (end > start && SPACE.test(text[end - 1]!)) {
    end--
  }

  // No selection if the cursor sits in the leading whitespace.
  if (offset < start) return null

  return start < end ? [start, end] : null
}
