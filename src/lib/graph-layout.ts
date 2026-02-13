import { getColorForColumn } from "./graph-colors";

export interface Commit {
  hash: string;
  parents: string[];
  refs: string[];
  subject: string;
  author: string;
  date: string;
  isStash: boolean;
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
  x: number;
  y: number;
  color: string;
}

export interface LanePath {
  column: number;
  rowStart: number;
  rowEnd: number;
  color: string;
}

export interface ConnectionPath {
  fromColumn: number;
  fromRow: number;
  toColumn: number;
  toRow: number;
  color: string;
  parentIndex: number;
  lineStyle: "solid" | "dashed";
}

function isStashRef(ref: string): boolean {
  const normalized = ref.trim();
  return normalized === "refs/stash" || /^stash@\{\d+\}$/.test(normalized);
}

function normalizeStashSubject(subject: string): string {
  const normalized = subject.trim();
  if (!normalized) return "stash";

  const withBranchPrefix = normalized.match(/^(?:WIP on|On)\s+[^:]+:\s*(.*)$/i);
  if (withBranchPrefix) {
    const message = withBranchPrefix[1].trim();
    return message || "stash";
  }

  if (/^(?:WIP on|On)\s+/i.test(normalized)) {
    return "stash";
  }

  return normalized;
}

export function parseGitLog(output: string): Commit[] {
  if (!output.trim()) return [];

  const parsed = output
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

      const isStash = refs.some(isStashRef);

      return {
        hash,
        parents: parentsStr ? parentsStr.split(" ") : [],
        refs,
        subject: isStash ? normalizeStashSubject(subject) : subject,
        author,
        date,
        isStash,
      };
    });

  const stashHelperHashes = new Set<string>();
  for (const commit of parsed) {
    if (!commit.isStash) continue;
    for (let parentIndex = 1; parentIndex < commit.parents.length; parentIndex += 1) {
      const parentHash = commit.parents[parentIndex];
      if (parentHash) stashHelperHashes.add(parentHash);
    }
  }

  if (stashHelperHashes.size === 0) {
    return parsed;
  }

  return parsed
    .filter((commit) => !stashHelperHashes.has(commit.hash))
    .map((commit) => {
      if (!commit.isStash) return commit;
      return {
        ...commit,
        parents: commit.parents.filter(
          (parentHash, parentIndex) => parentIndex === 0 || !stashHelperHashes.has(parentHash),
        ),
      };
    });
}

function assignColumns(commits: Commit[]): GraphNode[] {
  const nodes: GraphNode[] = [];
  const activeBranches = new Map<string, number>();
  const freeColumns: number[] = [];
  let nextColumn = 0;

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
    if (column === 0) return;
    if (!freeColumns.includes(column)) {
      freeColumns.push(column);
    }
  }

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
      color: getColorForColumn(column),
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
      activeBranches.set(firstParent, 0);
    } else if (firstParentColumn !== column) {
      releaseColumn(column);
    }

    for (const parentHash of secondaryParents) {
      if (activeBranches.has(parentHash)) continue;
      const parentColumn = takeColumn();
      activeBranches.set(parentHash, parentColumn);
    }
  }

  return nodes;
}

function buildLanesAndConnections(
  nodes: GraphNode[],
  _commits: Commit[],
): { lanes: LanePath[]; connections: ConnectionPath[] } {
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
    addLaneSegment(node.x, node.y, node.y);

    for (let parentIndex = 0; parentIndex < node.parents.length; parentIndex += 1) {
      const parentHash = node.parents[parentIndex];
      const parentNode = nodeByHash.get(parentHash);
      const isStashMainParent = node.isStash && parentIndex === 0;

      if (!parentNode) {
        if (parentIndex === 0) {
          addLaneSegment(node.x, node.y, node.y + 1);
        }
        continue;
      }

      if (parentNode.x === node.x) {
        if (isStashMainParent) {
          connections.push({
            fromColumn: node.x,
            fromRow: node.y,
            toColumn: parentNode.x,
            toRow: parentNode.y,
            color: node.color,
            parentIndex,
            lineStyle: "dashed",
          });
          addLaneSegment(parentNode.x, parentNode.y, parentNode.y);
          continue;
        }

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
        lineStyle: isStashMainParent ? "dashed" : "solid",
      });

      addLaneSegment(parentNode.x, parentNode.y, parentNode.y);
    }
  }

  const lanes: LanePath[] = [];
  for (const [column, segments] of laneSegments.entries()) {
    for (const segment of segments) {
      lanes.push({
        column,
        rowStart: segment.start,
        rowEnd: segment.end,
        color: getColorForColumn(column),
      });
    }
  }

  return { lanes, connections };
}

function mergeOverlappingSegments(lanes: LanePath[]): LanePath[] {
  const lanesByColumn = new Map<number, LanePath[]>();
  for (const lane of lanes) {
    const grouped = lanesByColumn.get(lane.column) ?? [];
    grouped.push(lane);
    lanesByColumn.set(lane.column, grouped);
  }

  const merged: LanePath[] = [];
  for (const [column, segments] of lanesByColumn.entries()) {
    if (segments.length === 0) continue;

    const sorted = [...segments].sort((a, b) =>
      a.rowStart === b.rowStart ? a.rowEnd - b.rowEnd : a.rowStart - b.rowStart,
    );

    let current = {
      rowStart: sorted[0].rowStart,
      rowEnd: sorted[0].rowEnd,
    };

    for (let i = 1; i < sorted.length; i += 1) {
      const next = sorted[i];
      if (next.rowStart <= current.rowEnd + 1) {
        current.rowEnd = Math.max(current.rowEnd, next.rowEnd);
      } else {
        merged.push({
          column,
          rowStart: current.rowStart,
          rowEnd: current.rowEnd,
          color: getColorForColumn(column),
        });
        current = { rowStart: next.rowStart, rowEnd: next.rowEnd };
      }
    }

    merged.push({
      column,
      rowStart: current.rowStart,
      rowEnd: current.rowEnd,
      color: getColorForColumn(column),
    });
  }

  merged.sort((a, b) => (a.column === b.column ? a.rowStart - b.rowStart : a.column - b.column));
  return merged;
}

export function calculateGraphLayout(commits: Commit[]): {
  nodes: GraphNode[];
  lanes: LanePath[];
  connections: ConnectionPath[];
} {
  if (commits.length === 0) {
    return { nodes: [], lanes: [], connections: [] };
  }

  const nodes = assignColumns(commits);
  const { lanes: rawLanes, connections } = buildLanesAndConnections(nodes, commits);
  const lanes = mergeOverlappingSegments(rawLanes);
  connections.sort((a, b) => (a.fromRow === b.fromRow ? a.fromColumn - b.fromColumn : a.fromRow - b.fromRow));

  return { nodes, lanes, connections };
}
