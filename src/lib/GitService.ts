import { invoke } from "@tauri-apps/api/core";

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

export interface AppSettings {
  repos: RepoEntry[];
  active_repo_id: string | null;
}

export class GitService {
  // --- Settings Commands ---

  static async getSettings(): Promise<AppSettings> {
    return invoke("cmd_get_settings");
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
  // --- Branch Management ---

  static async getBranches(includeRemote = false, repoPath?: string): Promise<string[]> {
    return invoke("cmd_get_git_branches", { includeRemote, repoPath });
  }

  static async getCurrentBranch(repoPath?: string): Promise<string> {
    return invoke("cmd_get_current_branch", { repoPath });
  }

  static async switchBranch(branchName: string, repoPath?: string): Promise<string> {
    return invoke("cmd_git_switch_branch", { branchName, repoPath });
  }

  static async checkoutNew(name: string, startPoint: string, repoPath?: string): Promise<string> {
    return invoke("cmd_git_checkout_new_branch", { name, startPoint, repoPath });
  }

  static async getCommitGraph(limit: number, repoPath?: string): Promise<string> {
    return invoke("cmd_get_commit_graph", { limit, repoPath });
  }

  static async merge(branch: string, repoPath?: string): Promise<string> {
    return invoke("cmd_git_merge", { branch, repoPath });
  }

  static async fetch(repoPath?: string): Promise<string> {
    return invoke("cmd_git_fetch", { repoPath });
  }

  static async pull(repoPath?: string): Promise<string> {
    return invoke("cmd_git_pull", { repoPath });
  }

  static async push(repoPath?: string): Promise<string> {
    return invoke("cmd_git_push", { repoPath });
  }

  static async getPendingCommitsCount(repoPath?: string): Promise<number> {
    return invoke("cmd_get_pending_commits_count", { repoPath });
  }
}
