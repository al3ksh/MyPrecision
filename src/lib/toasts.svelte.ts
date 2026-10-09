/** Transient error messages; each disappears after 5 s. */
export interface Toast {
  id: number
  text: string
}

const LIFETIME_MS = 5000

class ToastStore {
  items = $state<Toast[]>([])
  #next = 1

  push(text: string) {
    const id = this.#next++
    this.items.push({ id, text })
    setTimeout(() => this.dismiss(id), LIFETIME_MS)
  }

  dismiss(id: number) {
    this.items = this.items.filter((t) => t.id !== id)
  }
}

export const toasts = new ToastStore()

/** Command rejections arrive as plain strings from Rust; anything else gets a generic line. */
export function errorText(e: unknown): string {
  if (typeof e === 'string' && e) return e
  if (e instanceof Error && e.message) return e.message
  return 'Something went wrong.'
}
