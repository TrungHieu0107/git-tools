export interface BranchNode {
  name: string;
  fullPath?: string; // Only for leaf nodes
  children?: BranchNode[];
  isLeaf: boolean;
  // Optional: type to distinguish strictly
  type?: 'folder' | 'local' | 'remote';
}

export function buildBranchTree(branches: string[]): BranchNode[] {
  // Create fixed roots
  const localRoot: BranchNode = {
    name: "Local",
    isLeaf: false,
    children: [],
    type: 'folder'
  };

  const remoteRoot: BranchNode = {
    name: "Remote",
    isLeaf: false,
    children: [],
    type: 'folder'
  };

  branches.sort().forEach(branch => {
    // Determine if remote or local
    let isRemote = false;
    let parts: string[] = [];

    if (branch.startsWith("remotes/")) {
      isRemote = true;
      // Strip "remotes/" prefix for tree structure, keep fullPath for checkout
      // Example: "remotes/origin/main" -> ["origin", "main"]
      parts = branch.substring("remotes/".length).split('/');
    } else {
      isRemote = false;
      parts = branch.split('/');
    }

    let currentLevel = isRemote ? remoteRoot.children! : localRoot.children!;

    parts.forEach((part, index) => {
      const isLast = index === parts.length - 1;
      let existingNode = currentLevel.find(n => n.name === part && n.isLeaf === isLast);

      if (!existingNode) {
        existingNode = {
          name: part,
          isLeaf: isLast,
          children: isLast ? undefined : [],
          fullPath: isLast ? branch : undefined,
          type: isLast ? (isRemote ? 'remote' : 'local') : 'folder'
        };
        // Simple append - could insert sorted if we wanted folders-first
        currentLevel.push(existingNode);
        
        // Re-sort current level: folders first, then alpha
        // This is a bit expensive inside the loop, but fine for < 1000 branches
        currentLevel.sort((a, b) => {
            if (a.isLeaf === b.isLeaf) {
                return a.name.localeCompare(b.name);
            }
            return a.isLeaf ? 1 : -1; // folders first
        });
      }

      if (!isLast) {
        currentLevel = existingNode.children!;
      }
    });
  });

  // Only return roots if they have children? User requirement says "Organize branches into Local and Remote root groups"
  // So we always return them.
  return [localRoot, remoteRoot];
}

/**
 * Recursively filter the tree. 
 * A node is included if:
 * 1. It is a leaf and its name matches the query (case-insensitive).
 * 2. It is a folder and ANY of its children match (recursion).
 */
export function filterBranchTree(nodes: BranchNode[], query: string): BranchNode[] {
    if (!query) return nodes;
    
    const lowerQuery = query.toLowerCase();
    
    // Helper to check if a node matches
    // We only match leaf nodes (branches) by name against the query? 
    // Or do we match folder names too? 
    // Requirement: "Matching branches remain visible". 
    // If I type "feature", and have "feature/foo", it should match.
    // If I match a folder name, should I show all children?
    // Let's implement strict branch matching first: path to matching branch is kept.
    // Enhanced: if a folder matches, maybe we show it? But let's stick to fulfilling the "branch" finding goal.
    
    // Recursive filter
    return nodes.reduce<BranchNode[]>((acc, node) => {
        // If it's a leaf, check name
        if (node.isLeaf) {
            if (node.name.toLowerCase().includes(lowerQuery)) {
                acc.push(node);
            }
        } else {
            // If it's a folder, recurse
            const filteredChildren = filterBranchTree(node.children || [], query);
            
            // If children matched, keep this folder with filtered children
            if (filteredChildren.length > 0) {
                acc.push({
                    ...node,
                    children: filteredChildren
                });
            } else {
                // Should we check if the folder Name itself matches? 
                // If I type "remote", I expect to see the Remote folder?
                // If the folder matches, but has no matching children (branches), should we show it? 
                // If a user types "feature", and they have "feature/login", the recursion covers it.
                // If they have an empty folder "old-stuff", and type "old", maybe they want to see it?
                // But generally "Filtering branches" implies finding actionable items.
                // Let's stick to: keep if children match OR if name matches (and keep children as is? or empty?).
                // Let's stick to strictly keeping paths to matching leaves for now as it's cleaner.
                
                // Correction: If the user searches for "Local", they probably want to see the Local tree.
                // Let's allow folder match to keep the node, but arguably if we just filter children, 
                // we might show empty folder if we're not careful.
                
                // Let's do: Only keep folder if it has matching children.
            }
        }
        return acc;
    }, []);
}

/**
 * Recursively collect all paths for nodes that have children (folders).
 */
export function getAllFolderPaths(nodes: BranchNode[], parentPath = ""): string[] {
    let paths: string[] = [];
    
    for (const node of nodes) {
        const currentPath = parentPath ? `${parentPath}/${node.name}` : node.name;
        
        if (!node.isLeaf) {
            // It's a folder (or root), add it
            paths.push(currentPath);
            
            if (node.children) {
                paths = paths.concat(getAllFolderPaths(node.children, currentPath));
            }
        }
    }
    
    return paths;
}
