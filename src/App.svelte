<script lang="ts">
  import { onMount } from 'svelte';
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { GitService, type RepoEntry, type FileStatus, type AppSettings } from './lib/GitService';
  import GlobalConfirmation from './components/GlobalConfirmation.svelte';
  import CreateBranchDialog from './components/CreateBranchDialog.svelte';
  import PromptDialog from './components/common/PromptDialog.svelte';
  import ToastContainer from './components/ToastContainer.svelte';
  import RepoManager from './components/RepoManager.svelte';
  import Workspace from './components/Workspace.svelte';
  import TabBar from './components/TabBar.svelte';

  let settings = $state<AppSettings | null>(null);
  let openRepos = $state<RepoEntry[]>([]);
  let activeRepoId = $state<string | null>(null);
  let showRepoManager = $state(false);

  // Load settings and hydrate state
  async function loadSettings() {
    try {
        settings = await GitService.getSettings();
        
        // Populate openRepos based on IDs
        if (settings) {
            const allRepos = settings.repos;
            const openIds = settings.open_repo_ids || [];
            
            // Map IDs to RepoEntries
            openRepos = openIds
                .map(id => allRepos.find(r => r.id === id))
                .filter((r): r is RepoEntry => !!r); 
            
            activeRepoId = settings.active_repo_id;
            
            // If no open repos, show manager
            if (openRepos.length === 0) {
                showRepoManager = true;
            } else {
                showRepoManager = false;
            }
        }
    } catch (e) {
        console.error("Failed to load settings:", e);
    }
  }

  // Handle Tab Actions
  async function handleActivate(id: string) {
      if (activeRepoId === id) return;
      try {
          await GitService.setActiveRepo(id);
          // Reload settings to get updated state (active_id might change)
          await loadSettings();
      } catch (e) {
          console.error("Failed to activate repo:", e);
      }
  }

  async function handleClose(id: string) {
      try {
          // If we are closing the active repo, we might need to switch logic is handled by backend
          // Backend updates active_repo_id automatically if active is closed
          await GitService.closeRepo(id);
          await loadSettings();
      } catch (e) {
          console.error("Failed to close repo:", e);
      }
  }

  function handleAdd() {
      showRepoManager = true;
      activeRepoId = null; // Deselect tabs while in manager? Or keep it overlay?
      // Let's treat Repo Manager as an overlay or a "new tab" state
      // For now, if activeRepoId is null/empty, we show manager.
  }

  // Events from RepoManager
  onMount(() => {
    loadSettings();
    
    // Listen for repo activation from Repo Manager
    const handleRepoActivated = async (e: CustomEvent<{ id: string }>) => {
        const id = e.detail.id;
        // Open the repo (add to open_ids)
        await GitService.openRepo(id);
        await handleActivate(id);
    };

    window.addEventListener('repo-activated', handleRepoActivated as EventListener);
    
    const handleCloseManager = () => {
        showRepoManager = false;
        // Restore active repo ID from settings if available
        if (settings) activeRepoId = settings.active_repo_id;
    };
    window.addEventListener('close-repo-manager', handleCloseManager);

    return () => {
        window.removeEventListener('repo-activated', handleRepoActivated as EventListener);
        window.removeEventListener('close-repo-manager', handleCloseManager);
    };
  });
</script>

<main class="h-screen w-screen bg-[#0d1117] text-[#c9d1d9] flex flex-col overflow-hidden font-sans text-sm selection:bg-[#1f6feb] selection:text-white">
  
  {#if settings}
    <!-- Tab Bar -->
    <TabBar 
        openRepos={openRepos} 
        activeRepoId={activeRepoId}
        onActivate={handleActivate} 
        onClose={handleClose} 
        onAdd={handleAdd}
    />
  {/if}

  <!-- Content Area -->
  <div class="flex-1 relative overflow-hidden">
      
      <!-- Repo Manager (Overlay or Empty State) -->
      {#if showRepoManager || openRepos.length === 0}
          <div class="absolute inset-0 z-50 bg-[#0d1117]">
              <RepoManager />
          </div>
      {/if}

      <!-- Workspaces (Persisted DOM) -->
      {#each openRepos as repo (repo.id)}
          <Workspace 
            repoId={repo.id} 
            repoPath={repo.path} 
            isActive={activeRepoId === repo.id && !showRepoManager} 
          />
      {/each}

  </div>

  <GlobalConfirmation />
  <CreateBranchDialog />
  <PromptDialog />
  <ToastContainer />
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background-color: #0d1117;
    color: #c9d1d9;
  }
</style>
