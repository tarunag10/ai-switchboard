const STORAGE_KEY = "ai-switchboard.provider-billed-sampling.v1";

export interface ProviderBilledSamplingSettings {
  enabled: boolean;
  intervalDays: number;
  lastSampleAt: string | null;
}

export const DEFAULT_PROVIDER_BILLED_SAMPLING: ProviderBilledSamplingSettings = {
  enabled: false,
  intervalDays: 7,
  lastSampleAt: null,
};

export function loadProviderBilledSamplingSettings(): ProviderBilledSamplingSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_PROVIDER_BILLED_SAMPLING };
    const parsed = JSON.parse(raw) as ProviderBilledSamplingSettings;
    return {
      enabled: Boolean(parsed.enabled),
      intervalDays:
        Number.isFinite(parsed.intervalDays) && parsed.intervalDays > 0
          ? Math.floor(parsed.intervalDays)
          : 7,
      lastSampleAt:
        typeof parsed.lastSampleAt === "string" ? parsed.lastSampleAt : null,
    };
  } catch {
    return { ...DEFAULT_PROVIDER_BILLED_SAMPLING };
  }
}

export function saveProviderBilledSamplingSettings(
  settings: ProviderBilledSamplingSettings,
): ProviderBilledSamplingSettings {
  const normalized = {
    enabled: settings.enabled,
    intervalDays: Math.max(1, Math.floor(settings.intervalDays || 7)),
    lastSampleAt: settings.lastSampleAt,
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized));
  return normalized;
}

export function shouldRunProviderBilledSample(
  settings: ProviderBilledSamplingSettings,
  now = new Date(),
): boolean {
  if (!settings.enabled) return false;
  if (!settings.lastSampleAt) return true;
  const last = Date.parse(settings.lastSampleAt);
  if (!Number.isFinite(last)) return true;
  const elapsedMs = now.getTime() - last;
  return elapsedMs >= settings.intervalDays * 24 * 60 * 60 * 1000;
}

export function providerBilledSamplingDisclosure(): string {
  return "Optional weekly counterfactual sampling stores measured pairs in the local savings ledger only after you opt in. No samples run without explicit consent.";
}
