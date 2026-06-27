import { Play, LoaderCircle, Zap } from 'lucide-react'

import { ENGINES, type EngineKind } from '@/lib/engine'
import { store, switchEngine, run } from '@/lib/app'

import { ThemeToggle } from '@/components/theme-toggle'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

/** Top bar: brand, engine switcher, engine status, and the run control. */
export function Header() {
  const status = store.use((s) => s.status)
  const running = store.use((s) => s.running)
  const stmt = store.use((s) => s.activeStatement)

  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border px-3">
      <div className="flex items-center gap-2">
        <div className="flex size-6 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <Zap className="size-3.5" />
        </div>
        <span className="text-sm font-semibold tracking-tight">querent</span>
        <span className="text-muted-foreground hidden text-xs sm:inline">SQL completions</span>
      </div>

      <div className="flex items-center gap-3">
        <EngineSwitch />
        <StatusDot status={status} />
        <Button
          size="sm"
          onClick={() => (stmt ? run(stmt) : undefined)}
          disabled={status !== 'ready' || running || !stmt?.trim()}
        >
          {running ? <LoaderCircle className="size-3.5 animate-spin" /> : <Play className="size-3.5" />}
          Run
          <kbd className="ml-1 rounded px-1 text-[10px] leading-4 inline-flex items-center gap-1">
            <kbd className="font-sans text-[0.6rem] place-self-center">⌘</kbd>
            <kbd className="font-sans text-[0.6rem] place-self-center pt-1">↵</kbd>
          </kbd>
        </Button>
        <ThemeToggle />
      </div>
    </header>
  )
}

/** Segmented control to flip the active in-browser database. */
function EngineSwitch() {
  const engine = store.use((s) => s.engine)
  const busy = store.use((s) => s.status === 'booting')

  return (
    <div className="flex items-center rounded-lg border border-border bg-background-shade/60 p-0.5">
      {(Object.keys(ENGINES) as EngineKind[]).map((kind) => (
        <button
          key={kind}
          disabled={busy}
          onClick={() => void switchEngine(kind)}
          className={cn(
            'rounded-md px-2 py-0.5 text-xs font-medium transition-colors disabled:opacity-60',
            engine === kind ? 'bg-muted text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
          )}
        >
          {ENGINES[kind].label}
        </button>
      ))}
    </div>
  )
}

const LABELS: Record<string, string> = {
  booting: 'starting engine…',
  ready: 'engine ready',
  error: 'engine error',
}

function StatusDot({ status }: { status: string }) {
  const color = status === 'ready' ? 'bg-emerald-500' : status === 'error' ? 'bg-destructive' : 'bg-amber-500'
  return (
    <span className="text-muted-foreground hidden items-center gap-1.5 text-xs md:flex">
      <span className={`size-1.5 rounded-full ${color} ${status === 'booting' ? 'animate-pulse' : ''}`} />
      {LABELS[status] ?? status}
    </span>
  )
}
