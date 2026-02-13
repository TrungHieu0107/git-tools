import { invoke } from "@tauri-apps/api/core";
import type { GitCommandResult } from "../types";
import { executeGitCommand } from "./command-executor";

type CheckoutMode = "switch" | "checkout";

export class BranchService {
  static async getBranches(includeRemote = false, repoPath?: string): Promise<string[]> {
    return invoke("cmd_get_git_branches", { includeRemote, repoPath });
  }

  static async getCurrentBranch(repoPath?: string): Promise<string> {
    return invoke("cmd_get_current_branch", { repoPath });
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
