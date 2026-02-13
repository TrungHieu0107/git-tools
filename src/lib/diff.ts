import type { DiffHunk as BackendDiffHunk } from "./types";
import { backtrackEditOps, computeLCS, type EditOp } from "./lcs";
import { escapeHtml, isLargeFile, mapLineType } from "./diff-utils";
import { parseGitDiff as parseGitDiffImpl } from "./git-diff-parser";
import type { DiffLine, DiffHunk, DiffResult, InlineDiffLine } from "./diff-types";

export type {
  DiffLineType,
  DiffLine,
  DiffResult,
  DiffHunk,
  InlineDiffLine,
  DiffStageLineTarget,
} from "./diff-types";
export type { ParsedDiff } from "./git-diff-parser";
export { escapeHtml, isLargeFile };

function normalizeLines(content: string): string[] {
  const normalized = content.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const lines = normalized.split("\n");
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }
  return lines;
}

function computeLCSTable(oldLines: string[], newLines: string[]): number[][] {
  return computeLCS(oldLines, newLines);
}

function extractEditOperations(dp: number[][], oldLines: string[], newLines: string[]): EditOp[] {
  return backtrackEditOps(dp, oldLines, newLines);
}

function buildAlignedDiff(ops: EditOp[], _oldLines: string[], _newLines: string[]): DiffResult {
  const left: DiffLine[] = [];
  const right: DiffLine[] = [];

  let opIdx = 0;
  while (opIdx < ops.length) {
    const op = ops[opIdx];

    if (op.kind === "equal") {
      left.push({ content: op.baseLine, type: "equal", lineNumber: op.baseIdx });
      right.push({ content: op.baseLine, type: "equal", lineNumber: op.modIdx });
      opIdx++;
      continue;
    }

    const removedBatch: Extract<EditOp, { kind: "removed" }>[] = [];
    const addedBatch: Extract<EditOp, { kind: "added" }>[] = [];

    while (opIdx < ops.length && ops[opIdx].kind === "removed") {
      removedBatch.push(ops[opIdx] as Extract<EditOp, { kind: "removed" }>);
      opIdx++;
    }
    while (opIdx < ops.length && ops[opIdx].kind === "added") {
      addedBatch.push(ops[opIdx] as Extract<EditOp, { kind: "added" }>);
      opIdx++;
    }

    const pairCount = Math.min(removedBatch.length, addedBatch.length);

    for (let k = 0; k < pairCount; k++) {
      left.push({
        content: removedBatch[k].baseLine,
        type: "modified",
        lineNumber: removedBatch[k].baseIdx,
      });
      right.push({
        content: addedBatch[k].modLine,
        type: "modified",
        lineNumber: addedBatch[k].modIdx,
      });
    }

    for (let k = pairCount; k < removedBatch.length; k++) {
      left.push({
        content: removedBatch[k].baseLine,
        type: "removed",
        lineNumber: removedBatch[k].baseIdx,
      });
      right.push({ content: "", type: "removed", lineNumber: null });
    }

    for (let k = pairCount; k < addedBatch.length; k++) {
      left.push({ content: "", type: "added", lineNumber: null });
      right.push({
        content: addedBatch[k].modLine,
        type: "added",
        lineNumber: addedBatch[k].modIdx,
      });
    }
  }

  return { left, right };
}

export function computeDiff(baseContent: string, modifiedContent: string): DiffResult {
  const oldLines = normalizeLines(baseContent);
  const newLines = normalizeLines(modifiedContent);
  const dp = computeLCSTable(oldLines, newLines);
  const ops = extractEditOperations(dp, oldLines, newLines);
  return buildAlignedDiff(ops, oldLines, newLines);
}

export function extractHunks(result: DiffResult, contextLines: number = 3): DiffHunk[] {
  const { left, right } = result;
  const len = left.length;

  const changeIndices: number[] = [];
  for (let i = 0; i < len; i++) {
    if (left[i].type !== "equal" || right[i].type !== "equal") {
      changeIndices.push(i);
    }
  }

  if (changeIndices.length === 0) return [];

  interface RawRegion {
    start: number;
    end: number;
  }

  const rawRegions: RawRegion[] = [];
  let regionStart = changeIndices[0];
  let regionEnd = changeIndices[0] + 1;

  for (let i = 1; i < changeIndices.length; i++) {
    const gap = changeIndices[i] - regionEnd;
    if (gap <= contextLines * 2) {
      regionEnd = changeIndices[i] + 1;
    } else {
      rawRegions.push({ start: regionStart, end: regionEnd });
      regionStart = changeIndices[i];
      regionEnd = changeIndices[i] + 1;
    }
  }
  rawRegions.push({ start: regionStart, end: regionEnd });

  return rawRegions.map((region, idx) => {
    const expandedStart = Math.max(0, region.start - contextLines);
    const expandedEnd = Math.min(len, region.end + contextLines);

    const lines: { left: DiffLine; right: DiffLine }[] = [];
    for (let i = expandedStart; i < expandedEnd; i++) {
      lines.push({ left: left[i], right: right[i] });
    }

    return {
      id: `hunk-${idx}`,
      startIndex: expandedStart,
      endIndex: expandedEnd,
      lines,
    };
  });
}

export function toInlineView(result: DiffResult): InlineDiffLine[] {
  const { left, right } = result;
  const lines: InlineDiffLine[] = [];

  for (let i = 0; i < left.length; i++) {
    const l = left[i];
    const r = right[i];

    if (l.type === "equal") {
      lines.push({
        content: l.content,
        type: "equal",
        oldLineNumber: l.lineNumber,
        newLineNumber: r.lineNumber,
        sourceIndex: i,
      });
    } else if (l.type === "modified" && r.type === "modified") {
      lines.push({
        content: l.content,
        type: "removed",
        oldLineNumber: l.lineNumber,
        newLineNumber: null,
        sourceIndex: i,
      });
      lines.push({
        content: r.content,
        type: "added",
        oldLineNumber: null,
        newLineNumber: r.lineNumber,
        sourceIndex: i,
      });
    } else if (l.type === "removed") {
      lines.push({
        content: l.content,
        type: "removed",
        oldLineNumber: l.lineNumber,
        newLineNumber: null,
        sourceIndex: i,
      });
    } else if (l.type === "added") {
      lines.push({
        content: r.content,
        type: "added",
        oldLineNumber: null,
        newLineNumber: r.lineNumber,
        sourceIndex: i,
      });
    }
  }

  return lines;
}

export function mapBackendHunksToInline(hunks: BackendDiffHunk[]): InlineDiffLine[] {
  const lines: InlineDiffLine[] = [];

  for (const hunk of hunks) {
    lines.push({
      content: `@@ -${hunk.oldStart},? +${hunk.newStart},? @@`,
      type: "equal",
      oldLineNumber: null,
      newLineNumber: null,
      sourceIndex: 0,
    });

    for (const line of hunk.lines) {
      lines.push({
        content: line.content,
        type: mapLineType(line.type),
        oldLineNumber: line.oldLineNumber ?? null,
        newLineNumber: line.newLineNumber ?? null,
        sourceIndex: 0,
      });
    }
  }

  return lines;
}

export function mapBackendHunksToSideBySide(hunks: BackendDiffHunk[]): DiffHunk[] {
  return hunks.map((h) => {
    const pairedLines: { left: DiffLine; right: DiffLine }[] = [];

    let i = 0;
    while (i < h.lines.length) {
      const line = h.lines[i];

      if (line.type === "context") {
        pairedLines.push({
          left: { content: line.content, type: "equal", lineNumber: line.oldLineNumber ?? null },
          right: { content: line.content, type: "equal", lineNumber: line.newLineNumber ?? null },
        });
        i++;
        continue;
      }

      const removes: typeof line[] = [];
      const adds: typeof line[] = [];
      let j = i;
      while (j < h.lines.length && (h.lines[j].type === "remove" || h.lines[j].type === "add")) {
        if (h.lines[j].type === "remove") removes.push(h.lines[j]);
        else adds.push(h.lines[j]);
        j++;
      }

      const max = Math.max(removes.length, adds.length);
      for (let k = 0; k < max; k++) {
        const rem = removes[k];
        const add = adds[k];

        pairedLines.push({
          left: rem
            ? { content: rem.content, type: "removed", lineNumber: rem.oldLineNumber ?? null }
            : { content: "", type: "equal", lineNumber: null },
          right: add
            ? { content: add.content, type: "added", lineNumber: add.newLineNumber ?? null }
            : { content: "", type: "equal", lineNumber: null },
        });
      }

      i = j;
    }

    return {
      id: h.id,
      startIndex: 0,
      endIndex: 0,
      lines: pairedLines,
    };
  });
}

export const parseGitDiff = parseGitDiffImpl;
