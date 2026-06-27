import { useEffect, useRef } from 'react'

import { createEditor, createModel } from '@/lib/editor'
import { run, store } from '@/lib/app'

import { useTheme } from '@/components/theme-provider'

/** Monaco SQL editor wired to the querent completion engine via monaco-lsp-bridge. */
export function Editor() {
  const editor = useEditor()
  return <div ref={editor.ref} className="h-full w-full" />
}

const useEditor = () => {
  const ref = useRef<HTMLDivElement>(null)
  const page = store.use((s) => s.page)
  const theme = useTheme()

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const select = (stmt: string) => store.set({ activeStatement: stmt })
    const e = createEditor({ el, theme: theme.mode, run, select })

    const model = createModel({ uri: page.uri, content: page.initial })
    e.editor.setModel(model)

    return () => e.dispose()
  }, [theme.mode, page])

  return { ref }
}
