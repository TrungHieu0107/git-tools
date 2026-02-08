<script lang="ts">
  import { onMount } from 'svelte';
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { GitService, type RepoEntry, type FileStatus } from './lib/GitService';
  import { runGit, type GitResponse, type GitError } from "./lib/git"; 
  import { getAuthRequiredMessage } from "./lib/git-errors";
  import { parseGitLog, calculateGraphLayout, type GraphNode, type GraphEdge } from "./lib/graph-layout";
  import RepoManager from './components/RepoManager.svelte';
  import Conflicts from './components/Conflicts.svelte';
  import TerminalPanel from './components/TerminalPanel.svelte';
  import RepoSelector from './components/RepoSelector.svelte';
  import CommitGraph from './components/CommitGraph.svelte';
  import CommitPanel from './components/CommitPanel.svelte';
  import FileHistoryPanel from './components/FileHistoryPanel.svelte';
  import BranchExplorer from './components/BranchExplorer.svelte';
  import GlobalConfirmation from './components/GlobalConfirmation.svelte';
  import ToastContainer from './components/ToastContainer.svelte';
  import SettingsView from './components/SettingsView.svelte';
  import { graphReloadRequested } from './lib/stores/git-events';
  import ResizablePanel from './components/resize/ResizablePanel.svelte';

  let activeRepo = $state<RepoEntry | null>(null);
  let loading = $state(true);
  
  // Fix for binding error: declare repoPath as state
  let repoPath = $state("");

  // Simple view routing
  let currentView = $state<'repos' | 'conflicts'>('repos'); 
  
  // Graph State
  let graphNodes = $state<GraphNode[]>([]);
  let graphEdges = $state<GraphEdge[]>([]);
  let commitCount = $state("50");
  let graphLoading = $state(false);
  let hasConflicts = $state(false);

  let activeTab = $state<"terminal" | "graph" | "commit" | "history" | "settings">("terminal");
  let pendingPushCount = $state(0);
  let commitPanel = $state<any>(null);
  let selectedFile = $state<FileStatus | null>(null);

  async function updateConflictStatus() {
    if (!repoPath) {
        hasConflicts = false;
        return;
    }
    try {
        hasConflicts = await GitService.checkConflictState(repoPath);
    } catch (e) {
        console.error("Failed to check conflict state:", e);
        hasConflicts = false;
    }
  }

  async function checkActiveRepo() {
    loading = true;
    try {
      activeRepo = await GitService.getActiveRepo();
      if (activeRepo) {
         repoPath = activeRepo.path; // Sync state
         await updateConflictStatus();
         // Keep current view if possible, else default to 'repos'
         if (currentView === 'repos') {
             currentView = 'conflicts';
         }
      } else {
        currentView = 'repos';
      }
    } catch (e) {
      console.error(e);
      // Fallback to repo manager if error
      currentView = 'repos';
    } finally {
      loading = false;
    }
  }

  function formatErrorMessage(message: string): string {
    return getAuthRequiredMessage(message) ?? message;
  }

  async function loadGraph(switchToGraph: any = true) {
    if (!repoPath) return;
    graphLoading = true;
    try {
      // Refresh pending count
      try {
          pendingPushCount = await GitService.getPendingCommitsCount(repoPath);
      } catch (e) {
          console.error("Failed to get pending count", e);
      }

      // Format must match parseGitLog expectation: hash|parents|refs|author|date|subject
      const logOutput = await GitService.getCommitGraph(parseInt(commitCount), repoPath);
      const commits = parseGitLog(logOutput);
      const layout = calculateGraphLayout(commits);
      graphNodes = layout.nodes;
      graphEdges = layout.edges;
      
      // If explicit true or event object (truthy), switch. If strict false, don't.
      if (switchToGraph !== false) {
          activeTab = "graph";
      }
    } catch (e) {
      console.error("Failed to load graph:", e);
    } finally {
      graphLoading = false;
    }
  }

  // Subscribe to store manually to avoid runes conflict or linter issue
  let reloadTrigger = $state(0);
  onMount(() => {
      const unsub = graphReloadRequested.subscribe(v => reloadTrigger = v);
      return unsub;
  });

  $effect(() => {
    if (reloadTrigger > 0 && repoPath) {
        // Reload graph but preserve current view (don't force switch to graph tab)
        loadGraph(false);
    }
  });


  function navigateToRepos() {
    currentView = 'repos';
    activeRepo = null; // Clear active repo when navigating to repos view
    repoPath = ""; // Clear repo path
  }

  onMount(() => {
    checkActiveRepo();
    
    // Listen for repo changes from RepoManager
    const handleRepoChange = () => {
        checkActiveRepo();
    };
    window.addEventListener('repo-activated', handleRepoChange);

    return () => {
        window.removeEventListener('repo-activated', handleRepoChange);
    };
  });


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

<main class="h-screen w-screen bg-[#0d1117] text-[#c9d1d9] flex overflow-hidden font-sans text-sm selection:bg-[#1f6feb] selection:text-white">
  <!-- Left Sidebar (Resizable) -->
  <ResizablePanel initialSize={288} minSize={220} maxSize={450} side="right">
    <aside class="w-full h-full bg-[#161b22] border-r border-[#30363d] flex flex-col shrink-0 z-10">
      <!-- Header -->
      <div class="h-12 border-b border-[#30363d] flex items-center px-4 gap-2 select-none bg-[#161b22]">
        <div class="text-[#238636]">
          {@html Icons.Git}
        </div>
        <h1 class="font-semibold text-white tracking-tight flex-1">GitHelper</h1>
        <button 
           onclick={() => activeTab = 'settings'}
           class="p-1.5 rounded hover:bg-[#21262d] text-[#8b949e] hover:text-[#c9d1d9] transition-colors"
           title="Settings"
        >
           {@html Icons.Settings}
        </button>
      </div>

      <!-- Controls (Static) -->
      <div class="p-4 space-y-4 flex-none">
          <!-- Repo Section -->
          <div class="space-y-2">
              <label for="repo" class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider flex items-center gap-2">
                {@html Icons.Folder} Repository
              </label>
              <div class="relative group">
                  <input
                      id="repo"
                      type="text"
                      bind:value={repoPath}
                      placeholder="path/to/repo"
                      class="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-[#c9d1d9] placeholder-[#484f58] focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] outline-none transition-colors text-xs font-mono"
                  />
              </div>
          </div>

          <!-- Command Section -->
          <div class="space-y-2">
              <div class="flex gap-2">
                 {#if hasConflicts}
                     <button 
                        class="text-xs px-2 py-1 rounded hover:bg-red-900/50 text-red-400 font-bold border border-red-900 animate-pulse {currentView === 'conflicts' ? 'bg-red-900 text-white' : ''}"
                        onclick={() => currentView = 'conflicts'}
                     >
                        ⚠ Conflicts
                     </button>
                 {/if}
                  <button 
                    class="text-xs px-2 py-1 rounded hover:bg-gray-800 {currentView === 'repos' ? 'text-white font-bold' : 'text-gray-500'}"
                    onclick={navigateToRepos}
                 >
                    Repos
                 </button>
             </div>
          </div>

          <!-- Graph Section -->
          <div class="space-y-2">
              <label for="limit" class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider flex items-center gap-2">
                {@html Icons.Network} Commit Graph
              </label>
              <div class="flex gap-2">
                <input
                    id="limit"
                    type="number"
                    bind:value={commitCount}
                    class="w-16 bg-[#0d1117] border border-[#30363d] rounded-md px-2 py-1.5 text-[#c9d1d9] text-center outline-none focus:border-[#58a6ff] text-xs"
                />
                <button
                    onclick={loadGraph}
                    disabled={graphLoading}
                    class="flex-1 bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 text-white font-medium py-1.5 px-3 rounded-md shadow-sm transition-all active:scale-[0.98] flex items-center justify-center gap-2 border border-[rgba(240,246,252,0.1)] text-xs"
                >
                    {#if graphLoading}
                        <span>Loading...</span>
                    {:else}
                        <span>Load Graph</span>
                    {/if}
                </button>
              </div>
          </div>
      </div>

      <!-- Branch Explorer (Flexible) -->
      <div class="flex-1 overflow-hidden border-t border-[#30363d]">
          <BranchExplorer {repoPath} />
      </div>
      
      <!-- Footer Info -->
      <div class="p-3 border-t border-[#30363d] text-[10px] text-[#484f58] text-center bg-[#161b22]">
        GitHelper v0.2.0
      </div>
    </aside>
  </ResizablePanel>

  <!-- Right Panel - Content -->
  <div class="flex-1 flex flex-col min-w-0 bg-[#0d1117] relative">
      <!-- Tabs Header -->
      <div class="h-12 border-b border-[#30363d] flex items-center px-2 bg-[#161b22] gap-1">
        <button 
           onclick={() => activeTab = "terminal"}
           class="px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'terminal' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           {@html Icons.Terminal} Terminal
        </button>
        <button 
           onclick={() => activeTab = "graph"}
           class="px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'graph' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           {@html Icons.Network} Graph
        </button>
        <button 
           onclick={() => activeTab = "commit"}
           class="px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'commit' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"></circle><line x1="1.05" y1="12" x2="7" y2="12"></line><line x1="17.01" y1="12" x2="22.96" y2="12"></line></svg> Commit
        </button>
        <button 
           onclick={() => activeTab = "history"}
           class="px-4 py-1.5 rounded-md text-xs font-medium transition-colors flex items-center gap-2 {activeTab === 'history' ? 'bg-[#30363d] text-white' : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        >
           <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg> History
        </button>
      </div>

      <!-- Tab Content area -->
      <div class="flex-1 relative overflow-hidden">
         <!-- Terminal Tab -->
         <div class="absolute inset-0 flex flex-col {activeTab === 'terminal' ? 'z-10 visible' : 'z-0 invisible'}">
            {#if activeTab === 'terminal'}
                <TerminalPanel {repoPath} isActive={activeTab === 'terminal'} />
            {/if}
         </div>

         <!-- Graph Tab -->
         <div class="absolute inset-0 bg-[#0d1117] {activeTab === 'graph' ? 'z-10 visible' : 'z-0 invisible'}">
            {#if graphNodes.length === 0 && !graphLoading}
                <div class="absolute inset-0 flex flex-col items-center justify-center text-[#484f58] select-none">
                  <p class="text-sm">No graph loaded. Enter commit limit and click "Load Graph".</p>
                </div>
            {:else}
                <CommitGraph nodes={graphNodes} edges={graphEdges} repoPath={repoPath} pendingPushCount={pendingPushCount} onGraphReload={loadGraph} />
            {/if}
         </div>

          <!-- Commit Tab -->
          <div class="absolute inset-0 {activeTab === 'commit' ? 'z-10 visible' : 'z-0 invisible'}">
             <CommitPanel bind:this={commitPanel} repoPath={repoPath} isActive={activeTab === 'commit'} bind:selectedFile={selectedFile} />
          </div>

          <!-- History Tab -->
          <div class="absolute inset-0 bg-[#0d1117] {activeTab === 'history' ? 'z-10 visible' : 'z-0 invisible'}">
             <FileHistoryPanel 
               repoPath={repoPath} 
               filePath={selectedFile?.path ?? null} 
               onFileSelect={(path) => { selectedFile = { path, status: '', staged: false }; }}
             />
          </div>

          <!-- Settings Tab -->
          <div class="absolute inset-0 bg-[#0d1117] {activeTab === 'settings' ? 'z-20 visible' : 'z-0 invisible'}">
             <SettingsView />
          </div>
      </div>
  </div>

  <GlobalConfirmation />
  <ToastContainer />
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background-color: #0d1117;
    color: #c9d1d9;
  }
  
  .custom-scrollbar::-webkit-scrollbar {
    width: 10px;
    height: 10px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: #0d1117;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #30363d;
    border: 2px solid #0d1117;
    border-radius: 99px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #484f58;
  }
</style>
