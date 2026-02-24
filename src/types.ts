import { z } from "zod";

export type ProviderId = "claude" | "codex";
export type SourceKind = "structured" | "parsed" | "cache";
export type Confidence = "high" | "medium" | "low";

export type NormalizedErrorType =
  | "not-installed"
  | "auth-required"
  | "offline"
  | "tty-unavailable"
  | "timeout"
  | "parse-failed"
  | "invalid-response"
  | "acquire-failed"
  | "unexpected";

export const NormalizedRowSchema = z.object({
  provider: z.enum(["claude", "codex"]),
  window: z.string().min(1),

  // For some providers we may only know a percent. Keep both.
  used: z.number().int().nonnegative().nullable(),
  limit: z.number().int().positive().nullable(),
  usedPercent: z.number().min(0).max(100).nullable(),

  resetAt: z.string().datetime({ offset: true }).nullable(),

  source: z.enum(["structured", "parsed", "cache"]),
  confidence: z.enum(["high", "medium", "low"]),
  stale: z.boolean(),

  notes: z.string().nullable(),
});

export type NormalizedRow = z.infer<typeof NormalizedRowSchema>;

export const NormalizedErrorSchema = z.object({
  provider: z.enum(["claude", "codex"]),
  type: z.enum([
    "not-installed",
    "auth-required",
    "offline",
    "tty-unavailable",
    "timeout",
    "parse-failed",
    "invalid-response",
    "acquire-failed",
    "unexpected",
  ]),
  message: z.string().min(1),
  actionable: z.string().nullable(),
});

export type NormalizedError = z.infer<typeof NormalizedErrorSchema>;

export const NormalizedSnapshotSchema = z.object({
  fetchedAt: z.string().datetime({ offset: true }),
  rows: z.array(NormalizedRowSchema),
  errors: z.array(NormalizedErrorSchema),
});

export type NormalizedSnapshot = z.infer<typeof NormalizedSnapshotSchema>;
