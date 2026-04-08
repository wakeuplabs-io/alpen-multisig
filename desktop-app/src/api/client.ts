import type { ApiResult } from '@/types'

const BASE_URL = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:3000/api/v1'

type RequestOptions = {
  method?: string
  body?: unknown
  sessionToken?: string
}

async function request<T>(
  path: string,
  { method = 'GET', body, sessionToken }: RequestOptions = {}
): Promise<ApiResult<T>> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }

  if (sessionToken) {
    headers['Authorization'] = `Bearer ${sessionToken}`
  }

  try {
    const res = await fetch(`${BASE_URL}${path}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    })

    const json = await res.json()

    if (!res.ok) {
      return { ok: false, error: json.error ?? 'Unknown error' }
    }

    return { ok: true, data: json as T }
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Network error' }
  }
}

export const api = {
  get: <T>(path: string, token?: string) => request<T>(path, { sessionToken: token }),
  post: <T>(path: string, body: unknown, token?: string) =>
    request<T>(path, { method: 'POST', body, sessionToken: token }),
  delete: <T>(path: string, token?: string) =>
    request<T>(path, { method: 'DELETE', sessionToken: token }),
}
