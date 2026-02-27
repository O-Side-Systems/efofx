const API_BASE = import.meta.env.VITE_API_URL || 'https://api.efofx.ai';

export function apiClient(path: string, apiKey: string, options: RequestInit = {}): Promise<Response> {
  return fetch(`${API_BASE}/api/v1${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
      ...(options.headers || {}),
    },
  });
}

// Public endpoint (no auth) — used for branding
export function publicClient(path: string, options: RequestInit = {}): Promise<Response> {
  return fetch(`${API_BASE}/api/v1${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers || {}),
    },
  });
}
