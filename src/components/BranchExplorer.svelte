<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { GitService } from "../lib/GitService";
  import { buildBranchTree, filterBranchTree, getAllFolderPaths, type BranchNode } from "../lib/branch-utils";
  import { confirm } from "../lib/confirmation.svelte";
  import { openCreateBranchDialog } from "../lib/create-branch-dialog.svelte";
  import BranchContextMenu from "./common/BranchContextMenu.svelte";
  import type { BranchContextMenuState } from "./common/branch-context-menu-types";

  let {
      repoPath = undefined,
      isActive = false,
      onNavigateToCommitPanel
  }: {
      repoPath?: string;
      isActive?: boolean;
      onNavigateToCommitPanel?: () => void | Promise<void>;
  } = $props();

  let branches = $state<string[]>([]);
  let currentBranch = $state("");
  let tree = $state<BranchNode[]>([]);
  let loading = $state(false);
  
  // Search State
  let searchQuery = $state("");
  let visibleTree = $derived(searchQuery ? filterBranchTree(tree, searchQuery) : tree);

  // Expanded state handling
  let expandedPaths = $state(new Set<string>());
  // Store user's manual expansion state before filtering
  let savedExpandedPaths = $state<Set<string> | null>(null);

  // Watch for search query changes to handle expansion logic
  $effect(() => {
      const query = searchQuery; // access dependency
      
      untrack(() => {
          if (query) {
              // Filtering is active
              if (!savedExpandedPaths) {
                  // User just started typing, save current state
                  savedExpandedPaths = new Set(expandedPaths);
              }
              // Auto-expand all matching nodes in the filtered tree
              const allPaths = getAllFolderPaths(visibleTree);
              expandedPaths = new Set(allPaths);
          } else {
              // Filtering cleared
              if (savedExpandedPaths) {
                  // Restore previous state
                  expandedPaths = savedExpandedPaths;
                  savedExpandedPaths = null;
              }
          }
      });
  });

  // Action State
  let isCheckoutLoading = $state(false);
  let contextMenu = $state<BranchContextMenuState | null>(null);

  // Close context menu on global click
  function onGlobalClick() {
      contextMenu = null;
  }


  // Load data
  async function loadBranches() {
    if (!repoPath) {
        branches = [];
        tree = [];
        return;
    }
    try {
      loading = true;
      const [allBranches, current] = await Promise.all([
        GitService.getBranches(true, repoPath), // Fetch all branches (local + remote)
        GitService.getCurrentBranch(repoPath)
      ]);
      branches = allBranches;
      currentBranch = current;
      tree = buildBranchTree(branches);
      
      // Auto-expand to current branch
      expandPathToBranch(tree, current);
      
      // Initial state: expand Local root
      if (tree[0]?.children?.length && !expandedPaths.has("Local")) {
          toggleExpand("Local");
      }
    } catch (e) {
      console.error("Failed to load branches", e);
      branches = [];
      tree = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
      loadBranches();
  });

  function expandPathToBranch(nodes: BranchNode[], target: string, parentPath = ""): boolean {
      for (const node of nodes) {
          const currentPath = parentPath ? `${parentPath}/${node.name}` : node.name;
          
          if (node.isLeaf) {
              if (node.fullPath === target) {
                  return true;
              }
          } else if (node.children) {
              if (expandPathToBranch(node.children, target, currentPath)) {
                  const newSet = new Set(expandedPaths);
                  newSet.add(currentPath);
                  expandedPaths = newSet;
                  return true;
              }
          }
      }
      return false;
  }

  function toggleExpand(path: string) {
      const newSet = new Set(expandedPaths);
      if (newSet.has(path)) {
          newSet.delete(path);
      } else {
          newSet.add(path);
      }
      expandedPaths = newSet;
  }
  
  function expandAll() {
      const allPaths = getAllFolderPaths(visibleTree);
      expandedPaths = new Set(allPaths);
  }
  
  function collapseAll() {
      expandedPaths = new Set();
  }

  onMount(() => {
    loadBranches();
    window.addEventListener('repo-activated', loadBranches);
    return () => window.removeEventListener('repo-activated', loadBranches);
  });

  function openCreateBranch() {
    const localBranches = branches.filter(b => !b.startsWith('remotes/'));
    openCreateBranchDialog({
      branches: localBranches,
      currentBranch: currentBranch || 'main',
      repoPath: repoPath || ''
    });
  }

  $effect(() => {
    if (!isActive) return;
    const handleOpenCreate = () => openCreateBranch();
    const handleBranchCreated = () => loadBranches();
    window.addEventListener('open-create-branch', handleOpenCreate);
    window.addEventListener('branch-created', handleBranchCreated);
    return () => {
      window.removeEventListener('open-create-branch', handleOpenCreate);
      window.removeEventListener('branch-created', handleBranchCreated);
    };
  });

  function handleContextMenu(e: MouseEvent, node: BranchNode) {
      e.preventDefault();
      contextMenu = {
          x: e.clientX,
          y: e.clientY,
          payload: node,
          disableMerge: node.fullPath === currentBranch
      };
  }

  function toErrorText(error: unknown): string {
      if (error instanceof Error) {
          return error.message;
      }
      return String(error ?? "");
  }

  function isMergeOrRebaseInProgressMessage(message: string): boolean {
      const normalized = message.toLowerCase();
      return (
          normalized.includes("merge_head exists") ||
          normalized.includes("not concluded your merge") ||
          normalized.includes("you have unmerged paths") ||
          normalized.includes("conflict") ||
          normalized.includes("rebase") ||
          normalized.includes("cherry-pick")
      );
  }

  async function navigateToCommitWhenMergeBlocked(message: string): Promise<boolean> {
      const hasConflicts = await GitService.checkConflictState(repoPath).catch(() => false);
      if (hasConflicts || isMergeOrRebaseInProgressMessage(message)) {
          await onNavigateToCommitPanel?.();
          return true;
      }
      return false;
  }

  async function handleMerge(node: BranchNode) {
      // Block if same branch
      if (node.fullPath === currentBranch) {
          // Should be disabled in UI, but safe check
          return;
      }

      // Confirm
      const confirmed = await confirm({
           title: "Merge Branch",
           message: `Merge branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${node.name}</span> into <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${currentBranch}</span>?`,
           isHtmlMessage: true,
           confirmLabel: "Merge"
      });

      if (!confirmed) return;

      isCheckoutLoading = true;
      try {
          // Validate clean state if possible? 
          // Implementation plan said "ensure current branch is clean".
          // We can assume user knows what they are doing OR do a check.
          // Let's rely on git merge command failing if dirty and conflicting.
          
          const result = await GitService.merge(node.fullPath!, repoPath);
          if (result.success) {
              await loadBranches(); // Refresh in case merge refs changed? (unlikely to change branch list, but good for sync)
              // Also maybe notify success?
              // Since we don't have toast, we do nothing on success except maybe log.
              console.log("Merge successful");
              return;
          }

          const mergedErrorText = result.stderr || result.stdout || "";
          await navigateToCommitWhenMergeBlocked(mergedErrorText);
      } catch (e: any) {
          console.error("Merge failed", e);
          const errorText = toErrorText(e);
          const navigated = await navigateToCommitWhenMergeBlocked(errorText);
          if (navigated) return;
          await confirm({
              title: "Merge Failed",
              message: errorText,
              confirmLabel: "OK",
              cancelLabel: "Close"
          });
      } finally {
          isCheckoutLoading = false;
      }
  }

  async function handleContextCheckout(payload: unknown) {
      const node = payload as BranchNode;
      await handleBranchClick(node);
  }

  async function handleContextMerge(payload: unknown) {
      const node = payload as BranchNode;
      await handleMerge(node);
  }

  async function handleBranchClick(node: BranchNode) {
      if (node.fullPath === currentBranch || isCheckoutLoading) return; // Already on it

      let message = "";
      let isSwitch = true;

      // Determine message and action type
      if (node.type === 'remote') {
          if (branches.includes(node.name)) {
             // Local exists
             message = `A local branch named <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${node.name}</span> already exists.<br/><br/>Switch to existing local branch?`;
             isSwitch = true;
          } else {
             // Create new
             message = `Checkout remote branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${node.name}</span>?<br/><span class="text-xs text-gray-500">This will create a new local tracking branch.</span>`;
             isSwitch = false; 
          }
      } else {
          // Local switch
          message = `Switch to branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${node.name}</span>?`;
          isSwitch = true;
      }

      const confirmed = await confirm({
          title: "Confirm Checkout",
          message,
          isHtmlMessage: true,
          confirmLabel: "Checkout"
      });

      if (!confirmed) return;

      // Proceed
      isCheckoutLoading = true;
      try {
          if (!isSwitch && node.type === 'remote') {
             // Create new logic
             await GitService.checkoutNew(node.name, node.fullPath!, repoPath);
          } else {
             // Switch logic (local or remote-that-has-local)
             const target = (node.type === 'remote' && branches.includes(node.name)) ? node.name : node.fullPath!;
             await GitService.switchBranch(target, repoPath);
          }
          await loadBranches(); 
      } catch (e: any) {
          console.error("Checkout failed", e);
          // Optional: Show error via another alert or toast?
          // Since we don't have global alert yet, log it.
          // Or reuse confirm for error? "Error: ..."
          await confirm({
              title: "Checkout Failed",
              message: e.toString(),
              confirmLabel: "OK",
              cancelLabel: "Close"
          });
      } finally {
          isCheckoutLoading = false;
      }
  }

  // Icons
  const Icons = {
     Branch: `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/></svg>`,
     Folder: `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>`,
     Remote: `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect><rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>`,
     ChevronRight: `<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>`,
     ExpandAll: `<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline><polyline points="6 3 12 9 18 3"></polyline></svg>`, // Custom double down
     CollapseAll: `<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="18 15 12 9 6 15"></polyline><polyline points="18 9 12 3 6 9"></polyline></svg>` // Custom double up
  };
</script>

<!-- Tree Node Snippet -->
{#snippet treeNode(nodes: BranchNode[], parentPath: string = "")}
  {#each nodes as node}
    {@const currentPath = parentPath ? `${parentPath}/${node.name}` : node.name}
    
    {#if node.isLeaf}
       <!-- Leaf Branch -->
       <div 
          class="flex items-center gap-2 py-1 px-2 cursor-pointer hover:bg-[#21262d] rounded text-xs truncate group
                 {node.fullPath === currentBranch ? 'bg-[#1f6feb]/20 text-[#58a6ff]' : 'text-[#c9d1d9]'}"
          ondblclick={() => handleBranchClick(node)}
          oncontextmenu={(e) => handleContextMenu(e, node)}
          title={node.fullPath}
          role="button"
          tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && handleBranchClick(node)}
       >
          <div class="opacity-70">{@html Icons.Branch}</div>
          <span class="truncate">{node.name}</span>
          {#if node.fullPath === currentBranch}
             <div class="ml-auto w-1.5 h-1.5 rounded-full bg-[#58a6ff]"></div>
          {/if}
       </div>
    {:else}
       <!-- Folder -->
       {@const isOpen = expandedPaths.has(currentPath)}
       <div>
           <div 
              class="flex items-center gap-1 py-1 px-2 cursor-pointer hover:text-white text-[#8b949e] text-xs select-none"
              onclick={() => toggleExpand(currentPath)}
              role="button"
              tabindex="0"
              onkeydown={(e) => e.key === 'Enter' && toggleExpand(currentPath)}
              title={isOpen ? "Collapse" : "Expand"}
           >
               <div class="w-4 h-4 flex items-center justify-center transition-transform duration-100 {isOpen ? 'rotate-90' : ''}">
                   {@html Icons.ChevronRight}
               </div>
               <div class="text-amber-500/80">
                   {@html node.name === 'Remote' ? Icons.Remote : Icons.Folder}
               
</div>
               <span class="font-medium truncate">{node.name}</span>
           </div>
           
           {#if isOpen}
               <div class="pl-4 border-l border-[#30363d] ml-2">
                   {@render treeNode(node.children!, currentPath)}
               </div>
           {/if}
       </div>
    {/if}
  {/each}
{/snippet}

<div class="flex flex-col h-full bg-[#0d1117] relative">
    <!-- Header -->
    <div class="flex flex-col border-b border-[#30363d]">
        <div class="flex items-center justify-between px-3 py-2">
            <span class="text-xs font-bold text-[#8b949e] uppercase tracking-wider">Branches</span>
            
            <div class="flex items-center gap-1">
                <button onclick={expandAll} class="text-[#8b949e] hover:text-white p-1 rounded hover:bg-[#30363d]" title="Expand All">
                   {@html Icons.ExpandAll}
                </button>
                <button onclick={collapseAll} class="text-[#8b949e] hover:text-white p-1 rounded hover:bg-[#30363d]" title="Collapse All">
                   {@html Icons.CollapseAll}
                </button>
                <div class="w-px h-3 bg-[#30363d] mx-1"></div>
                <button onclick={openCreateBranch} class="text-[#8b949e] hover:text-white p-1 rounded hover:bg-[#30363d]" title="Create Branch (Ctrl+B)">
                   <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                </button>
                <button onclick={loadBranches} class="text-[#8b949e] hover:text-white p-1 rounded hover:bg-[#30363d]" title="Refresh">
                   <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.3"/></svg>
                </button>
            </div>
        </div>
        
        <!-- Search Input -->
        <div class="px-2 pb-2">
            <input 
                type="text" 
                bind:value={searchQuery}
                placeholder="Filter branches..." 
                class="w-full bg-[#0d1117] border border-[#30363d] rounded px-2 py-1 text-xs text-[#c9d1d9] placeholder-[#484f58] focus:border-[#58a6ff] focus:outline-none transition-colors"
                spellcheck="false"
            />
        </div>
    </div>
    
    <!-- Content -->
    <div class="flex-1 overflow-y-auto custom-scrollbar p-1">
        {#if loading && branches.length === 0}
            <div class="p-4 text-center text-xs text-[#484f58]">Loading...</div>
        {:else if visibleTree.length === 0}
             <div class="p-4 text-center text-xs text-[#484f58]">
                 {#if searchQuery}
                     No branches match "{searchQuery}"
                 {:else}
                     No branches found
                 {/if}
             </div>
        {:else}
            {@render treeNode(visibleTree)}
        {/if}
    </div>


</div>

<BranchContextMenu
  menu={contextMenu}
  onClose={onGlobalClick}
  onCheckout={handleContextCheckout}
  onMerge={handleContextMerge}
/>
