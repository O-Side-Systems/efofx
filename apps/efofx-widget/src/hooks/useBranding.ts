import { useState, useEffect } from 'react';
import { getBranding } from '../api/branding';
import type { BrandingConfig } from '../types/widget';

export function useBranding(apiKey: string) {
  const [branding, setBranding] = useState<BrandingConfig | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!apiKey) return;
    // Extract prefix: sk_live_{32chars}_{random} -> 32chars
    const parts = apiKey.split('_');
    const prefix = parts.length >= 3 ? parts[2].slice(0, 32) : '';
    if (!prefix) return;

    getBranding(prefix)
      .then(setBranding)
      .catch((e) => {
        console.error('[efOfX widget] Failed to load branding:', e);
        setError(e.message);
      });
  }, [apiKey]);

  return { branding, error };
}
