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
   * Get merge state
   */
  static async getMergeState(): Promise<any> {
    return invoke("cmd_get_merge_state");
  }

  /**
   * Continue the current operation (merge/rebase/etc)
   */
  static async continueOp(): Promise<void> {
    return invoke("cmd_continue_op");
  }

  /**
   * Abort the current operation
   */
  static async abortOp(): Promise<void> {
    return invoke("cmd_abort_op");
  }

  /**
   * Check if the repo is in a conflict state (merge/rebase/etc AND unmerged files)
   */
  static async checkConflictState(repoPath?: string): Promise<boolean> {
    return invoke("cmd_check_conflict_state", { repoPath });
  }
}
