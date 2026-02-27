const API_BASE = import.meta.env.VITE_API_URL || 'https://api.efofx.ai';

/**
 * apiClient — Authenticated fetch wrapper.
 *
 * Adds Authorization: Bearer {apiKey} header to all requests (WSEC-02).
 * Surfaces 401/403 auth errors with clear messages instead of opaque status codes.
 */
export async function apiClient(path: string, apiKey: string, options: RequestInit = {}): Promise<Response> {
  const res = await fetch(`${API_BASE}/api/v1${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
      ...(options.headers || {}),
    },
  });

  // Surface auth errors clearly (WSEC-02)
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
 * Used for public endpoints (branding) that do not require auth (BRND-04).
 */
export function publicClient(path: string, options: RequestInit = {}): Promise<Response> {
  return fetch(`${API_BASE}/api/v1${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers || {}),
    },
  });
}
