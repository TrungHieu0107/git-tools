import type { GraphNode } from "../../lib/graph-layout";

export type BranchContextMenuState = {
  x: number;
  y: number;
  branchName: string;
  branchType: "branch" | "remote";
  commitHash: string;
  currentBranch: string;
  node: GraphNode;
};

export type BranchContextMenuAction =
  | { type: "checkout" }
  | { type: "merge" }
  | { type: "rebase" }
  | { type: "interactive-rebase" }
  | { type: "cherry-pick" }
  | { type: "create-branch-here" }
  | { type: "reset"; mode: "soft" | "mixed" | "hard" }
  | { type: "revert" }
  | { type: "delete" }
  | { type: "copy-branch-name" }
  | { type: "copy-commit-sha" }
  | { type: "create-tag"; annotated: boolean };

export type BranchContextActionHandler = (
  action: BranchContextMenuAction,
  menu: BranchContextMenuState,
) => void | Promise<void>;
