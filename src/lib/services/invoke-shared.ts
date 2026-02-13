import { invoke } from "@tauri-apps/api/core";
import { triggerGraphReload } from "../stores/git-events";
import { toast } from "../toast.svelte";

type ToastMessage<T> = string | ((value: T) => string | null | undefined);

function resolveToastMessage<T>(message: ToastMessage<T> | false | undefined, value: T): string {
  if (!message) {
    return "";
  }
  const resolved = typeof message === "function" ? message(value) : message;
  return resolved ?? "";
}

export type InvokeSharedOptions<TInvokeResult, TReturn = TInvokeResult> = {
  isSuccess?: (result: TInvokeResult) => boolean;
  successToast?: ToastMessage<TInvokeResult> | false;
  failureToast?: ToastMessage<TInvokeResult> | false;
  errorToast?: ToastMessage<unknown> | false;
  reloadGraphOnSuccess?: boolean;
  formatResult?: (result: TInvokeResult) => TReturn;
};

export async function invokeShared<TInvokeResult, TReturn = TInvokeResult>(
  command: string,
  params?: Record<string, unknown>,
  options: InvokeSharedOptions<TInvokeResult, TReturn> = {},
): Promise<TReturn> {
  try {
    const result = await invoke<TInvokeResult>(command, params ?? {});
    const isSuccess = options.isSuccess ? options.isSuccess(result) : true;

    if (isSuccess) {
      const successMessage = resolveToastMessage(options.successToast, result);
      if (successMessage) {
        toast.success(successMessage);
      }
      if (options.reloadGraphOnSuccess) {
        triggerGraphReload();
      }
    } else {
      const failureMessage = resolveToastMessage(options.failureToast, result);
      if (failureMessage) {
        toast.error(failureMessage);
      }
    }

    if (options.formatResult) {
      return options.formatResult(result);
    }

    return result as unknown as TReturn;
  } catch (error) {
    const errorMessage = resolveToastMessage(options.errorToast, error);
    if (errorMessage) {
      toast.error(errorMessage);
    }
    throw error;
  }
}

type GitOutputResult = {
  success: boolean;
  stdout: string;
  stderr: string;
};

export function formatGitOutput(result: GitOutputResult): string {
  let output = result.stdout;
  if (result.stderr) {
    output += `\n${result.stderr}`;
  }
  return output || (result.success ? "Success" : "Failed");
}
