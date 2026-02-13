import type { DiffHunk, DiffLine, DiffResult } from "./diff-types";

export interface ParsedDiff {
  diff: DiffResult;
  hunks: DiffHunk[];
}

type ParsedLineType = "add" | "remove" | "context" | "metadata";

export function parseHunkHeader(line: string): { oldStart: number; newStart: number } | null {
  if (!line.startsWith("@@ ")) {
    return null;
  }

  const parts = line.split(" ");
  if (parts.length < 4) {
    return null;
  }

  const oldPart = parts[1].substring(1);
  const oldBase = parseInt(oldPart.split(",")[0], 10);
  const newPart = parts[2].substring(1);
  const newBase = parseInt(newPart.split(",")[0], 10);

  return {
    oldStart: Number.isNaN(oldBase) ? 0 : oldBase,
    newStart: Number.isNaN(newBase) ? 0 : newBase,
  };
}

export function parseLineType(line: string): ParsedLineType {
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "remove";
  if (line.startsWith(" ") || line === "") return "context";
  return "metadata";
}

export function parseGitDiff(diffOutput: string): ParsedDiff {
  const lines = diffOutput.split("\n");
  const left: DiffLine[] = [];
  const right: DiffLine[] = [];
  const hunks: DiffHunk[] = [];

  let oldLn = 0;
  let newLn = 0;
  let inDiff = false;
  let currentHunk: DiffHunk | null = null;

  for (const line of lines) {
    const hunkHeader = parseHunkHeader(line);
    if (hunkHeader) {
      inDiff = true;
      if (currentHunk) {
        currentHunk.endIndex = left.length;
        hunks.push(currentHunk);
      }

      oldLn = hunkHeader.oldStart;
      newLn = hunkHeader.newStart;
      currentHunk = {
        id: `hunk-${hunks.length}`,
        startIndex: left.length,
        endIndex: 0,
        lines: [],
      };
      continue;
    }

    if (!inDiff) {
      continue;
    }

    const lineType = parseLineType(line);
    let leftLine: DiffLine | null = null;
    let rightLine: DiffLine | null = null;

    if (lineType === "add") {
      leftLine = { content: "", type: "added", lineNumber: null };
      rightLine = { content: line.substring(1), type: "added", lineNumber: newLn++ };
    } else if (lineType === "remove") {
      leftLine = { content: line.substring(1), type: "removed", lineNumber: oldLn++ };
      rightLine = { content: "", type: "removed", lineNumber: null };
    } else if (lineType === "context") {
      const content = line.startsWith(" ") ? line.substring(1) : line;
      leftLine = { content, type: "equal", lineNumber: oldLn++ };
      rightLine = { content, type: "equal", lineNumber: newLn++ };
    } else if (line.startsWith("\\ No newline at end of file")) {
      continue;
    } else if (line.startsWith("diff --git")) {
      inDiff = false;
      if (currentHunk) {
        currentHunk.endIndex = left.length;
        hunks.push(currentHunk);
        currentHunk = null;
      }
      continue;
    }

    if (leftLine && rightLine) {
      left.push(leftLine);
      right.push(rightLine);
      if (currentHunk) {
        currentHunk.lines.push({ left: leftLine, right: rightLine });
      }
    }
  }

  if (currentHunk) {
    currentHunk.endIndex = left.length;
    hunks.push(currentHunk);
  }

  const collapsedGlobal = collapseDiff(left, right);

  return {
    diff: collapsedGlobal,
    hunks,
  };
}

function collapseDiff(left: DiffLine[], right: DiffLine[]): DiffResult {
  const newLeft: DiffLine[] = [];
  const newRight: DiffLine[] = [];

  let i = 0;
  while (i < left.length) {
    if (left[i].type === "removed" && right[i].type === "removed") {
      const remStart = i;
      while (i < left.length && left[i].type === "removed") i++;
      const remEnd = i;

      const addStart = i;
      while (i < left.length && right[i].type === "added") i++;
      const addEnd = i;

      const remCount = remEnd - remStart;
      const addCount = addEnd - addStart;
      const common = Math.min(remCount, addCount);

      for (let k = 0; k < common; k++) {
        newLeft.push({ ...left[remStart + k], type: "modified" });
        newRight.push({ ...right[addStart + k], type: "modified" });
      }

      for (let k = common; k < remCount; k++) {
        newLeft.push(left[remStart + k]);
        newRight.push(right[remStart + k]);
      }

      for (let k = common; k < addCount; k++) {
        newLeft.push(left[addStart + k]);
        newRight.push(right[addStart + k]);
      }
      continue;
    }

    newLeft.push(left[i]);
    newRight.push(right[i]);
    i++;
  }

  return { left: newLeft, right: newRight };
}
