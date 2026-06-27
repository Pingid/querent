import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import * as monaco from 'monaco-editor/esm/vs/editor/editor.main.js'
import { createHighlighterCore } from 'shiki/core'
import { shikiToMonaco } from '@shikijs/monaco'

import { createJavaScriptRegexEngine } from 'shiki/engine/javascript'
import githubLight from 'shiki/themes/github-light-default.mjs'
import githubDark from 'shiki/themes/github-dark-default.mjs'
import sqlLang from 'shiki/langs/sql.mjs'

// Monaco resolves editor features from a web worker. We only use the base
// editor worker (SQL highlighting comes from Shiki's TextMate grammar below).
;(self as any).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
}

export const THEMES = { dark: 'querent-dark', light: 'querent-light' }

// Editor chrome overrides layered on top of the Shiki theme to match the UI.
const DARK = {
  'editor.background': 'oklch(1 0 0)',
  'editor.lineHighlightBackground': '#18181b',
  'editorLineNumber.foreground': '#3f3f46',
  'editorLineNumber.activeForeground': '#a1a1aa',
  'editorCursor.foreground': '#e4e4e7',
  'editor.selectionBackground': '#27272a',
  'editorWidget.background': '#18181b',
  'editorWidget.border': '#27272a',
  'editorSuggestWidget.background': '#18181b',
  'editorSuggestWidget.border': '#27272a',
  'editorSuggestWidget.selectedBackground': '#27272a',
  'editorIndentGuide.background1': '#1f1f23',
}

const LIGHT = {}

const THEME = 'querent-dark'

// Synchronous fallback so the editor has a dark theme before Shiki finishes.
monaco.editor.defineTheme(THEMES.dark, { base: 'vs-dark', inherit: true, rules: [], colors: DARK })
monaco.editor.defineTheme(THEMES.light, { base: 'vs', inherit: true, rules: [], colors: LIGHT })

let started = false

/** Load Shiki, apply its TextMate SQL grammar to Monaco, and swap in the theme. */
export async function setupHighlighting() {
  if (started) return
  started = true
  const dark = { ...githubDark, name: THEMES.dark, colors: { ...githubDark.colors, ...DARK } }
  const light = { ...githubLight, name: THEMES.light, colors: { ...githubLight.colors } }
  const highlighter = await createHighlighterCore({
    themes: [dark, light],
    langs: [sqlLang],
    engine: createJavaScriptRegexEngine(),
  })
  shikiToMonaco(highlighter, monaco)
  monaco.editor.setTheme(THEME)
}

void setupHighlighting()

export { monaco }
