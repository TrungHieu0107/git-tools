// Line-level diff engine using LCS (Longest Common Subsequence).
// Line-level diff engine using LCS (Longest Common Subsequence).
// Produces aligned left/right arrays for side-by-side rendering.
import type { DiffHunk as BackendDiffHunk } from "./types";

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

export interface DiffStageLineTarget {
  oldLineNumber: number | null;
  newLineNumber: number | null;
}

export function escapeHtml(unsafe: string): string {
  return unsafe
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
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

// ── Mappers for Backend Diffs ───────────────────────────────────

export function mapBackendHunksToInline(hunks: BackendDiffHunk[]): InlineDiffLine[] {
  const lines: InlineDiffLine[] = [];
  for (const hunk of hunks) {
    // Add a header/separator line
    lines.push({
        content: `@@ -${hunk.oldStart},? +${hunk.newStart},? @@`,
        type: 'equal', // Use equal to avoid coloring, maybe add special styling later
        oldLineNumber: null,
        newLineNumber: null,
        sourceIndex: 0
    });

    for (const line of hunk.lines) {
      lines.push({
        content: line.content,
        // Map types: context->equal, add->added, remove->removed
        type: line.type === 'context' ? 'equal' : line.type === 'add' ? 'added' : 'removed',
        oldLineNumber: line.oldLineNumber ?? null,
        newLineNumber: line.newLineNumber ?? null,
        sourceIndex: 0
      });
    }
  }
  return lines;
}

export function mapBackendHunksToSideBySide(hunks: BackendDiffHunk[]): DiffHunk[] {
  return hunks.map((h, index) => {
    const pairedLines: { left: DiffLine; right: DiffLine }[] = [];
    
    // Simple alignment strategy:
    // Iterate lines. 
    // If context: push to both.
    // If remove: push to left, right empty.
    // If add: push to right, left empty.
    // Try to group adjacent remove/add into "modified" rows if they appear sequentially?
    // Given the stream is flattened, we can't easily peek ahead without buffering.
    // We already have the whole list.
    
    let i = 0;
    while (i < h.lines.length) {
        const line = h.lines[i];
        
        if (line.type === 'context') {
            pairedLines.push({
                left: { content: line.content, type: 'equal', lineNumber: line.oldLineNumber ?? null },
                right: { content: line.content, type: 'equal', lineNumber: line.newLineNumber ?? null }
            });
            i++;
        } else {
            // Collect batch of removes and adds
            let removes: typeof line[] = [];
            let adds: typeof line[] = [];
            
            let j = i;
            while (j < h.lines.length && (h.lines[j].type === 'remove' || h.lines[j].type === 'add')) {
                if (h.lines[j].type === 'remove') removes.push(h.lines[j]);
                else adds.push(h.lines[j]);
                j++;
            }
            
            // Now distribute them
            // We want to align them if possible.
            // Just pair them up 1:1.
            const max = Math.max(removes.length, adds.length);
            for (let k = 0; k < max; k++) {
                const rem = removes[k];
                const add = adds[k];
                
                pairedLines.push({
                    left: rem 
                        ? { content: rem.content, type: 'removed', lineNumber: rem.oldLineNumber ?? null }
                        : { content: '', type: 'equal', lineNumber: null }, // empty cell
                    right: add
                        ? { content: add.content, type: 'added', lineNumber: add.newLineNumber ?? null }
                        : { content: '', type: 'equal', lineNumber: null }
                });
            }
            
            i = j;
        }
    }

    return {
        id: h.id,
        startIndex: 0,
        endIndex: 0,
        lines: pairedLines,
    };
  });
}

// ── Parsing Raw Git Diff ────────────────────────────────────────

export interface ParsedDiff {
    diff: DiffResult;
    hunks: DiffHunk[];
}

/**
 * Parses the raw output of `git show <commit> -- <path>` into a ParsedDiff.
 * This handles standard unified diff format.
 */
export function parseGitDiff(diffOutput: string): ParsedDiff {
    const lines = diffOutput.split('\n');
    const left: DiffLine[] = [];
    const right: DiffLine[] = [];
    const hunks: DiffHunk[] = [];

    let oldLn = 0;
    let newLn = 0;

    let inDiff = false;
    let currentHunk: DiffHunk | null = null;
    let hunkStartIndex = 0; // Index in the global left/right arrays where the current hunk starts

    for (const line of lines) {
        if (line.startsWith('@@ ')) {
            inDiff = true;
            
            // Close previous hunk if exists
            if (currentHunk) {
                currentHunk.endIndex = left.length;
                hunks.push(currentHunk);
            }

            // Parse hunk header: @@ -old,len +new,len @@
            // Example: @@ -1,4 +1,5 @@
            const parts = line.split(' ');
            if (parts.length >= 4) {
                // -old,len
                const oldPart = parts[1].substring(1); // remove '-'
                const oldBase = parseInt(oldPart.split(',')[0], 10);
                oldLn = isNaN(oldBase) ? 0 : oldBase;

                // +new,len
                const newPart = parts[2].substring(1); // remove '+'
                const newBase = parseInt(newPart.split(',')[0], 10);
                newLn = isNaN(newBase) ? 0 : newBase;
            }

            hunkStartIndex = left.length;
            currentHunk = {
                id: `hunk-${hunks.length}`,
                startIndex: hunkStartIndex,
                endIndex: 0, // Will set when hunk closes or finishes
                lines: []
            };
            continue;
        }

        if (!inDiff) continue;

        let leftLine: DiffLine | null = null;
        let rightLine: DiffLine | null = null;

        if (line.startsWith('+')) {
            // Added
            leftLine = { content: "", type: "added", lineNumber: null };
            rightLine = { content: line.substring(1), type: "added", lineNumber: newLn++ };
        } else if (line.startsWith('-')) {
            // Removed
            leftLine = { content: line.substring(1), type: "removed", lineNumber: oldLn++ };
            rightLine = { content: "", type: "removed", lineNumber: null };
        } else if (line.startsWith(' ') || line === '') {
            // Context
            const content = line.startsWith(' ') ? line.substring(1) : line;
            leftLine = { content, type: "equal", lineNumber: oldLn++ };
            rightLine = { content, type: "equal", lineNumber: newLn++ };
        } else if (line.startsWith('\\ No newline at end of file')) {
            // Ignore
        } else {
            // Check if it's the start of a diff (diff --git ...)
            // If subsequent file in a multi-file diff (not expected here based on single file command, but good safety)
            if (line.startsWith('diff --git')) {
                inDiff = false;
                if (currentHunk) {
                     currentHunk.endIndex = left.length;
                     hunks.push(currentHunk);
                     currentHunk = null;
                }
            }
            // Otherwise unknown junk
        }

        if (leftLine && rightLine) {
            left.push(leftLine);
            right.push(rightLine);
            if (currentHunk) {
                currentHunk.lines.push({ left: leftLine, right: rightLine });
            }
        }
    }

    // Close last hunk
    if (currentHunk) {
        currentHunk.endIndex = left.length;
        hunks.push(currentHunk);
    }
    
    // If no hunks were found (e.g. empty diff or full file?), we might want to treat whole thing as one hunk?
    // But usually parseGitDiff is for "git show", which guarantees diff output.
    // If it's a new file (all additions), it usually has @@ -0,0 +1,5 @@.
    
    // Optimization: Collapse "modified" blocks (sequential remove then add) within the global left/right arrays
    // AND within the hunk lines.
    // This allows Side-by-Side to look good.
    // We can run `collapseDiff` on the global arrays.
    // For Hunks, we would need to run collapse on each hunk's lines too if we want them to look good in Side-by-Side mode INSIDE the Hunk view.
    
    // For now, let's keep it simple. `collapseDiff` returns new arrays.
    const collapsedGlobal = collapseDiff(left, right);
    
    // We should probably reconstruct hunks from the collapsed result to ensure consistency?
    // Or just accept that "Hunk Mode" might show disjoint add/remove if we don't collapse them specifically.
    // Given the requirement "Diff content must ... differ only in presentation", consistency is key.
    
    // Let's rely on `collapseDiff` for the global `diff` result (used by SideBySide and Inline).
    // For `hunks`, we can either leave them uncollapsed (pure patch view) or collapse them.
    // SideBySide view generally wants collapsed. Hunk view (patch) often keeps them separate.
    // But "Hunk Mode" in tools like Kraken often just means "Grouped".
    // Let's return the uncollapsed hunks for now, as they represent the patch structure accurately.
    // We can improve Hunk visualization later if needed.
    
    return {
        diff: collapsedGlobal,
        hunks
    };
}

function collapseDiff(left: DiffLine[], right: DiffLine[]): DiffResult {
    const newLeft: DiffLine[] = [];
    const newRight: DiffLine[] = [];
    
    let i = 0;
    while (i < left.length) {
        // Look for block of "removed" (left) / "empty" (right)
        if (left[i].type === 'removed' && right[i].type === 'removed') { // right[i].type is "removed" (empty/padding) from our parser
             // Buffer removals
             let remStart = i;
             while (i < left.length && left[i].type === 'removed') i++;
             let remEnd = i;
             
             // Now check next lines for additions (left=added/empty, right=added)
             let addStart = i;
             while (i < left.length && right[i].type === 'added') i++;
             let addEnd = i;
             
             // Setup for merging
             const remCount = remEnd - remStart;
             const addCount = addEnd - addStart;
             const common = Math.min(remCount, addCount);
             
             // Push collapsed pairs (Modified)
             for (let k = 0; k < common; k++) {
                 newLeft.push({ ...left[remStart + k], type: 'modified' });
                 newRight.push({ ...right[addStart + k], type: 'modified' });
             }
             
             // Push remaining removals
             for (let k = common; k < remCount; k++) {
                 newLeft.push(left[remStart + k]);
                 newRight.push(right[remStart + k]);
             }
             
             // Push remaining additions
             for (let k = common; k < addCount; k++) {
                 newLeft.push(left[addStart + k]);
                 newRight.push(right[addStart + k]);
             }
             
             continue;
        }
        
        // Context or single side
        newLeft.push(left[i]);
        newRight.push(right[i]);
        i++;
    }
    
    return { left: newLeft, right: newRight };
}
