import { invoke } from "@tauri-apps/api/core";
import type { GitCommandResult } from "../types";
import { executeGitCommand } from "./command-executor";

type CheckoutMode = "switch" | "checkout";
type ResetMode = "soft" | "mixed" | "hard";

export class BranchService {
  static async getBranches(includeRemote = false, repoPath?: string): Promise<string[]> {
    return invoke("cmd_get_git_branches", { includeRemote, repoPath });
  }

  static async getCurrentBranch(repoPath?: string): Promise<string> {
    return invoke("cmd_get_current_branch", { repoPath });
  }

  static async getBranchTip(branchName: string, repoPath?: string): Promise<string> {
    return invoke("cmd_get_branch_tip", { branchName, repoPath });
  }

  static async switchBranch(branchName: string, repoPath?: string): Promise<GitCommandResult> {
    return BranchService.checkoutBranch("switch", branchName, repoPath);
  }

  static async checkout(branchName: string, repoPath?: string): Promise<GitCommandResult> {
    return BranchService.checkoutBranch("checkout", branchName, repoPath);
  }

  static async checkoutNew(name: string, startPoint: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_checkout_new_branch",
      { name, startPoint, repoPath },
      `Created branch '${name}'`,
      "Failed to create branch",
      { reloadGraph: true },
    );
  }

  static async createBranch(name: string, base: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_create_branch",
      { name, base, repoPath },
      `Branch '${name}' created successfully`,
      "Failed to create branch",
    );
  }

  static async getCommitGraph(limit: number, repoPath?: string): Promise<string> {
    return invoke("cmd_get_commit_graph", { limit, repoPath });
  }

  static async merge(branch: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_merge",
      { branch, repoPath },
      `Merged '${branch}'`,
      "Merge failed",
      { reloadGraph: true },
    );
  }

  static async rebase(branch: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_rebase",
      { branch, repoPath },
      `Rebased onto '${branch}'`,
      "Rebase failed",
      { reloadGraph: true },
    );
  }

  static async cherryPick(commitHash: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_cherry_pick",
      { commitHash, repoPath },
      `Cherry-picked ${commitHash.slice(0, 8)}`,
      "Cherry-pick failed",
      { reloadGraph: true },
    );
  }

  static async abortOperation(repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_abort_operation",
      { repoPath },
      "Aborted current git operation",
      "Abort failed",
      { reloadGraph: true },
    );
  }

  static async fetch(repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand("cmd_git_fetch", { repoPath }, "Fetch completed", "Fetch failed", {
      reloadGraph: true,
    });
  }

  static async pull(repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand("cmd_git_pull", { repoPath }, "Pull completed", "Pull failed", {
      reloadGraph: true,
    });
  }

  static async push(repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand("cmd_git_push", { repoPath }, "Push completed", "Push failed", {
      reloadGraph: true,
    });
  }

  static async revertCommit(commitHash: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_revert",
      { commitHash, repoPath },
      `Reverted ${commitHash.slice(0, 8)}`,
      "Revert failed",
      { reloadGraph: true },
    );
  }

  static async resetToCommit(commitHash: string, mode: ResetMode, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_reset",
      { commitHash, mode, repoPath },
      `Reset to ${commitHash.slice(0, 8)} (${mode})`,
      "Reset failed",
      { reloadGraph: true },
    );
  }

  static async createTag(
    tagName: string,
    commitHash: string,
    message?: string,
    repoPath?: string,
  ): Promise<GitCommandResult> {
    const isAnnotated = !!message?.trim();
    return executeGitCommand(
      "cmd_git_create_tag",
      { tagName, commitHash, message, repoPath },
      isAnnotated ? `Created annotated tag '${tagName}'` : `Created tag '${tagName}'`,
      "Create tag failed",
      { reloadGraph: true },
    );
  }

  static async deleteBranch(branchName: string, force = false, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_delete_branch",
      { branchName, force, repoPath },
      `Deleted branch '${branchName}'`,
      "Delete branch failed",
      { reloadGraph: true },
    );
  }

  static async deleteRemoteBranch(
    remote: string,
    branchName: string,
    repoPath?: string,
  ): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_delete_remote_branch",
      { remote, branchName, repoPath },
      `Deleted remote branch '${remote}/${branchName}'`,
      "Delete remote branch failed",
      { reloadGraph: true },
    );
  }

  static async renameBranch(oldName: string, newName: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_rename_branch",
      { oldName, newName, repoPath },
      `Renamed branch '${oldName}' to '${newName}'`,
      "Rename branch failed",
      { reloadGraph: true },
    );
  }

  static async setUpstream(branchName: string, upstream: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_set_upstream",
      { branchName, upstream, repoPath },
      `Set upstream of '${branchName}' to '${upstream}'`,
      "Set upstream failed",
      { reloadGraph: true },
    );
  }

  static async createPatchFromCommit(commitHash: string, repoPath?: string): Promise<string> {
    return invoke("cmd_create_patch_from_commit", { commitHash, repoPath });
  }

  static async applyStash(commitHash: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_apply_stash",
      { commitHash, repoPath },
      `Applied stash ${commitHash.slice(0, 8)}`,
      "Apply stash failed",
      { reloadGraph: true },
    );
  }

  static async popStash(commitHash: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_pop_stash",
      { commitHash, repoPath },
      `Popped stash ${commitHash.slice(0, 8)}`,
      "Pop stash failed",
      { reloadGraph: true },
    );
  }

  static async deleteStash(commitHash: string, repoPath?: string): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_delete_stash",
      { commitHash, repoPath },
      `Deleted stash ${commitHash.slice(0, 8)}`,
      "Delete stash failed",
      { reloadGraph: true },
    );
  }

  static async editStashMessage(
    commitHash: string,
    message: string,
    repoPath?: string,
  ): Promise<GitCommandResult> {
    return executeGitCommand(
      "cmd_git_edit_stash_message",
      { commitHash, message, repoPath },
      `Updated stash message (${commitHash.slice(0, 8)})`,
      "Edit stash message failed",
      { reloadGraph: true },
    );
  }

  static async createPatchFromStash(commitHash: string, repoPath?: string): Promise<string> {
    return invoke("cmd_create_patch_from_stash", { commitHash, repoPath });
  }

  private static async checkoutBranch(
    mode: CheckoutMode,
    branchName: string,
    repoPath?: string,
  ): Promise<GitCommandResult> {
    const command = mode === "switch" ? "cmd_git_switch_branch" : "cmd_git_checkout";
    const params = mode === "switch" ? { branchName, repoPath } : { branch: branchName, repoPath };
    const successMessage =
      mode === "switch" ? `Switched to branch '${branchName}'` : `Checked out branch '${branchName}'`;
    const errorMessage = mode === "switch" ? "Failed to switch branch" : "Checkout failed";

    return executeGitCommand(command, params, successMessage, errorMessage, {
      reloadGraph: true,
    });
  }
}
