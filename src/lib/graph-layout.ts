export interface Commit {
  hash: string;
  parents: string[];
  refs: string[];
  subject: string;
  author: string;
  date: string;
}

export interface GraphNode extends Commit {
  x: number; // Stable column index (0, 1, 2...)
  y: number; // Row index (0, 1, 2...)
  color: string;
}

export interface GraphEdge {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  color: string;
  parentIndex: number; // 0 = first-parent (continuity), >0 = secondary (merge/fork)
}

const PALETTE = [
  "#58a6ff", // Blue
  "#3fb950", // Green
  "#f2cc60", // Amber
  "#ff7b72", // Red
  "#bc8cff", // Purple
  "#79c0ff", // Cyan
  "#ffa657", // Orange
  "#d2a8ff", // Lavender
  "#56d364", // Mint
];

export function buildStraightGraphPath(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  turnGap = 10,
  turnAtStart = false
): string {
  if (x1 === x2) {
    return `M ${x1} ${y1} V ${y2}`;
  }

  const verticalDistance = Math.abs(y2 - y1);
  const clampedTurnGap = Math.max(4, Math.min(turnGap, verticalDistance / 2));

  if (turnAtStart) {
    // Turn near the source (y1): depart horizontally, then run vertically to parent.
    const turnY = y2 >= y1 ? y1 + clampedTurnGap : y1 - clampedTurnGap;
    return `M ${x1} ${y1} V ${turnY} H ${x2} V ${y2}`;
  } else {
    // Turn near the target (y2): run vertically, then turn horizontally into parent.
    const turnY = y2 >= y1 ? y2 - clampedTurnGap : y2 + clampedTurnGap;
    return `M ${x1} ${y1} V ${turnY} H ${x2} V ${y2}`;
  }
}

export function parseGitLog(output: string): Commit[] {
  if (!output.trim()) return [];

  // Format: %h|%p|%d|%an|%ad|%s
  return output
    .split("\n")
    .filter((line) => line.trim())
    .map((line) => {
      const parts = line.split("|");
      const hash = parts[0];
      const parentsStr = parts[1];
      const refsStr = parts[2];
      const author = parts[3];
      const date = parts[4];
      const subject = parts.slice(5).join("|");

      let refs: string[] = [];
      if (refsStr && refsStr.trim()) {
        refs = refsStr
          .trim()
          .replace(/^\(/, "")
          .replace(/\)$/, "")
          .split(",")
          .map((r) => r.trim());
      }

      return {
        hash,
        parents: parentsStr ? parentsStr.split(" ") : [],
        refs,
        subject,
        author,
        date,
      };
    });
}

export function calculateGraphLayout(commits: Commit[]): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];

  // branch tip hash -> fixed column index
  const activeBranches = new Map<string, number>();
  const freeColumns: number[] = [];
  const laneColors: string[] = [];
  let nextColumn = 0;

  function getColor(columnIndex: number) {
    if (!laneColors[columnIndex]) {
      laneColors[columnIndex] = PALETTE[columnIndex % PALETTE.length];
    }
    return laneColors[columnIndex];
  }

  function takeColumn(): number {
    if (freeColumns.length > 0) {
      freeColumns.sort((a, b) => a - b);
      return freeColumns.shift()!;
    }
    const column = nextColumn;
    nextColumn += 1;
    return column;
  }

  function releaseColumn(column: number) {
    // Keep root lane anchored on the far left.
    if (column === 0) return;
    if (!freeColumns.includes(column)) {
      freeColumns.push(column);
    }
  }

  for (let i = 0; i < commits.length; i++) {
    const commit = commits[i];
    let column = activeBranches.get(commit.hash);

    if (column !== undefined) {
      activeBranches.delete(commit.hash);
    } else {
      column = takeColumn();
    }

    nodes.push({
      ...commit,
      x: column,
      y: i,
      color: getColor(column),
    });

    if (commit.parents.length === 0) {
      releaseColumn(column);
      continue;
    }

    const [firstParent, ...secondaryParents] = commit.parents;
    const firstParentColumn = activeBranches.get(firstParent);

    if (firstParentColumn === undefined) {
      activeBranches.set(firstParent, column);
    } else if (firstParentColumn === column) {
      // Already on the right column — no-op.
    } else if (column === 0) {
      // Column 0 always wins contention to keep the root lane stable.
      activeBranches.set(firstParent, 0);
    } else {
      // This branch merged into an existing lane; current lane can be reused.
      releaseColumn(column);
    }

    for (const parentHash of secondaryParents) {
      if (activeBranches.has(parentHash)) continue;
      const secondaryColumn = takeColumn();
      activeBranches.set(parentHash, secondaryColumn);
      getColor(secondaryColumn);
    }
  }

  const nodeMap = new Map<string, GraphNode>();
  nodes.forEach((node) => nodeMap.set(node.hash, node));

  for (const node of nodes) {
    for (let parentIndex = 0; parentIndex < node.parents.length; parentIndex++) {
      const parentHash = node.parents[parentIndex];
      const parentNode = nodeMap.get(parentHash);

      if (parentNode) {
        edges.push({
          x1: node.x,
          y1: node.y,
          x2: parentNode.x,
          y2: parentNode.y,
          color: parentIndex === 0 ? node.color : parentNode.color,
          parentIndex,
        });
      } else if (parentIndex === 0) {
        // Parent is outside loaded range; continue line down to viewport edge.
        edges.push({
          x1: node.x,
          y1: node.y,
          x2: node.x,
          y2: node.y + 1,
          color: node.color,
          parentIndex: 0,
        });
      }
    }
  }

  return { nodes, edges };
}
