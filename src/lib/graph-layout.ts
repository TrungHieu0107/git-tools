export interface Commit {
  hash: string;
  parents: string[];
  refs: string[];
  subject: string;
  author: string;
  date: string;
}

function hashString(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash;
}

function toInitials(authorName: string): string {
  const normalized = authorName.trim();
  if (!normalized) return "?";
  const parts = normalized.split(/\s+/).filter(Boolean);
  if (parts.length === 1) {
    return parts[0].slice(0, 2).toUpperCase();
  }
  return `${parts[0][0]}${parts[parts.length - 1][0]}`.toUpperCase();
}

function escapeXml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

/**
 * Build a deterministic inline avatar URI from author name.
 * Same author always gets the same avatar without network requests.
 */
export function getAvatarUrl(authorName: string): string {
  const seed = authorName.trim() || "Unknown";
  const initials = escapeXml(toInitials(seed));
  const hash = hashString(seed.toLowerCase());
  const hue = hash % 360;
  const hue2 = (hue + 38) % 360;

  const svg = [
    '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">',
    "<defs>",
    '<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">',
    `<stop offset="0%" stop-color="hsl(${hue} 72% 54%)"/>`,
    `<stop offset="100%" stop-color="hsl(${hue2} 70% 40%)"/>`,
    "</linearGradient>",
    "</defs>",
    '<rect width="64" height="64" rx="32" fill="url(#bg)"/>',
    '<circle cx="20" cy="20" r="18" fill="rgba(255,255,255,0.12)"/>',
    '<text x="32" y="34" fill="#f8fafc" font-size="23" font-family="Segoe UI, Arial, sans-serif" font-weight="700" text-anchor="middle" dominant-baseline="middle">',
    initials,
    "</text>",
    "</svg>",
  ].join("");

  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

export interface GraphNode extends Commit {
  x: number; // Stable column index (0, 1, 2...)
  y: number; // Row index (0, 1, 2...)
  color: string;
}

/** A continuous vertical lane segment for a single column span. */
export interface LanePath {
  column: number; // Column index
  rowStart: number; // First row of this lane segment
  rowEnd: number; // Last row of this lane segment
  color: string; // Lane color
}

/** A curved connection between two different columns (merge/fork). */
export interface ConnectionPath {
  fromColumn: number; // Source column
  fromRow: number; // Source row
  toColumn: number; // Target column
  toRow: number; // Target row
  color: string; // Connection color
  parentIndex: number; // 0 = first parent, >0 = merge parent
}

const PALETTE = [
  "#4a90d9", // Blue
  "#37a349", // Green
  "#d4b44e", // Amber
  "#e06860", // Red
  "#a57ad9", // Purple
  "#6aabdb", // Cyan
  "#db904a", // Orange
  "#b893db", // Lavender
  "#49b856", // Mint
];

export function parseGitLog(output: string): Commit[] {
  if (!output.trim()) return [];

  // Format: %H|%P|%d|%an|%cI|%s
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

export function calculateGraphLayout(commits: Commit[]): {
  nodes: GraphNode[];
  lanes: LanePath[];
  connections: ConnectionPath[];
} {
  if (commits.length === 0) {
    return { nodes: [], lanes: [], connections: [] };
  }

  const nodes: GraphNode[] = [];

  // hash -> lane column for commits that have not been rendered yet.
  const activeBranches = new Map<string, number>();
  const freeColumns: number[] = [];
  const laneColors: string[] = [];
  let nextColumn = 0;

  function getColor(columnIndex: number): string {
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

  function releaseColumn(column: number): void {
    // Keep the left-most lane stable for the primary history line.
    if (column === 0) return;
    if (!freeColumns.includes(column)) {
      freeColumns.push(column);
    }
  }

  // Pass 1: assign each commit to a lane column.
  for (let i = 0; i < commits.length; i += 1) {
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
      // Lane continuity already set.
    } else if (column === 0) {
      // Keep the primary lane anchored on column 0.
      activeBranches.set(firstParent, 0);
    } else if (firstParentColumn !== column) {
      // Current lane ended by merge into an existing lane.
      releaseColumn(column);
    }

    for (const parentHash of secondaryParents) {
      if (activeBranches.has(parentHash)) continue;
      const parentColumn = takeColumn();
      activeBranches.set(parentHash, parentColumn);
      getColor(parentColumn);
    }
  }

  // Pass 2: build vertical lane segments and cross-lane connections.
  type LaneSegment = { start: number; end: number };
  const laneSegments = new Map<number, LaneSegment[]>();
  const connections: ConnectionPath[] = [];
  const nodeByHash = new Map<string, GraphNode>();

  for (const node of nodes) {
    nodeByHash.set(node.hash, node);
  }

  function addLaneSegment(column: number, rowA: number, rowB: number): void {
    const start = Math.min(rowA, rowB);
    const end = Math.max(rowA, rowB);
    const segments = laneSegments.get(column) ?? [];
    segments.push({ start, end });
    laneSegments.set(column, segments);
  }

  for (const node of nodes) {
    // Keep at least a point in the node lane.
    addLaneSegment(node.x, node.y, node.y);

    for (let parentIndex = 0; parentIndex < node.parents.length; parentIndex += 1) {
      const parentHash = node.parents[parentIndex];
      const parentNode = nodeByHash.get(parentHash);

      if (!parentNode) {
        // Parent outside loaded range; continue the first-parent lane visually.
        if (parentIndex === 0) {
          addLaneSegment(node.x, node.y, node.y + 1);
        }
        continue;
      }

      if (parentNode.x === node.x) {
        addLaneSegment(node.x, node.y, parentNode.y);
        continue;
      }

      connections.push({
        fromColumn: node.x,
        fromRow: node.y,
        toColumn: parentNode.x,
        toRow: parentNode.y,
        color: parentIndex === 0 ? node.color : parentNode.color,
        parentIndex,
      });

      // Anchor destination lane at parent row to avoid visual gaps.
      addLaneSegment(parentNode.x, parentNode.y, parentNode.y);
    }
  }

  // Merge overlapping or touching vertical segments in each column.
  const lanes: LanePath[] = [];
  for (const [column, segments] of laneSegments.entries()) {
    if (segments.length === 0) continue;

    const sorted = [...segments].sort((a, b) => (a.start === b.start ? a.end - b.end : a.start - b.start));
    let current = { ...sorted[0] };

    for (let i = 1; i < sorted.length; i += 1) {
      const next = sorted[i];
      if (next.start <= current.end + 1) {
        current.end = Math.max(current.end, next.end);
      } else {
        lanes.push({
          column,
          rowStart: current.start,
          rowEnd: current.end,
          color: getColor(column),
        });
        current = { ...next };
      }
    }

    lanes.push({
      column,
      rowStart: current.start,
      rowEnd: current.end,
      color: getColor(column),
    });
  }

  lanes.sort((a, b) => (a.column === b.column ? a.rowStart - b.rowStart : a.column - b.column));
  connections.sort((a, b) => (a.fromRow === b.fromRow ? a.fromColumn - b.fromColumn : a.fromRow - b.fromRow));

  return { nodes, lanes, connections };
}
