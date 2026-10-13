import { api } from './api'

/** Shown outside the known Dell lines, and until the line-specific name arrives. */
export const FALLBACK_NAME = 'MyStation'

/** "MyXPS" on an XPS, "MyPrecision" on a Precision; the fallback when the model can't be read. */
export async function appName(): Promise<string> {
  try {
    const name = await api.getAppName()
    return typeof name === 'string' && name ? name : FALLBACK_NAME
  } catch {
    return FALLBACK_NAME
  }
}
