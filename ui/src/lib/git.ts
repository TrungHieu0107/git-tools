import { invoke } from "@tauri-apps/api/core";
import type { GitCommandError, GitCommandOutput, GitCommandRequest } from "./types";

export async function runGitCommand(
  request: GitCommandRequest,
): Promise<GitCommandOutput> {
  try {
    return await invoke<GitCommandOutput>("run_git", { request });
  } catch (error) {
    throw error as GitCommandError;
  }
}
