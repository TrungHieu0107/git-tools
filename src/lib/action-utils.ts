import type { ConfirmationOptions } from "./confirmation.svelte";
import { confirm } from "./confirmation.svelte";
import { toast } from "./toast.svelte";

type MaybePromise<T> = T | Promise<T>;

export type ExecuteWithFeedbackOptions<T> = {
  confirmation?: ConfirmationOptions;
  action: () => Promise<T>;
  onSuccess?: (result: T) => MaybePromise<void>;
  successMessage?: string;
  errorMessage?: string;
  onError?: (error: unknown) => MaybePromise<void>;
};

function toErrorMessage(error: unknown, fallback?: string): string {
  if (fallback) return fallback;
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  return "Operation failed";
}

export async function withToast<T>(
  action: () => Promise<T>,
  successMessage: string,
  errorMessage?: string,
): Promise<T | null> {
  try {
    const result = await action();
    if (successMessage.trim()) {
      toast.success(successMessage);
    }
    return result;
  } catch (error) {
    toast.error(toErrorMessage(error, errorMessage));
    return null;
  }
}

export async function executeWithFeedback<T>(options: ExecuteWithFeedbackOptions<T>): Promise<T | null> {
  if (options.confirmation) {
    const confirmed = await confirm(options.confirmation);
    if (!confirmed) return null;
  }

  try {
    const result = await options.action();
    if (options.successMessage?.trim()) {
      toast.success(options.successMessage);
    }
    await options.onSuccess?.(result);
    return result;
  } catch (error) {
    await options.onError?.(error);
    if (options.errorMessage !== "") {
      toast.error(toErrorMessage(error, options.errorMessage));
    }
    return null;
  }
}
