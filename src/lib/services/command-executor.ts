import type { GitCommandResult } from "../types";
import { invokeShared } from "./invoke-shared";

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
  return invokeShared<T>(command, params, {
    successToast: successMsg ? successMsg : false,
    errorToast: (error) => `${errorMsg}: ${error}`,
    reloadGraphOnSuccess: options?.reloadGraph,
  });
}

export async function executeGitCommand(
  command: string,
  params: Record<string, unknown>,
  successMsg: string,
  errorMsg: string,
  options?: ExecuteOptions,
): Promise<GitCommandResult> {
  return invokeShared<GitCommandResult>(command, params, {
    isSuccess: (result) => result.success,
    successToast: successMsg ? successMsg : false,
    failureToast: (result) => `${errorMsg}: ${result.stderr}`,
    errorToast: (error) => `${errorMsg}: ${error}`,
    reloadGraphOnSuccess: options?.reloadGraph,
  });
}
