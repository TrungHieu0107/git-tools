export interface Commit {
  hash: string;
  parents: string[];
  refs: string[];
  subject: string;
  author: string;
  date: string;
}

export interface GraphNode extends Commit {
  x: number; // Lane index (0, 1, 2...)
  y: number; // Row index (0, 1, 2...)
  color: string;
}

export interface GraphEdge {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  color: string;
  type: "straight" | "curve";
}

const PALETTE = [
  "#0ea5e9", // Sky Blue
  "#22c55e", // Green
  "#eab308", // Yellow
  "#f97316", // Orange
  "#ef4444", // Red
  "#a855f7", // Purple
  "#ec4899", // Pink
  "#14b8a6", // Teal
  "#6366f1", // Indigo
];

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
      const subject = parts.slice(5).join("|"); // Join rest in case of pipe in subject

      let refs: string[] = [];
      if (refsStr && refsStr.trim()) {
        refs = refsStr.trim().replace(/^\(/, "").replace(/\)$/, "").split(",").map(r => r.trim());
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
  
  // Lane state management
  // Maps a lane index to the commit hash it is currently "expecting" (reserved for)
  const lanes: (string | null)[] = []; 
  // Track colors for consistency
  const laneColors: string[] = []; 

  function getColor(laneIdx: number) {
      if (!laneColors[laneIdx]) {
          laneColors[laneIdx] = PALETTE[laneIdx % PALETTE.length];
      }
      return laneColors[laneIdx];
  }

  // Iterate top-down to place nodes
  for (let i = 0; i < commits.length; i++) {
    const commit = commits[i];
    let laneIdx = -1;

    // 1. Check if any lane is reserved for this commit (from a previous child)
    // We prioritize the lowest index lane to keep graph left-aligned
    const reservedLaneIdx = lanes.indexOf(commit.hash);

    if (reservedLaneIdx !== -1) {
        laneIdx = reservedLaneIdx;
        // Clear this reservation, we are placing the commit now
        lanes[laneIdx] = null;
    } else {
        // No reservation (new branch, or disjoint history start)
        // Find first empty lane
        let freeIdx = lanes.indexOf(null);
        if (freeIdx === -1) {
            freeIdx = lanes.length;
            lanes.push(null);
        }
        laneIdx = freeIdx;
    }

    // Assign Node placement
    const color = getColor(laneIdx);
    const node: GraphNode = {
        ...commit,
        x: laneIdx,
        y: i,
        color
    };
    nodes.push(node);

    // 2. Prepare reservations for parents
    const parents = commit.parents;
    
    // First parent typically continues the branch (straight line down)
    // IMPORTANT: Only reserve if the parent isn't ALREADY reserved by someone else (merge scenario)
    // If parent is already reserved, we have to curve to it later (merge).
    // If we are the primary child (first), we try to keep the lane.
    
    if (parents.length > 0) {
        const firstParent = parents[0];
        
        // If current lane is holding a reservation for something else (e.g. we just cleared it above, but logic?)
        // Actually, we just cleared `lanes[laneIdx]` above.
        
        // Check if firstParent is already reserved elsewhere
        const existingReservation = lanes.indexOf(firstParent);
        
        if (existingReservation !== -1) {
            // Merge logic: Parent is already continuing another lane (we are merging INTO it)
            // We cease this lane. `lanes[laneIdx]` stays null (or free).
            // We will draw a curve to that other lane later.
        } else {
            // Continue this lane
            lanes[laneIdx] = firstParent;
        }

        // Secondary parents (Merge bases)
        for (let p = 1; p < parents.length; p++) {
             const otherParent = parents[p];
             const otherIdx = lanes.indexOf(otherParent);
             
             if (otherIdx === -1) {
                 // Reserve a NEW lane for this other parent history
                 let freeIdx = lanes.indexOf(null);
                 if (freeIdx === -1) {
                     freeIdx = lanes.length;
                     lanes.push(null);
                 }
                 lanes[freeIdx] = otherParent;
                 // Assign color if new
                 if(!laneColors[freeIdx]) laneColors[freeIdx] = PALETTE[freeIdx % PALETTE.length];
             }
             // If already reserved, we just connect to it later
        }
    }
  }

  // Pass 2: Generate Edges based on placed nodes
  // Optimization: We can do this in Pass 1 if we had a "Node Map", but separating is cleaner for logic
  // Since we only have the Array, for every node we need to find its parents' positions.
  // With N=100 or 500, simple lookup is instant.
  
  // Note: graph might not contain all parents (if list is limited/paged)
  const nodeMap = new Map<string, GraphNode>();
  nodes.forEach(n => nodeMap.set(n.hash, n));

  for (const node of nodes) {
      for (let p=0; p < node.parents.length; p++) {
          const parentHash = node.parents[p];
          const parentNode = nodeMap.get(parentHash);

          if (parentNode) {
              edges.push({
                  x1: node.x,
                  y1: node.y,
                  x2: parentNode.x,
                  y2: parentNode.y,
                  color: p === 0 ? node.color : parentNode.color, // Primary edge takes node color, merge edges take source branch color usually
                  type: (node.x === parentNode.x) ? "straight" : "curve"
              });
          } else {
              // Parent off-screen. Draw generic "down" edge.
              // If it's first parent, assume straight down.
              if (p === 0) {
                  edges.push({
                      x1: node.x, y1: node.y,
                      x2: node.x, y2: node.y + 1, // Just fade out down
                      color: node.color,
                      type: "straight"
                  });
              }
              // Ignore secondary parents off-screen to reduce clutter
          }
      }
  }

  return { nodes, edges };
}
