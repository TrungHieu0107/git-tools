import { invoke } from "@tauri-apps/api/core";
import type { ConflictFile, GitOperationState } from "../GitService";

export class ConflictService {
  static async getConflicts(repoPath?: string): Promise<string[]> {
    return invoke("cmd_get_conflicts", { repoPath });
  }

  static async getConflictFile(path: string, repoPath?: string, encoding?: string): Promise<ConflictFile> {
    return invoke("cmd_get_conflict_file", { path, encoding, repoPath });
  }

  static async resolveOurs(path: string, repoPath?: string): Promise<void> {
    return invoke("cmd_resolve_ours", { path, repoPath });
  }

  static async resolveTheirs(path: string, repoPath?: string): Promise<void> {
    return invoke("cmd_resolve_theirs", { path, repoPath });
  }

  static async markResolved(path: string, repoPath?: string): Promise<void> {
    return invoke("cmd_mark_resolved", { path, repoPath });
  }

  static async writeFile(path: string, content: string, repoPath?: string, encoding?: string): Promise<void> {
    return invoke("cmd_write_file", { path, content, encoding, repoPath });
  }

  static async checkConflictState(repoPath?: string): Promise<boolean> {
    return invoke("cmd_check_conflict_state", { repoPath });
  }

  static async getOperationState(repoPath?: string): Promise<GitOperationState> {
    return invoke("cmd_get_operation_state", { repoPath });
  }
}
