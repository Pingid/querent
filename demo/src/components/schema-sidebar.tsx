import { ChevronRight, Table2, Eye, Database } from 'lucide-react'
import { useState } from 'react'

import type { Column } from '@/querent'
import { ENGINES } from '@/lib/engine'
import { store } from '@/lib/app'
import { cn } from '@/lib/utils'

/** Left panel: an explorer over the introspected database schema. */
export function SchemaSidebar() {
  const schema = store.use((s) => s.schema)
  const engine = store.use((s) => s.engine)
  const tables = schema?.tables ?? []
  const byTable = groupColumns(schema?.columns ?? [])

  return (
    <aside className="flex h-full w-60 shrink-0 flex-col border-r border-border bg-background-shade/40">
      <div className="text-muted-foreground flex h-9 shrink-0 items-center gap-1.5 border-b border-border px-3 text-xs font-medium">
        <Database className="size-3.5" />
        {ENGINES[engine].label}
        <span className="ml-auto tabular-nums">{tables.length}</span>
      </div>
      <div className="no-bars flex-1 overflow-y-auto py-1">
        {tables.length === 0 && <p className="text-muted-foreground px-3 py-2 text-xs">introspecting…</p>}
        {tables.map((t) => (
          <TableItem
            key={t.table_name}
            name={t.table_name}
            isView={t.table_type === 'view'}
            columns={byTable.get(t.table_name) ?? []}
          />
        ))}
      </div>
    </aside>
  )
}

function TableItem({ name, isView, columns }: { name: string; isView: boolean; columns: Column[] }) {
  const [open, setOpen] = useState(true)
  const Icon = isView ? Eye : Table2

  return (
    <div className="px-1">
      <button
        onClick={() => setOpen((v) => !v)}
        className="hover:bg-muted/60 flex w-full items-center gap-1 rounded-md px-1.5 py-1 text-left text-xs"
      >
        <ChevronRight className={cn('size-3 transition-transform', open && 'rotate-90')} />
        <Icon className="size-3.5 text-muted-foreground" />
        <span className="font-medium">{name}</span>
        <span className="text-muted-foreground ml-auto tabular-nums">{columns.length}</span>
      </button>
      {open && (
        <ul className="mb-0.5 ml-[18px] border-l border-border">
          {columns.map((c) => (
            <li
              key={c.column_name}
              className="text-muted-foreground hover:text-foreground flex items-center gap-2 py-0.5 pl-2.5 pr-1.5 text-xs"
            >
              <span className="truncate">{c.column_name}</span>
              <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground/70">{c.data_type}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

function groupColumns(columns: Column[]): Map<string, Column[]> {
  const map = new Map<string, Column[]>()
  for (const c of columns) {
    if (!c.table_name) continue
    const list = map.get(c.table_name) ?? []
    list.push(c)
    map.set(c.table_name, list)
  }
  return map
}
