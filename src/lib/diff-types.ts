export type DiffLineType = "equal" | "added" | "removed" | "modified";

export interface DiffLine {
  content: string;
  type: DiffLineType;
  lineNumber: number | null;
}

export interface DiffResult {
  left: DiffLine[];
  right: DiffLine[];
}

export interface DiffHunk {
  id: string;
  startIndex: number;
  endIndex: number;
  lines: { left: DiffLine; right: DiffLine }[];
}

export interface InlineDiffLine {
  content: string;
  type: DiffLineType;
  oldLineNumber: number | null;
  newLineNumber: number | null;
  sourceIndex: number;
}

export interface DiffStageLineTarget {
  oldLineNumber: number | null;
  newLineNumber: number | null;
}
