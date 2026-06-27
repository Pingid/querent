import { useSyncExternalStore } from 'react'

/**
 * Minimal external store: closes over private state, exposes get/set and a
 * React selector hook. Selectors must return stable references for object
 * slices (we only ever store fresh objects on `set`, so this holds).
 */
export function createStore<T extends object>(initial: T) {
  let state = initial
  const subs = new Set<() => void>()

  const get = () => state

  const set = (patch: Partial<T> | ((s: T) => Partial<T>)) => {
    state = { ...state, ...(typeof patch === 'function' ? patch(state) : patch) }
    subs.forEach((fn) => fn())
  }

  const subscribe = (fn: () => void) => {
    subs.add(fn)
    return () => subs.delete(fn)
  }

  const use = <S>(selector: (s: T) => S): S => useSyncExternalStore(subscribe, () => selector(state))

  return { get, set, use }
}
