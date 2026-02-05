import { invoke } from "@tauri-apps/api/core";

export interface GitResponse {
  stdout: string;
  stderr: string;
  exit_code: number;
}

export type GitError = 
  | { type: "NotARepo"; message: string }
  | { type: "CommandError"; message: string }
  | { type: "MergeConflict"; message: string }
  | { type: "IoError"; message: string }
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
  try {
    return await invoke<GitResponse>("run_git", { repoPath, subcommand });
  } catch (error) {
    console.error("Git Command Failed:", error);
    throw error as GitError;
  }
}
