// ─── API Response shapes ──────────────────────────────────────────────────────

export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: string }
export { AuthRole } from '@/types/auth-role'
