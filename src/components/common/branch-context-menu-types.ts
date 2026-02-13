export type BranchContextMenuState = {
  x: number;
  y: number;
  payload: unknown;
  disableCheckout?: boolean;
  disableMerge?: boolean;
};
