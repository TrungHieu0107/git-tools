import { invoke } from "@tauri-apps/api/core";
import type { DiffStageLineTarget } from "../diff";
import type { FileStatus } from "../GitService";
import { executeCommand } from "./command-executor";

export class FileService {
  static async getStatusFiles(repoPath?: string): Promise<FileStatus[]> {
    return invoke("cmd_get_status_files", { repoPath });
  }

  static async getDiff(filePath: string, staged: boolean, repoPath?: string, encoding?: string): Promise<string> {
    return invoke("cmd_get_diff_file", { filePath, staged, repoPath, encoding });
  }

  static async getFileBaseContent(
    filePath: string,
    staged: boolean,
    repoPath?: string,
    encoding?: string,
  ): Promise<string> {
    return invoke("cmd_get_file_base_content", { filePath, staged, repoPath, encoding });
  }

  static async getFileModifiedContent(
    filePath: string,
    staged: boolean,
    repoPath?: string,
    encoding?: string,
  ): Promise<string> {
    return invoke("cmd_get_file_modified_content", { filePath, staged, repoPath, encoding });
  }

  static async stageFile(path: string, repoPath?: string): Promise<void> {
    await executeCommand<void>("cmd_git_add", { path, repoPath }, `Staged ${path}`, "Stage failed");
  }

  static async unstageFile(path: string, repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_git_unstage",
      { path, repoPath },
      `Unstaged ${path}`,
      "Unstage failed",
    );
  }

  static async stageAll(repoPath?: string): Promise<void> {
    await executeCommand<void>("cmd_git_add_all", { repoPath }, "Staged all files", "Stage all failed");
  }

  static async unstageAll(repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_git_unstage_all",
      { repoPath },
      "Unstaged all files",
      "Unstage all failed",
    );
  }

  static async stageLine(path: string, line: DiffStageLineTarget, repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_git_stage_line",
      { path, line, repoPath },
      "Staged selected line",
      "Stage line failed",
    );
  }

  static async unstageLine(path: string, line: DiffStageLineTarget, repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_git_unstage_line",
      { path, line, repoPath },
      "Unstaged selected line",
      "Unstage line failed",
    );
  }

  static async discardChanges(files: FileStatus[], repoPath?: string): Promise<void> {
    if (files.length === 0) {
      return;
    }

    const successMessage = files.length === 1 ? `Discarded changes in ${files[0].path}` : "Discarded all changes";
    await executeCommand<void>(
      "cmd_git_discard_changes",
      { files, repoPath },
      successMessage,
      "Discard failed",
    );
  }

  static async stashFile(file: FileStatus, repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_git_stash_file",
      { file, repoPath },
      `Stashed ${file.path}`,
      "Stash failed",
      { reloadGraph: true },
    );
  }

  static async stashAll(repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_git_stash_all",
      { repoPath },
      "Stashed all changes",
      "Stash all failed",
      { reloadGraph: true },
    );
  }

  static async openRepoFile(filePath: string, repoPath?: string): Promise<void> {
    await executeCommand<void>("cmd_open_repo_file", { filePath, repoPath }, "", "Open file failed");
  }

  static async ignoreFile(pattern: string, repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_git_ignore_file",
      { pattern, repoPath },
      `Added ${pattern} to .gitignore`,
      "Ignore file failed",
    );
  }

  static async openInDiffTool(filePath: string, staged: boolean, repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_open_in_diff_tool",
      { filePath, staged, repoPath },
      "",
      "Open in external diff tool failed",
    );
  }

  static async openInEditor(filePath: string, repoPath?: string): Promise<void> {
    await executeCommand<void>(
      "cmd_open_in_editor",
      { filePath, repoPath },
      "",
      "Open in external editor failed",
    );
  }

  static async showInFolder(filePath: string, repoPath?: string): Promise<void> {
    await executeCommand<void>("cmd_show_in_folder", { filePath, repoPath }, "", "Show in folder failed");
  }

  static async createPatch(filePath: string, staged: boolean, repoPath?: string): Promise<string> {
    return executeCommand<string>(
      "cmd_create_patch",
      { filePath, staged, repoPath },
      "",
      "Create patch failed",
    );
  }

  static async deleteFile(filePath: string, repoPath?: string): Promise<void> {
    await executeCommand<void>("cmd_delete_file", { filePath, repoPath }, `Deleted ${filePath}`, "Delete file failed");
  }
}
