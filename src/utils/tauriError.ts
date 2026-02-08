import type { AppError } from "../types/project";

function asRecord(value: unknown): Record<string, unknown> | null {
  if (typeof value === "object" && value !== null) {
    return value as Record<string, unknown>;
  }
  return null;
}

function parseFromRecord(
  value: Record<string, unknown>,
  fallbackCode: string,
  fallbackMessage: string
): AppError | null {
  const code = typeof value.code === "string" ? value.code : fallbackCode;
  const message =
    typeof value.message === "string" && value.message.trim().length > 0
      ? value.message
      : null;
  const suggestion =
    typeof value.suggestion === "string" ? value.suggestion : undefined;
  if (message) {
    return { code, message, suggestion };
  }
  return null;
}

export function normalizeInvokeError(
  error: unknown,
  fallbackCode: string,
  fallbackMessage: string
): AppError {
  if (typeof error === "string" && error.trim().length > 0) {
    return { code: fallbackCode, message: error };
  }
  if (error instanceof Error && error.message.trim().length > 0) {
    return { code: fallbackCode, message: error.message };
  }

  const direct = asRecord(error);
  if (direct) {
    const parsed = parseFromRecord(direct, fallbackCode, fallbackMessage);
    if (parsed) {
      return parsed;
    }

    const nested = asRecord(direct.error);
    if (nested) {
      const nestedParsed = parseFromRecord(nested, fallbackCode, fallbackMessage);
      if (nestedParsed) {
        return nestedParsed;
      }
    }
  }

  return { code: fallbackCode, message: fallbackMessage };
}

