import type { DiffHunk as BackendDiffHunk } from "../../lib/types";
import type { DiffHunk, DiffResult, DiffStageLineTarget } from "../../lib/diff";

export interface DiffViewerBaseProps {
  diffResult: DiffResult | null;
  hunks?: DiffHunk[] | null;
  commitHunks?: BackendDiffHunk[];
  canStageLine?: boolean;
  onStageLine?: (line: DiffStageLineTarget) => void | Promise<void>;
  canUnstageLine?: boolean;
  onUnstageLine?: (line: DiffStageLineTarget) => void | Promise<void>;
}
