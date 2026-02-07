// Line-level diff engine using LCS (Longest Common Subsequence).
// Produces aligned left/right arrays for side-by-side rendering.

export type DiffLineType = "equal" | "added" | "removed" | "modified";

export interface DiffLine {
  content: string;
  type: DiffLineType;
  lineNumber: number | null; // null = padding row (no real line)
}

export interface DiffResult {
  left: DiffLine[];
  right: DiffLine[];
}

export interface DiffHunk {
  id: string; // "hunk-0", "hunk-1" — stable DOM anchor for scrollIntoView
  startIndex: number; // index into DiffResult.left/right arrays
  endIndex: number; // exclusive end index
  lines: { left: DiffLine; right: DiffLine }[];
}

export interface InlineDiffLine {
  content: string;
  type: DiffLineType;
  oldLineNumber: number | null;
  newLineNumber: number | null;
  sourceIndex: number; // index into DiffResult arrays — maps to hunk ranges
}

const MAX_DIFF_LINES = 10_000;

/** Quick check to avoid O(n*m) blow-up on huge files. */
export function isLargeFile(content: string): boolean {
  let count = 0;
  for (let i = 0; i < content.length; i++) {
    if (content[i] === "\n") count++;
    if (count > MAX_DIFF_LINES) return true;
  }
  return false;
}

type EditOp =
  | { kind: "equal"; baseLine: string; baseIdx: number; modIdx: number }
  | { kind: "removed"; baseLine: string; baseIdx: number }
  | { kind: "added"; modLine: string; modIdx: number };

/**
 * Compute a side-by-side diff between two file contents.
 *
 * Returns two aligned arrays (same length) with padding lines inserted
 * so that corresponding rows always line up visually.
 */
export function computeDiff(
  baseContent: string,
  modifiedContent: string
): DiffResult {
  // Normalize line endings — critical on Windows where git may use LF
  // but the working directory file uses CRLF
  const normalizedBase = baseContent.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const normalizedMod = modifiedContent.replace(/\r\n/g, "\n").replace(/\r/g, "\n");

  const baseLines = normalizedBase.split("\n");
  const modLines = normalizedMod.split("\n");

  // Drop trailing empty element when the file ends with a newline,
  // so we don't render a phantom empty last line
  if (baseLines.length > 0 && baseLines[baseLines.length - 1] === "") {
    baseLines.pop();
  }
  if (modLines.length > 0 && modLines[modLines.length - 1] === "") {
    modLines.pop();
  }

  const m = baseLines.length;
  const n = modLines.length;

  // ── LCS dynamic-programming table ──────────────────────────────
  const dp: number[][] = Array.from({ length: m + 1 }, () =>
    new Array(n + 1).fill(0)
  );

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (baseLines[i - 1] === modLines[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  // ── Backtrack to produce edit operations ───────────────────────
  const ops: EditOp[] = [];
  let i = m;
  let j = n;

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && baseLines[i - 1] === modLines[j - 1]) {
      ops.push({
        kind: "equal",
        baseLine: baseLines[i - 1],
        baseIdx: i,
        modIdx: j,
      });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      ops.push({ kind: "added", modLine: modLines[j - 1], modIdx: j });
      j--;
    } else {
      ops.push({ kind: "removed", baseLine: baseLines[i - 1], baseIdx: i });
      i--;
    }
  }

  ops.reverse();

  // ── Build aligned left/right output ────────────────────────────
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

    // Collect consecutive removed/added runs so we can pair them as "modified"
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

    // Pair overlapping removed+added as "modified"
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

    // Remaining removals (no matching addition) → padding on the right
    for (let k = pairCount; k < removedBatch.length; k++) {
      left.push({
        content: removedBatch[k].baseLine,
        type: "removed",
        lineNumber: removedBatch[k].baseIdx,
      });
      right.push({ content: "", type: "removed", lineNumber: null });
    }

    // Remaining additions (no matching removal) → padding on the left
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

// ── Hunk extraction ─────────────────────────────────────────────

/**
 * Extract changed regions with surrounding context from a DiffResult.
 *
 * Adjacent hunks whose context would overlap are merged to avoid
 * showing the same lines twice.
 */
export function extractHunks(
  result: DiffResult,
  contextLines: number = 3
): DiffHunk[] {
  const { left, right } = result;
  const len = left.length;

  // Find every row index that contains a change
  const changeIndices: number[] = [];
  for (let i = 0; i < len; i++) {
    if (left[i].type !== "equal" || right[i].type !== "equal") {
      changeIndices.push(i);
    }
  }

  if (changeIndices.length === 0) return [];

  // Group consecutive changes into raw regions, merging when the gap
  // between two regions would be smaller than 2*contextLines (their
  // expanded context would overlap)
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

  // Expand each region by contextLines, clamped to array bounds
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

// ── Inline (unified) view ───────────────────────────────────────

/**
 * Flatten a side-by-side DiffResult into a single-column inline sequence.
 *
 * "modified" pairs become two lines: old (removed) then new (added),
 * matching how unified diffs traditionally display changes.
 */
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
      // Show old version (removed) then new version (added)
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
      // Pure removal — right side is padding
      lines.push({
        content: l.content,
        type: "removed",
        oldLineNumber: l.lineNumber,
        newLineNumber: null,
        sourceIndex: i,
      });
    } else if (l.type === "added") {
      // Pure addition — left side is padding, real content on right
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
