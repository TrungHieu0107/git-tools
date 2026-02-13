export type GitParsedOutput = {
  type: "status";
  isClean: boolean;
};

export type GitCommandOutput = {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  parsed: GitParsedOutput | null;
};

export type GitCommandRequest = {
  repoPath: string;
  subcommand: string[];
};

export type GitCommandError =
  | { type: "invalidRepoPath"; path: string }
  | { type: "notRepository"; path: string }
  | { type: "mergeConflict" }
  | { type: "commandFailed"; code: number | null; stderr: string }
  | { type: "io"; message: string };

export type GitCommandType =
  | "Checkout"
  | "Merge"
  | "Commit"
  | "Pull"
  | "Push"
  | "Fetch"
  | "Branch"
  | "Other";

export type GitCommandResult = {
  success: boolean;
  stdout: string;
  stderr: string;
  exitCode: number;
  commandType: GitCommandType;
};

export interface FileCommit {
  hash: string;
  author: string;
  date: string;
  message: string;
}

export interface BlameLine {
  commitHash: string;
  author: string;
  date: string;
  lineNumber: number;
  content: string;
}

export type DiffLineType = "context" | "add" | "remove";

export interface DiffLine {
  type: DiffLineType;
  content: string;
  oldLineNumber?: number;
  newLineNumber?: number;
}

export interface DiffHunk {
  id: string;
  oldStart: number;
  newStart: number;
  lines: DiffLine[];
}

export interface DiffFile {
  path: string;
  status: string;
  hunks: DiffHunk[];
}

export interface CommitDiff {
  commitHash: string;
  parentHash?: string;
  files: DiffFile[];
}

