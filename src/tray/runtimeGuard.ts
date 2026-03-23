export type RuntimeErrorReporter = (
  scope: string,
  message: string,
  error: unknown
) => void | Promise<void>;

export function formatRuntimeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }
  if (error == null) {
    return "Unknown error";
  }
  return String(error);
}

export async function runGuardedTrayTask(
  scope: string,
  task: () => Promise<void>,
  onError: RuntimeErrorReporter
): Promise<boolean> {
  try {
    await task();
    return true;
  } catch (error) {
    await onError(scope, formatRuntimeError(error), error);
    return false;
  }
}
