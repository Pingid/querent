import { lazy, Suspense, useEffect } from 'react'

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable'
import { SchemaSidebar } from '@/components/schema-sidebar'
import { ThemeProvider } from '@/components/theme-provider'
import { Results } from '@/components/results'
import { Header } from '@/components/header'

import { boot, store } from '@/lib/app'

const Editor = lazy(() => import('@/components/editor').then((m) => ({ default: m.Editor })))

export const App = () => {
  useEffect(() => void boot(), [])
  const bootError = store.use((s) => (s.status === 'error' ? s.bootError : undefined))

  return (
    <ThemeProvider defaultTheme="dark">
      <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
        <Header />
        {bootError && (
          <div className="bg-destructive/10 text-destructive border-b border-destructive/30 px-3 py-1.5 text-xs">
            Failed to start engine: {bootError}
          </div>
        )}
        <div className="flex min-h-0 flex-1">
          <SchemaSidebar />
          <ResizablePanelGroup orientation="vertical" className="min-w-0 flex-1">
            <ResizablePanel defaultSize={55} minSize={20}>
              <Suspense fallback={<Loading />}>
                <Editor />
              </Suspense>
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel defaultSize={45} minSize={15}>
              <Results />
            </ResizablePanel>
          </ResizablePanelGroup>
        </div>
      </div>
    </ThemeProvider>
  )
}

const Loading = () => {
  return (
    <div className="flex h-full w-full items-center justify-center animate-pulse text-sm text-muted-foreground">
      Loading...
    </div>
  )
}
