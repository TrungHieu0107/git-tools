<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { GitService } from "../lib/GitService";
  import { buildBranchTree, filterBranchTree, getAllFolderPaths, type BranchNode } from "../lib/branch-utils";

  let { repoPath = undefined }: { repoPath?: string } = $props();

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

  // Modal State
  let showModal = $state(false);
  let selectedBranchNode = $state<BranchNode | null>(null);
  let isCheckoutLoading = $state(false);
  let checkoutError = $state<string | null>(null);

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

  function handleBranchClick(node: BranchNode) {
      if (node.fullPath === currentBranch) return; // Already on it
      selectedBranchNode = node;
      checkoutError = null;
      showModal = true;
  }

  function closeModal() {
      showModal = false;
      selectedBranchNode = null;
      checkoutError = null;
  }

  async function confirmCheckout() {
      if (!selectedBranchNode || !selectedBranchNode.fullPath) return;

      isCheckoutLoading = true;
      checkoutError = null;

      try {
          if (selectedBranchNode.type === 'remote') {
              let branchName = selectedBranchNode.name;
              
              // Check if a local branch with this name already exists
              // branches array contains plain names called "local" branches, and "remotes/..."
              if (branches.includes(branchName)) {
                  // Local branch exists, switch to it
                  await GitService.switchBranch(branchName, repoPath);
              } else {
                  // No local branch, create new tracking branch
                  const startPoint = selectedBranchNode.fullPath;
                  await GitService.checkoutNew(branchName, startPoint, repoPath);
              }
          } else {
              await GitService.switchBranch(selectedBranchNode.fullPath, repoPath);
          }

          await loadBranches(); 
          closeModal();
      } catch (e: any) {
          console.error("Checkout failed", e);
          checkoutError = e.toString();
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
          onclick={() => handleBranchClick(node)}
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

    <!-- Confirmation Modal -->
    {#if showModal && selectedBranchNode}
        <!-- Backdrop -->
        <div class="absolute inset-0 z-50 flex items-start justify-center pt-10 bg-black/60 backdrop-blur-sm" onclick={closeModal} role="presentation">
            <!-- Modal Content -->
            <div class="bg-[#161b22] border border-[#30363d] rounded-lg shadow-xl w-64 max-w-[90%] overflow-hidden" onclick={(e) => e.stopPropagation()} role="dialog">
                <div class="px-4 py-3 border-b border-[#30363d] bg-[#0d1117]">
                    <h3 class="text-sm font-semibold text-white">Confirm Checkout</h3>
                </div>
                
                <div class="p-4">
                    <p class="text-xs text-[#c9d1d9] mb-3">
                        {#if selectedBranchNode.type === 'remote' && branches.includes(selectedBranchNode.name)}
                            A local branch named <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">{selectedBranchNode.name}</span> already exists.
                            <br/><br/>
                            Switch to existing local branch?
                        {:else}
                            Are you sure you want to checkout <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">{selectedBranchNode.name}</span>?
                        {/if}
                    </p>
                    
                    {#if checkoutError}
                        <div class="p-2 mb-3 text-xs text-red-400 bg-red-900/20 border border-red-900/50 rounded overflow-auto max-h-24">
                            {checkoutError}
                        </div>
                    {/if}

                    <div class="flex justify-end gap-2 mt-2">
                        <button 
                            class="px-3 py-1.5 text-xs text-[#8b949e] hover:text-white rounded border border-transparent hover:border-[#30363d] transition-colors"
                            onclick={closeModal}
                            disabled={isCheckoutLoading}
                        >
                            Cancel
                        </button>
                        <button 
                            class="px-3 py-1.5 text-xs text-white bg-[#238636] hover:bg-[#2ea043] rounded font-medium shadow-sm transition-colors flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                            onclick={confirmCheckout}
                            disabled={isCheckoutLoading}
                        >
                            {#if isCheckoutLoading}
                                <svg class="animate-spin h-3 w-3 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                </svg>
                            {/if}
                            Confirm
                        </button>
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>
