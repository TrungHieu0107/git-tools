import { invoke } from "@tauri-apps/api/core";
import type { CommitChangedFile } from "../GitService";
import type { CommitDiff, FileCommit, GitCommandResult } from "../types";
import { executeGitCommand } from "./command-executor";

export class CommitService {
  static async generateCommitMessage(repoPath?: string): Promise<string> {
    return invoke("cmd_generate_commit_message", { repoPath });
  }

  static async commit(message: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_commit",
      { message, repoPath },
      "Commit successful",
      "Commit failed",
      { reloadGraph: true },
    );
  }

  static async getPendingCommitsCount(repoPath?: string): Promise<number> {
    return invoke("cmd_get_pending_commits_count", { repoPath });
  }

  static async getFileHistory(filePath: string, limit = 100, repoPath?: string): Promise<FileCommit[]> {
    return invoke("cmd_get_file_history", { filePath, limit, repoPath });
  }

  static async searchRepoFiles(pattern?: string, repoPath?: string): Promise<string[]> {
    return invoke("cmd_search_repo_files", { pattern, repoPath });
  }

  static async getCommitChangedFiles(commitHash: string, repoPath?: string): Promise<CommitChangedFile[]> {
    try {
      return await invoke<CommitChangedFile[]>("cmd_get_commit_changed_files", { commitHash, repoPath });
    } catch (e: any) {
      console.error("Failed to get changed files for commit", e);
      throw e;
    }
  }

  static async getCommitFileDiff(commitHash: string, filePath: string, repoPath?: string): Promise<GitCommandResult> {
    return invoke("cmd_get_commit_file_diff", { commitHash, filePath, repoPath });
  }

  static async getCommitDiff(
    commitHash: string,
    repoPath?: string,
    filePath?: string,
    encoding?: string,
  ): Promise<CommitDiff> {
    return invoke("cmd_get_commit_diff", { commitHash, filePath, repoPath, encoding });
  }

  static async getFileAtCommit(
    commitHash: string,
    filePath: string,
    repoPath?: string,
    encoding?: string,
  ): Promise<string> {
    return invoke("cmd_get_file_at_commit", { commitHash, filePath, repoPath, encoding });
  }
}
