import type { GraphNode } from "../../lib/graph-layout";

export type StashCommitContextMenuState = {
  x: number;
  y: number;
  node: GraphNode;
};

export type StashCommitContextMenuAction =
  | { type: "apply-stash" }
  | { type: "pop-stash" }
  | { type: "delete-stash" }
  | { type: "edit-stash-message" }
  | { type: "share-stash-cloud-patch" }
  | { type: "hide" };

export type StashCommitContextActionHandler = (
  action: StashCommitContextMenuAction,
  menu: StashCommitContextMenuState,
) => void | Promise<void>;
