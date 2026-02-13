import { invoke } from "@tauri-apps/api/core";
import type { GitCommandResult } from "../types";
import { toast } from "../toast.svelte";
import { triggerGraphReload } from "../stores/git-events";

type ExecuteOptions = {
  reloadGraph?: boolean;
};

export async function executeCommand<T>(
  command: string,
  params: Record<string, unknown>,
  successMsg: string,
  errorMsg: string,
  options?: ExecuteOptions,
): Promise<T> {
  try {
    const result = await invoke<T>(command, params);
    if (successMsg) {
      toast.success(successMsg);
    }
    if (options?.reloadGraph) {
      triggerGraphReload();
    }
    return result;
  } catch (e: any) {
    toast.error(`${errorMsg}: ${e}`);
    throw e;
  }
}

export async function executeGitCommand(
  command: string,
  params: Record<string, unknown>,
  successMsg: string,
  errorMsg: string,
  options?: ExecuteOptions,
): Promise<GitCommandResult> {
  try {
    const result = await invoke<GitCommandResult>(command, params);
    if (result.success) {
      if (successMsg) {
        toast.success(successMsg);
      }
      if (options?.reloadGraph) {
        triggerGraphReload();
      }
    } else {
      toast.error(`${errorMsg}: ${result.stderr}`);
    }
    return result;
  } catch (e: any) {
    toast.error(`${errorMsg}: ${e}`);
    throw e;
  }
}
