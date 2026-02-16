<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { GitService, type RepoEntry, type FileStatus } from '../lib/GitService';
  import { parseGitLog, calculateGraphLayout, type GraphNode, type LanePath, type ConnectionPath } from "../lib/graph-layout";
  import { getAuthRequiredMessage } from "../lib/git-errors";
  
  import Conflicts from './Conflicts.svelte';
  import TerminalPanel from './TerminalPanel.svelte';
  import CommitGraph from './CommitGraph.svelte';
  import FileHistoryPanel from './FileHistoryPanel.svelte';
  import BlameView from './blame/BlameView.svelte';
  import BranchExplorer from './BranchExplorer.svelte';
  import ResizablePanel from './resize/ResizablePanel.svelte';
  import SettingsView from './SettingsView.svelte';

  // Stores
  import { graphReloadRequested } from '../lib/stores/git-events';

  interface Props {
    repoId: string;
    repoPath: string;
    isActive: boolean;
  }
  
  let { repoId, repoPath, isActive }: Props = $props();

  // Local State for this Workspace
  let loading = $state(true);
  
  // View Routing
  let currentView = $state<'repos' | 'conflicts'>('repos'); 
  let activeTab = $state<"terminal" | "graph" | "history" | "blame" | "settings">("graph");
  
  // Graph State
  let graphNodes = $state<GraphNode[]>([]);
  let graphLanes = $state<LanePath[]>([]);
  let graphConnections = $state<ConnectionPath[]>([]);
  const INITIAL_GRAPH_COMMIT_LIMIT = 200;
  const GRAPH_COMMIT_LOAD_STEP = 200;
  let graphCommitLimit = $state(INITIAL_GRAPH_COMMIT_LIMIT);
  let graphHasMoreCommits = $state(true);
  let graphLoading = $state(false);
  let graphLoadingMore = $state(false);
  
  // Repo State
  let hasConflicts = $state(false);
  let pendingPushCount = $state(0);
  let commitGraph = $state<any>(null);
  let selectedFile = $state<FileStatus | null>(null);
  let pendingCommitFocusHash = $state<string | null>(null);

  async function handleConflictDetection() {
    if (!repoPath) {
        hasConflicts = false;
        return;
    }
    try {
        hasConflicts = await GitService.checkConflictState(repoPath);
        if (hasConflicts && currentView === 'repos') {
             currentView = 'conflicts';
        }
    } catch (e) {
        console.error("Failed to check conflict state:", e);
        hasConflicts = false;
    }
  }

  // Load initial data when repoPath changes or component mounts
  $effect(() => {
     if (repoPath) {
         handleConflictDetection();
     }
  });

  async function loadGraph(options: { switchToGraph?: boolean; limit?: number } = {}) {
    if (!repoPath) return;
    const switchToGraph = options.switchToGraph ?? true;
    const requestedLimit = options.limit ?? graphCommitLimit;
    const normalizedLimit = Math.max(1, Math.floor(requestedLimit));
    graphLoading = true;
    try {
      // Refresh pending count
      try {
          pendingPushCount = await GitService.getPendingCommitsCount(repoPath);
      } catch (e) {
          console.error("Failed to get pending count", e);
      }

      const logOutput = await GitService.getCommitGraph(normalizedLimit, repoPath);
      const commits = parseGitLog(logOutput);
      const layout = calculateGraphLayout(commits);
      graphNodes = layout.nodes;
      graphLanes = layout.lanes;
      graphConnections = layout.connections;
      graphCommitLimit = normalizedLimit;
      graphHasMoreCommits = commits.length >= normalizedLimit;
      
      if (switchToGraph !== false) {
          activeTab = "graph";
      }
      await tick();
      await focusPendingCommit();
    } catch (e) {
      console.error("Failed to load graph:", e);
    } finally {
      graphLoading = false;
    }
  }

  async function handleLoadMoreCommits(): Promise<boolean> {
      if (!repoPath || graphLoading || graphLoadingMore || !graphHasMoreCommits) {
          return false;
      }

      graphLoadingMore = true;
      const previousCount = graphNodes.length;
      const nextLimit = graphCommitLimit + GRAPH_COMMIT_LOAD_STEP;

      try {
          await loadGraph({ switchToGraph: false, limit: nextLimit });
      } finally {
          graphLoadingMore = false;
      }

      return graphNodes.length > previousCount;
  }

  function setSelectedFilePath(path: string): void {
      selectedFile = { path, status: "", staged: false };
  }

  async function navigateToCommitPanel(): Promise<void> {
      activeTab = "graph";
      await tick();
      if (commitGraph?.selectWipRow) {
          commitGraph.selectWipRow();
      }
  }

  function handleShowFileHistory(path: string): void {
      setSelectedFilePath(path);
      activeTab = "history";
  }

  function handleShowFileBlame(path: string): void {
      setSelectedFilePath(path);
      activeTab = "blame";
  }

  async function focusPendingCommit(): Promise<void> {
      if (!pendingCommitFocusHash || !commitGraph) return;
      const targetHash = pendingCommitFocusHash;
      const focused = await commitGraph.focusCommit(targetHash);
      if (focused) {
          pendingCommitFocusHash = null;
      }
  }

  async function handleBlameCommitSelect(commitHash: string): Promise<void> {
      pendingCommitFocusHash = commitHash;
      activeTab = "graph";
      if (graphNodes.length === 0 && !graphLoading) {
          await loadGraph({ switchToGraph: false });
          return;
      }
      await focusPendingCommit();
  }

  // Subscribe to global reload events, but check if they apply to us (or just reload all)
  // Ideally events should be scoped, but for now global refresh is okay
  let reloadTrigger = $state(0);
  onMount(() => {
    // Initial Load?
    if (activeTab === 'graph') {
        loadGraph({ switchToGraph: false });
    }
    
    const unsub = graphReloadRequested.subscribe(v => reloadTrigger = v);
    return unsub;
  });

  $effect(() => {
    if (reloadTrigger > 0 && repoPath) {
        // Reload graph if we are looking at it, or just strictly reload data
        // For now, reloading graph data is cheap enough
        if (activeTab === 'graph') {
             loadGraph({ switchToGraph: false });
        }
        handleConflictDetection();
    }
  });

  // Auto-load graph when switching to the Graph tab
  $effect(() => {
    if (activeTab === 'graph' && repoPath && graphNodes.length === 0 && !graphLoading) {
        loadGraph({ switchToGraph: false });
    }
  });

  function navigateToRepos() {
      // asking parent to switch to repo manager?
      // Or just local view state?
      // Note: "Repos" view in original App.svelte cleared activeRepo. 
      // In Multi-tab, "Repos" view might mean "Dashboard" or "Repo List".
      // But here we are IN a workspace. 
      // If user wants to go to Repo Manager, they should probably open a new tab or click a specific "Home" tab.
      // For now, let's keep it as is, but maybe trigger an event?
      // Actually, standard workspace doesn't have "Repos" view. 
      // The Sidebar has a "Repos" button.
      // In multi-repo, that button probably shouldn't close the current repo.
      // It might just open the Repo Manager in a new tab or switch to it.
      
      // For this implementation, let's say "Repos" button functionality is handled by parent or removed from workspace.
      // We'll emit an event.
      dispatch('navigate-repos');
  }

  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();


  // Icons
  const Icons = {
    Play: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 3 19 12 5 21 5 3"/></svg>`,
    Folder: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>`,
    Terminal: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/></svg>`,
    Git: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"/><path d="M9 18c-4.51 2-5-2-7-2"/></svg>`,
    Network: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="5" r="3"/><circle cx="6" cy="19" r="3"/><circle cx="18" cy="19" r="3"/><line x1="12" y1="8" x2="6" y2="19"/><line x1="12" y1="8" x2="18" y2="19"/></svg>`,
    Settings: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`
  };
</script>

<div class="h-full w-full min-w-0 flex overflow-auto font-sans text-sm selection:bg-[#1f6feb] selection:text-white bg-[#0d1117] text-[#c9d1d9]" class:hidden={!isActive}>
  <!-- Left Sidebar (Resizable) -->
  <ResizablePanel initialSize={288} minSize={220} maxSize={450} side="right">
    <aside class="w-full h-full bg-[#161b22] border-r border-[#30363d] flex flex-col shrink-0 z-10">
      
      <!-- Controls (Static) -->
      <div class="p-4 space-y-4 flex-none pt-4"> 
          <!-- Repo Section -->
          <div class="space-y-2">
              <label for="repo-{repoId}" class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider flex items-center gap-2">
                {@html Icons.Folder} Repository
              </label>
              <div class="relative group">
                  <input
                      id="repo-{repoId}"
                      type="text"
                      value={repoPath}
                      readonly
                      class="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-[#c9d1d9] placeholder-[#484f58] focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] outline-none transition-colors text-xs font-mono opacity-70 cursor-default"
                  />
              </div>
          </div>

          <!-- Command Section -->
          <div class="space-y-2">
              <div class="flex gap-2">
                 {#if hasConflicts}
                     <button 
                        class="text-xs px-2 py-1 rounded hover:bg-red-900/50 text-red-400 font-bold border border-red-900 animate-pulse {currentView === 'conflicts' ? 'bg-red-900 text-white' : ''}"
                        onclick={navigateToCommitPanel}
                     >
                        ⚠ Conflicts
                     </button>
                 {/if}
                  <!-- "Repos" button here might need rethink. Maybe "Close Workspace"? -->
                  <!-- For now, removing "Repos" button from Workspace level as navigation acts as "Repos" view via tabs -->
             </div>
          </div>

      </div>

      <!-- Branch Explorer (Flexible) -->
      <div class="flex-1 overflow-hidden border-t border-[#30363d]">
          <BranchExplorer
            {repoPath}
            {isActive}
            onNavigateToCommitPanel={navigateToCommitPanel}
          />
      </div>
      
    </aside>
  </ResizablePanel>

  <!-- Right Panel - Content -->
  <div class="flex-1 flex flex-col min-w-0 bg-[#0d1117] relative">
      <!-- Tabs Header -->
      <div class="min-h-12 border-b border-[#30363d] flex flex-wrap items-center px-2 py-1 bg-[#161b22] gap-1 overflow-x-auto">
        <button 
           onclick={() => activeTab = "graph"}
           class="shrink-0 px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'graph' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           {@html Icons.Network} Graph
        </button>
        <button 
           onclick={() => activeTab = "terminal"}
           class="shrink-0 px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'terminal' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           {@html Icons.Terminal} Terminal
        </button>
        <button
           onclick={() => activeTab = "history"}
           class="shrink-0 px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'history' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg> History
        </button>
        <button 
           onclick={() => activeTab = "blame"}
           class="shrink-0 px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'blame' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 6h13"/><path d="M8 12h13"/><path d="M8 18h13"/><path d="M3 6h.01"/><path d="M3 12h.01"/><path d="M3 18h.01"/></svg> Blame
        </button>
        <div class="flex-1"></div>
        <button 
           onclick={() => activeTab = "settings"}
           class="shrink-0 px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'settings' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
           title="Repository Settings"
        >
           {@html Icons.Settings}
        </button>
      </div>

      <!-- Tab Content area -->
      <div class="flex-1 relative overflow-hidden">
         <!-- Terminal Tab -->
         <div class="absolute inset-0 flex flex-col {activeTab === 'terminal' ? 'z-10 visible' : 'z-0 invisible'}">
            <!-- Terminal panel handles its own active state check if needed, but we control visibility -->
            <!-- We pass isActive to TerminalPanel so it can pause/resume if implemented, 
                 but more importantly repoPath must be static for this workspace -->
             <TerminalPanel {repoPath} isActive={isActive && activeTab === 'terminal'} />
         </div>

         <!-- Graph Tab -->
         <div class="absolute inset-0 bg-[#0d1117] {activeTab === 'graph' ? 'z-10 visible' : 'z-0 invisible'}">
            {#if graphNodes.length === 0 && !graphLoading}
                <div class="absolute inset-0 flex flex-col items-center justify-center text-[#484f58] select-none">
                  <p class="text-sm">No commits available to display.</p>
                </div>
            {:else}
                <CommitGraph
                  bind:this={commitGraph}
                  nodes={graphNodes}
                  lanes={graphLanes}
                  connections={graphConnections}
                  repoPath={repoPath}
                  pendingPushCount={pendingPushCount}
                  onGraphReload={loadGraph}
                  onLoadMoreCommits={handleLoadMoreCommits}
                  hasMoreCommits={graphHasMoreCommits}
                  isLoadingMoreCommits={graphLoadingMore}
                  onShowHistory={handleShowFileHistory}
                  onShowBlame={handleShowFileBlame}
                />
            {/if}
         </div>

          <!-- History Tab -->
          <div class="absolute inset-0 bg-[#0d1117] {activeTab === 'history' ? 'z-10 visible' : 'z-0 invisible'}">
             <FileHistoryPanel 
               repoPath={repoPath} 
               filePath={selectedFile?.path ?? null} 
               onFileSelect={setSelectedFilePath}
             />
          </div>

          <!-- Blame Tab -->
          <div class="absolute inset-0 bg-[#0d1117] {activeTab === 'blame' ? 'z-10 visible' : 'z-0 invisible'}">
             <BlameView
               repoPath={repoPath}
               filePath={selectedFile?.path ?? null}
               onCommitSelect={handleBlameCommitSelect}
             />
          </div>

          <!-- Settings Tab -->
          <div class="absolute inset-0 bg-[#0d1117] {activeTab === 'settings' ? 'z-20 visible' : 'z-0 invisible'}">
             <SettingsView {repoPath} />
          </div>
      </div>
  </div>
</div>
