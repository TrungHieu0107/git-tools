import { invokeShared } from "./services/invoke-shared";

export interface GitResponse {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

export type GitError =
  | { type: "NotARepo"; message: string }
  | { type: "CommandError"; message: string }
  | { type: "MergeConflict"; message: string }
  | { type: "IoError"; message: string }
  | { type: "GitNotFound"; message: string }
  | { type: "Timeout"; message: string }
  | { type: "InvalidRepoPath"; message: string }
  | { type: "Unknown"; message: string };

/**
 * Executes a git command asynchronously.
 * @param repoPath Absolute path to the repository.
 * @param subcommand Argument list (e.g., ["status", "--short"]).
 */
export async function runGit(
  repoPath: string,
  subcommand: string[]
): Promise<GitResponse> {
  const cmdStr = ["git", ...subcommand].join(" ");
  const shortCmd = cmdStr.length > 30 ? cmdStr.substring(0, 27) + "..." : cmdStr;
  
  try {
    return await invokeShared<GitResponse>("run_git", { repoPath, subcommand }, {
      isSuccess: (res) => res.exit_code === 0,
      successToast: () => `'${shortCmd}' succeeded`,
      failureToast: (res) => `'${shortCmd}' failed (exit ${res.exit_code})`,
      errorToast: () => `'${shortCmd}' error`,
    });
  } catch (error) {
    console.error("Git Command Failed:", error);
    throw error as GitError;
  }
}
