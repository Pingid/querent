import { CircleAlert, Columns3 } from 'lucide-react'

import type { QueryResult } from '@/lib/engine'
import { store } from '@/lib/app'

/** Bottom panel: tabular query output, timing, or the last query error. */
export function Results() {
  const result = store.use((s) => s.result)
  const error = store.use((s) => s.queryError)
  const running = store.use((s) => s.running)

  return (
    <div className="flex h-full flex-col">
      <ResultBar result={result} error={error} running={running} />
      <div className="no-bars flex-1 overflow-auto">
        {error ? <ErrorView message={error} /> : result ? <ResultTable result={result} /> : <Empty />}
      </div>
    </div>
  )
}

function ResultBar({ result, error, running }: { result?: QueryResult; error?: string; running: boolean }) {
  return (
    <div className="text-muted-foreground flex h-9 shrink-0 items-center gap-3 border-b border-border px-3 text-xs">
      <span className="flex items-center gap-1.5 font-medium text-foreground">
        <Columns3 className="size-3.5" />
        Results
      </span>
      {running && <span className="text-amber-500">running…</span>}
      {!running && error && <span className="text-destructive">error</span>}
      {!running && !error && result && (
        <span className="ml-auto flex items-center gap-3 tabular-nums">
          <span>{result.rowCount} rows</span>
          <span>{result.elapsedMs.toFixed(1)} ms</span>
        </span>
      )}
    </div>
  )
}

function ResultTable({ result }: { result: QueryResult }) {
  if (result.columns.length === 0) return <Empty label="Statement executed." />

  return (
    <table className="w-full border-collapse text-xs">
      <thead className="sticky top-0 z-10 bg-background-shade">
        <tr>
          <th className="w-10 border-b border-r border-border px-2 py-1.5 text-right font-normal text-muted-foreground/60" />
          {result.columns.map((col) => (
            <th
              key={col}
              className="border-b border-r border-border px-3 py-1.5 text-left font-medium whitespace-nowrap"
            >
              {col}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {result.rows.map((row, i) => (
          <tr key={i} className="hover:bg-muted/40">
            <td className="border-b border-r border-border px-2 py-1 text-right tabular-nums text-muted-foreground/50">
              {i + 1}
            </td>
            {row.map((cell, j) => (
              <td
                key={j}
                className="max-w-md truncate border-b border-r border-border px-3 py-1 font-mono whitespace-nowrap"
                title={cell}
              >
                {cell === '' ? <span className="text-muted-foreground/40">null</span> : cell}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  )
}

function ErrorView({ message }: { message: string }) {
  return (
    <div className="flex items-start gap-2 p-4 text-xs">
      <CircleAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
      <pre className="text-destructive whitespace-pre-wrap font-mono">{message}</pre>
    </div>
  )
}

function Empty({ label = 'Run a query to see results.' }: { label?: string }) {
  return <div className="text-muted-foreground/60 flex h-full items-center justify-center text-xs">{label}</div>
}
