import { publicClient } from './client';
import type { BrandingConfig } from '../types/widget';

export async function getBranding(apiKeyPrefix: string): Promise<BrandingConfig> {
  const res = await publicClient(`/widget/branding/${apiKeyPrefix}`);
  if (!res.ok) throw new Error(`Branding fetch failed: ${res.status}`);
  return res.json();
}
