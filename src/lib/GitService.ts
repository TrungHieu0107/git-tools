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
  static async getConflicts(): Promise<string[]> {
    return invoke("cmd_get_conflicts");
  }

  /**
   * Get conflict file details (base, ours, theirs)
   */
  static async getConflictFile(path: string): Promise<ConflictFile> {
    return invoke("cmd_get_conflict_file", { path });
  }

  /**
   * Resolve a conflict by choosing "ours"
   */
  static async resolveOurs(path: string): Promise<void> {
    return invoke("cmd_resolve_ours", { path });
  }

  /**
   * Resolve a conflict by choosing "theirs"
   */
  static async resolveTheirs(path: string): Promise<void> {
    return invoke("cmd_resolve_theirs", { path });
  }

  /**
   * Mark a file as resolved (git add)
   */
  static async markResolved(path: string): Promise<void> {
    return invoke("cmd_mark_resolved", { path });
  }

  /**
   * Write content to a file (for manual conflict resolution)
   */
  static async writeFile(path: string, content: string): Promise<void> {
    return invoke("cmd_write_file", { path, content });
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
}
