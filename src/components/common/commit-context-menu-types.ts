import type { GraphNode } from "../../lib/graph-layout";

export type ResetMode = "soft" | "mixed" | "hard";

export type CommitContextMenuState = {
  x: number;
  y: number;
  node: GraphNode;
  isHead: boolean;
  currentBranch: string;
  localBranches: string[];
  remoteBranches: string[];
  tags: string[];
};

export type CommitContextMenuAction =
  | { type: "pull" }
  | { type: "push" }
  | { type: "fetch" }
  | { type: "set-upstream" }
  | { type: "checkout-local"; branch: string }
  | { type: "checkout-remote"; remoteRef: string }
  | { type: "checkout-detached" }
  | { type: "create-branch-here" }
  | { type: "reset"; mode: ResetMode }
  | { type: "revert" }
  | { type: "rename-branch"; branch: string }
  | { type: "delete-local-branch"; branch: string }
  | { type: "delete-remote-branch"; remoteRef: string }
  | { type: "delete-local-and-remote"; branch: string; remoteRef: string }
  | { type: "copy-commit-sha" }
  | { type: "copy-branch-name"; branch: string }
  | { type: "create-patch-from-commit" }
  | { type: "create-tag"; annotated: boolean };

export type CommitContextActionHandler = (
  action: CommitContextMenuAction,
  menu: CommitContextMenuState,
) => void | Promise<void>;
