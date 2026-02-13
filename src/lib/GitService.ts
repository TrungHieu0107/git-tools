import type { DiffStageLineTarget } from "./diff";
import type { BlameLine, CommitDiff, FileCommit, GitCommandResult } from "./types";
import { RepositoryService } from "./services/RepositoryService";
import { FileService } from "./services/FileService";
import { CommitService } from "./services/CommitService";
import { BranchService } from "./services/BranchService";
import { ConflictService } from "./services/ConflictService";
import { TerminalService } from "./services/TerminalService";

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

export interface CommitChangedFile {
  path: string;
  status: string;
}

export interface AppSettings {
  repos: RepoEntry[];
  active_repo_id: string | null;
  open_repo_ids: string[];
  excluded_files: string[];
  repo_filters: Record<string, string>;
  file_encodings?: Record<string, string>;
  gemini_api_token?: string | null;
  gemini_model?: string | null;
}

export class GitService {
  static async getSettings(): Promise<AppSettings> {
    return RepositoryService.getSettings();
  }

  static async setExcludedFiles(exclusions: string[]): Promise<AppSettings> {
    return RepositoryService.setExcludedFiles(exclusions);
  }

  static async setRepoFilter(repoId: string, filter: string): Promise<AppSettings> {
    return RepositoryService.setRepoFilter(repoId, filter);
  }

  static async setGeminiApiToken(token: string): Promise<AppSettings> {
    return RepositoryService.setGeminiApiToken(token);
  }

  static async setGeminiModel(model: string): Promise<AppSettings> {
    return RepositoryService.setGeminiModel(model);
  }

  static async getGeminiModels(token?: string): Promise<string[]> {
    return RepositoryService.getGeminiModels(token);
  }

  static async addRepo(name: string, path: string): Promise<AppSettings> {
    return RepositoryService.addRepo(name, path);
  }

  static async removeRepo(id: string): Promise<AppSettings> {
    return RepositoryService.removeRepo(id);
  }

  static async setActiveRepo(id: string): Promise<AppSettings> {
    return RepositoryService.setActiveRepo(id);
  }

  static async openRepo(id: string): Promise<AppSettings> {
    return RepositoryService.openRepo(id);
  }

  static async closeRepo(id: string): Promise<AppSettings> {
    return RepositoryService.closeRepo(id);
  }

  static async getActiveRepo(): Promise<RepoEntry | null> {
    return RepositoryService.getActiveRepo();
  }

  static async getConflicts(repoPath?: string): Promise<string[]> {
    return ConflictService.getConflicts(repoPath);
  }

  static async getConflictFile(path: string, repoPath?: string): Promise<ConflictFile> {
    return ConflictService.getConflictFile(path, repoPath);
  }

  static async resolveOurs(path: string, repoPath?: string): Promise<void> {
    return ConflictService.resolveOurs(path, repoPath);
  }

  static async resolveTheirs(path: string, repoPath?: string): Promise<void> {
    return ConflictService.resolveTheirs(path, repoPath);
  }

  static async markResolved(path: string, repoPath?: string): Promise<void> {
    return ConflictService.markResolved(path, repoPath);
  }

  static async writeFile(path: string, content: string, repoPath?: string): Promise<void> {
    return ConflictService.writeFile(path, content, repoPath);
  }

  static async checkConflictState(repoPath?: string): Promise<boolean> {
    return ConflictService.checkConflictState(repoPath);
  }

  static async getStatusFiles(repoPath?: string): Promise<FileStatus[]> {
    return FileService.getStatusFiles(repoPath);
  }

  static async generateCommitMessage(repoPath?: string): Promise<string> {
    return CommitService.generateCommitMessage(repoPath);
  }

  static async getDiff(filePath: string, staged: boolean, repoPath?: string, encoding?: string): Promise<string> {
    return FileService.getDiff(filePath, staged, repoPath, encoding);
  }

  static async getFileBaseContent(
    filePath: string,
    staged: boolean,
    repoPath?: string,
    encoding?: string,
  ): Promise<string> {
    return FileService.getFileBaseContent(filePath, staged, repoPath, encoding);
  }

  static async getFileModifiedContent(
    filePath: string,
    staged: boolean,
    repoPath?: string,
    encoding?: string,
  ): Promise<string> {
    return FileService.getFileModifiedContent(filePath, staged, repoPath, encoding);
  }

  static async stageFile(path: string, repoPath?: string): Promise<void> {
    return FileService.stageFile(path, repoPath);
  }

  static async unstageFile(path: string, repoPath?: string): Promise<void> {
    return FileService.unstageFile(path, repoPath);
  }

  static async stageAll(repoPath?: string): Promise<void> {
    return FileService.stageAll(repoPath);
  }

  static async unstageAll(repoPath?: string): Promise<void> {
    return FileService.unstageAll(repoPath);
  }

  static async stageLine(path: string, line: DiffStageLineTarget, repoPath?: string): Promise<void> {
    return FileService.stageLine(path, line, repoPath);
  }

  static async unstageLine(path: string, line: DiffStageLineTarget, repoPath?: string): Promise<void> {
    return FileService.unstageLine(path, line, repoPath);
  }

  static async discardChanges(files: FileStatus[], repoPath?: string): Promise<void> {
    return FileService.discardChanges(files, repoPath);
  }

  static async stashFile(file: FileStatus, repoPath?: string): Promise<void> {
    return FileService.stashFile(file, repoPath);
  }

  static async stashAll(repoPath?: string): Promise<void> {
    return FileService.stashAll(repoPath);
  }

  static async openRepoFile(filePath: string, repoPath?: string): Promise<void> {
    return FileService.openRepoFile(filePath, repoPath);
  }

  static async ignoreFile(pattern: string, repoPath?: string): Promise<void> {
    return FileService.ignoreFile(pattern, repoPath);
  }

  static async openInDiffTool(filePath: string, staged: boolean, repoPath?: string): Promise<void> {
    return FileService.openInDiffTool(filePath, staged, repoPath);
  }

  static async openInEditor(filePath: string, repoPath?: string): Promise<void> {
    return FileService.openInEditor(filePath, repoPath);
  }

  static async showInFolder(filePath: string, repoPath?: string): Promise<void> {
    return FileService.showInFolder(filePath, repoPath);
  }

  static async createPatch(filePath: string, staged: boolean, repoPath?: string): Promise<string> {
    return FileService.createPatch(filePath, staged, repoPath);
  }

  static async deleteFile(filePath: string, repoPath?: string): Promise<void> {
    return FileService.deleteFile(filePath, repoPath);
  }

  static async getBranches(includeRemote = false, repoPath?: string): Promise<string[]> {
    return BranchService.getBranches(includeRemote, repoPath);
  }

  static async getCurrentBranch(repoPath?: string): Promise<string> {
    return BranchService.getCurrentBranch(repoPath);
  }

  static async switchBranch(branchName: string, repoPath?: string): Promise<GitCommandResult> {
    return BranchService.switchBranch(branchName, repoPath);
  }

  static async checkout(branchName: string, repoPath?: string): Promise<GitCommandResult> {
    return BranchService.checkout(branchName, repoPath);
  }

  static async checkoutNew(name: string, startPoint: string, repoPath?: string): Promise<GitCommandResult> {
    return BranchService.checkoutNew(name, startPoint, repoPath);
  }

  static async createBranch(name: string, base: string, repoPath?: string): Promise<GitCommandResult> {
    return BranchService.createBranch(name, base, repoPath);
  }

  static async getCommitGraph(limit: number, repoPath?: string): Promise<string> {
    return BranchService.getCommitGraph(limit, repoPath);
  }

  static async merge(branch: string, repoPath?: string): Promise<GitCommandResult> {
    return BranchService.merge(branch, repoPath);
  }

  static async fetch(repoPath?: string): Promise<GitCommandResult> {
    return BranchService.fetch(repoPath);
  }

  static async pull(repoPath?: string): Promise<GitCommandResult> {
    return BranchService.pull(repoPath);
  }

  static async push(repoPath?: string): Promise<GitCommandResult> {
    return BranchService.push(repoPath);
  }

  static async commit(message: string, repoPath?: string): Promise<GitCommandResult> {
    return CommitService.commit(message, repoPath);
  }

  static async getPendingCommitsCount(repoPath?: string): Promise<number> {
    return CommitService.getPendingCommitsCount(repoPath);
  }

  static async getFileHistory(filePath: string, limit = 100, repoPath?: string): Promise<FileCommit[]> {
    return CommitService.getFileHistory(filePath, limit, repoPath);
  }

  static async getBlame(filePath: string, repoPath?: string): Promise<BlameLine[]> {
    return CommitService.getBlame(filePath, repoPath);
  }

  static async searchRepoFiles(pattern?: string, repoPath?: string): Promise<string[]> {
    return CommitService.searchRepoFiles(pattern, repoPath);
  }

  static async getCommitChangedFiles(commitHash: string, repoPath?: string): Promise<CommitChangedFile[]> {
    return CommitService.getCommitChangedFiles(commitHash, repoPath);
  }

  static async getCommitFileDiff(commitHash: string, filePath: string, repoPath?: string): Promise<GitCommandResult> {
    return CommitService.getCommitFileDiff(commitHash, filePath, repoPath);
  }

  static async getCommitDiff(
    commitHash: string,
    repoPath?: string,
    filePath?: string,
    encoding?: string,
  ): Promise<CommitDiff> {
    return CommitService.getCommitDiff(commitHash, repoPath, filePath, encoding);
  }

  static async getFileAtCommit(
    commitHash: string,
    filePath: string,
    repoPath?: string,
    encoding?: string,
  ): Promise<string> {
    return CommitService.getFileAtCommit(commitHash, filePath, repoPath, encoding);
  }

  static async startTerminal(repoPath: string): Promise<void> {
    return TerminalService.startTerminal(repoPath);
  }

  static async writeTerminal(repoPath: string, input: string): Promise<void> {
    return TerminalService.writeTerminal(repoPath, input);
  }

  static async stopTerminal(repoPath: string): Promise<void> {
    return TerminalService.stopTerminal(repoPath);
  }
}
