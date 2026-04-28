const API_BASE = import.meta.env.VITE_API_URL || 'https://api.efofx.ai';

/**
 * apiClient — Authenticated fetch wrapper.
 *
 * Adds the `x-api-key: {apiKey}` header (Rust core widget auth — Bearer is
 * reserved for Supabase JWT). Surfaces 401/403 with clear messages instead
 * of opaque status codes.
 */
export async function apiClient(path: string, apiKey: string, options: RequestInit = {}): Promise<Response> {
  const res = await fetch(`${API_BASE}/v1${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': apiKey,
      ...(options.headers || {}),
    },
  });

  if (res.status === 401) {
    throw new Error('Invalid API key — check your data-api-key attribute');
  }
  if (res.status === 403) {
    throw new Error('API key not verified — please verify your email first');
  }

  return res;
}

/**
 * publicClient — Unauthenticated fetch wrapper.
 *
 * Used for public endpoints (branding) that do not require auth.
 */
export function publicClient(path: string, options: RequestInit = {}): Promise<Response> {
  return fetch(`${API_BASE}/v1${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers || {}),
    },
  });
}
