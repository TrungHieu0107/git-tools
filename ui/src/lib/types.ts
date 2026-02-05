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
