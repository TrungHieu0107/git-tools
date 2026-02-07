import { invoke } from "@tauri-apps/api/core";
import { toast } from "./toast.svelte";
import type { GitCommandResult } from "./types";
import { triggerGraphReload } from "./stores/git-events";

export interface ConflictFile {
  base: string;
  ours: string;
  theirs: string;
}

export interface RepoEntry {
  id: string;
  name: string;
  path: string;
}

export interface FileStatus {
  path: string;
  status: string;
  staged: boolean;
}

export interface AppSettings {
  repos: RepoEntry[];
  active_repo_id: string | null;
  excluded_files: string[];
}

export class GitService {
  // --- Settings Commands ---

  static async getSettings(): Promise<AppSettings> {
    return invoke("cmd_get_settings");
  }

  static async setExcludedFiles(exclusions: string[]): Promise<AppSettings> {
    return invoke("cmd_set_excluded_files", { exclusions });
  }

  static async addRepo(name: string, path: string): Promise<AppSettings> {
    return invoke("cmd_add_repo", { name, path });
  }

  static async removeRepo(id: string): Promise<AppSettings> {
    return invoke("cmd_remove_repo", { id });
  }

  static async setActiveRepo(id: string): Promise<AppSettings> {
    return invoke("cmd_set_active_repo", { id });
  }

  static async getActiveRepo(): Promise<RepoEntry | null> {
    return invoke("cmd_get_active_repo");
  }

  // --- Git Commands (Context Aware) ---

  /**
   * Get list of conflicting files
   */
  static async getConflicts(repoPath?: string): Promise<string[]> {
    return invoke("cmd_get_conflicts", { repoPath });
  }

  /**
   * Get conflict file details (base, ours, theirs)
   */
  static async getConflictFile(path: string, repoPath?: string): Promise<ConflictFile> {
    return invoke("cmd_get_conflict_file", { path, repoPath });
  }

  /**
   * Resolve a conflict by choosing "ours"
   */
  static async resolveOurs(path: string, repoPath?: string): Promise<void> {
    return invoke("cmd_resolve_ours", { path, repoPath });
  }

  /**
   * Resolve a conflict by choosing "theirs"
   */
  static async resolveTheirs(path: string, repoPath?: string): Promise<void> {
    return invoke("cmd_resolve_theirs", { path, repoPath });
  }

  /**
   * Mark a file as resolved (git add)
   */
  static async markResolved(path: string, repoPath?: string): Promise<void> {
    return invoke("cmd_mark_resolved", { path, repoPath });
  }

  /**
   * Write content to a file (for manual conflict resolution)
   */
  static async writeFile(path: string, content: string, repoPath?: string): Promise<void> {
    return invoke("cmd_write_file", { path, content, repoPath });
  }

  /**
   * Check if the repo is in a conflict state (merge/rebase/etc AND unmerged files)
   */
  static async checkConflictState(repoPath?: string): Promise<boolean> {
    return invoke("cmd_check_conflict_state", { repoPath });
  }

  static async getStatusFiles(repoPath?: string): Promise<FileStatus[]> {
    return invoke("cmd_get_status_files", { repoPath });
  }

  static async getDiff(filePath: string, staged: boolean, repoPath?: string): Promise<string> {
    return invoke("cmd_get_diff_file", { filePath, staged, repoPath });
  }

  static async getFileBaseContent(filePath: string, repoPath?: string): Promise<string> {
    return invoke("cmd_get_file_base_content", { filePath, repoPath });
  }

  static async getFileModifiedContent(filePath: string, staged: boolean, repoPath?: string): Promise<string> {
    return invoke("cmd_get_file_modified_content", { filePath, staged, repoPath });
  }

  static async stageFile(path: string, repoPath?: string): Promise<void> {
    try {
        await invoke("cmd_git_add", { path, repoPath });
        toast.success(`Staged ${path}`);
    } catch(e: any) {
        toast.error(`Stage failed: ${e}`);
        throw e;
    }
  }

  static async unstageFile(path: string, repoPath?: string): Promise<void> {
      try {
        await invoke("cmd_git_unstage", { path, repoPath });
        toast.success(`Unstaged ${path}`);
    } catch(e: any) {
        toast.error(`Unstage failed: ${e}`);
        throw e;
    }
  }

  static async stageAll(repoPath?: string): Promise<void> {
      try {
          await invoke("cmd_git_add_all", { repoPath });
          toast.success("Staged all files");
      } catch (e: any) {
          toast.error(`Stage all failed: ${e}`);
          throw e;
      }
  }

  static async unstageAll(repoPath?: string): Promise<void> {
      try {
          await invoke("cmd_git_unstage_all", { repoPath });
          toast.success("Unstaged all files");
      } catch (e: any) {
          toast.error(`Unstage all failed: ${e}`);
          throw e;
      }
  }

  // --- Branch Management ---

  static async getBranches(includeRemote = false, repoPath?: string): Promise<string[]> {
    return invoke("cmd_get_git_branches", { includeRemote, repoPath });
  }

  static async getCurrentBranch(repoPath?: string): Promise<string> {
    return invoke("cmd_get_current_branch", { repoPath });
  }

  static async switchBranch(branchName: string, repoPath?: string): Promise<GitCommandResult> {
    try {
      const res = await invoke<GitCommandResult>("cmd_git_switch_branch", { branchName, repoPath });
      if (res.success) {
          toast.success(`Switched to branch '${branchName}'`);
          triggerGraphReload();
      } else {
          toast.error(`Failed to switch branch: ${res.stderr}`);
      }
      return res;
    } catch (e: any) {
      toast.error(`Failed to switch branch: ${e}`);
      throw e;
    }
  }

  static async checkoutNew(name: string, startPoint: string, repoPath?: string): Promise<GitCommandResult> {
    try {
      const res = await invoke<GitCommandResult>("cmd_git_checkout_new_branch", { name, startPoint, repoPath });
      if (res.success) {
          toast.success(`Created branch '${name}'`);
          triggerGraphReload();
      } else {
          toast.error(`Failed to create branch: ${res.stderr}`);
      }
      return res;
    } catch (e: any) {
      toast.error(`Failed to create branch: ${e}`);
      throw e;
    }
  }

  static async getCommitGraph(limit: number, repoPath?: string): Promise<string> {
    return invoke("cmd_get_commit_graph", { limit, repoPath });
  }

  static async merge(branch: string, repoPath?: string): Promise<GitCommandResult> {
    try {
      const res = await invoke<GitCommandResult>("cmd_git_merge", { branch, repoPath });
      if (res.success) {
          toast.success(`Merged '${branch}'`);
          triggerGraphReload();
      } else {
          toast.error(`Merge failed: ${res.stderr}`);
      }
      return res;
    } catch (e: any) {
      toast.error(`Merge failed: ${e}`);
      throw e;
    }
  }

  static async fetch(repoPath?: string): Promise<GitCommandResult> {
    try {
      const res = await invoke<GitCommandResult>("cmd_git_fetch", { repoPath });
      if (res.success) {
          toast.success("Fetch completed");
          triggerGraphReload();
      } else {
          toast.error(`Fetch failed: ${res.stderr}`);
      }
      return res;
    } catch (e: any) {
      toast.error(`Fetch failed: ${e}`);
      throw e;
    }
  }

  static async pull(repoPath?: string): Promise<GitCommandResult> {
    try {
      const res = await invoke<GitCommandResult>("cmd_git_pull", { repoPath });
      if (res.success) {
          toast.success("Pull completed");
          triggerGraphReload();
      } else {
          toast.error(`Pull failed: ${res.stderr}`);
      }
      return res;
    } catch (e: any) {
      toast.error(`Pull failed: ${e}`);
      throw e;
    }
  }

  static async push(repoPath?: string): Promise<GitCommandResult> {
    try {
      const res = await invoke<GitCommandResult>("cmd_git_push", { repoPath });
      if (res.success) {
          toast.success("Push completed");
          triggerGraphReload();
      } else {
          toast.error(`Push failed: ${res.stderr}`);
      }
      return res;
    } catch (e: any) {
      toast.error(`Push failed: ${e}`);
      throw e;
    }
  }

  static async commit(message: string, repoPath?: string): Promise<GitCommandResult> {
    try {
      const res = await invoke<GitCommandResult>("cmd_git_commit", { message, repoPath });
      if (res.success) {
          toast.success("Commit successful");
          triggerGraphReload();
      } else {
          toast.error(`Commit failed: ${res.stderr}`);
      }
      return res;
    } catch (e: any) {
      toast.error(`Commit failed: ${e}`);
      throw e;
    }
  }

  static async getPendingCommitsCount(repoPath?: string): Promise<number> {
    return invoke("cmd_get_pending_commits_count", { repoPath });
  }
}
