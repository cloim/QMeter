import type { NormalizedError, NormalizedRow, ProviderId } from "../types.js";

export type AcquireContext = {
  refresh: boolean;
  debug: boolean;
};

export type ProviderResult = {
  rows: NormalizedRow[];
  errors: NormalizedError[];
  debug?: Record<string, unknown>;
};

export interface Provider {
  id: ProviderId;
  acquire(ctx: AcquireContext): Promise<ProviderResult>;
}
