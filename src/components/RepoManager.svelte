<script lang="ts">
  import { onMount } from 'svelte';
  import { GitService, type AppSettings } from '../lib/GitService';
  import { open } from '@tauri-apps/plugin-dialog';

  let settings = $state<AppSettings | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let newRepoName = $state('');
  let newRepoPath = $state('');
  let adding = $state(false);

  async function loadSettings() {
    loading = true;
    error = null;
    try {
      settings = await GitService.getSettings();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadSettings();
  });

  async function browsePath() {
      try {
          const selected = await open({
              directory: true,
              multiple: false,
              title: "Select Repository Folder"
          });
          if (selected) {
              newRepoPath = selected as string;
              // Auto-fill name if empty - handle trailing slashes
              if (!newRepoName) {
                  const cleanedPath = newRepoPath.replace(/[\\/]+$/, '');
                  const parts = cleanedPath.split(/[\\/]/);
                  const folderName = parts[parts.length - 1];
                  if (folderName) newRepoName = folderName;
              }
          }
      } catch (e) {
          error = "Failed to open dialog: " + e;
      }
  }

  function handleBack() {
      // Notify parent to close manager
      window.dispatchEvent(new CustomEvent('close-repo-manager'));
  }

  async function addRepo() {
    if (!newRepoName || !newRepoPath) return;
    adding = true;
    error = null;
    try {
      settings = await GitService.addRepo(newRepoName, newRepoPath);
      const newId = settings.repos[settings.repos.length - 1].id;
      newRepoName = '';
      newRepoPath = '';
      // Notify parent to reload and activate the new repo
      window.dispatchEvent(new CustomEvent('repo-activated', { detail: { id: newId } }));
    } catch (e) {
      error = String(e);
    } finally {
      adding = false;
    }
  }

  async function removeRepo(id: string) {
    if (!confirm("Are you sure you want to remove this repository from the list?")) return;
    try {
      settings = await GitService.removeRepo(id);
    } catch (e) {
      error = String(e);
    }
  }

  async function setActive(id: string) {
    try {
        await GitService.setActiveRepo(id);
        // Reload settings to reflect active change
        settings = await GitService.getSettings();
        // Notify parent
        window.dispatchEvent(new CustomEvent('repo-activated', { detail: { id } }));
    } catch (e) {
        error = String(e);
    }
  }
</script>

<div class="flex flex-col h-full bg-gray-950 text-gray-200 p-8">
    <div class="max-w-3xl mx-auto w-full">
        <h1 class="text-2xl font-bold mb-6 flex items-center gap-3">
            <span>📚</span>
            Repository Manager
        </h1>

        <!-- Add Repo Form -->
        <div class="bg-gray-900 border border-gray-800 rounded-lg p-6 mb-8 shadow-lg relative">
            <div class="flex justify-between items-center mb-4">
                <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Add Repository</h2>
                {#if settings && settings.open_repo_ids.length > 0}
                    <button 
                        class="text-xs text-gray-500 hover:text-gray-300 transition-colors flex items-center gap-1"
                        onclick={handleBack}
                    >
                        ✕ Close
                    </button>
                {/if}
            </div>
            <div class="flex gap-4 items-end">
                <div class="flex-1">
                    <label class="block text-xs text-gray-500 mb-1">Name</label>
                    <input 
                        type="text" 
                        class="w-full bg-gray-950 border border-gray-800 rounded px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
                        placeholder="My Awesome Project"
                        bind:value={newRepoName}
                    />
                </div>
                <div class="flex-[2]">
                    <label class="block text-xs text-gray-500 mb-1">Path</label>
                    <div class="flex gap-2">
                        <input 
                            type="text" 
                            class="flex-1 bg-gray-950 border border-gray-800 rounded px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
                            placeholder="C:\Users\Dev\Projects\..."
                            bind:value={newRepoPath}
                        />
                        <button 
                             class="px-3 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded border border-gray-700 text-sm"
                             onclick={browsePath}
                             title="Browse Folder"
                        >
                            📂
                        </button>
                    </div>
                </div>
                <button 
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded font-medium text-sm flex items-center gap-2 disabled:opacity-50"
                    onclick={addRepo}
                    disabled={adding || !newRepoName || !newRepoPath}
                >
                    {#if adding}Adding...{:else}Add Repo{/if}
                </button>
            </div>
            {#if error}
                <div class="mt-4 p-3 bg-red-900/20 text-red-300 text-sm rounded border border-red-900/50">
                    {error}
                </div>
            {/if}
        </div>

        <!-- Repo List -->
        <div class="bg-gray-900 border border-gray-800 rounded-lg shadow-lg overflow-hidden">
             <div class="px-6 py-4 border-b border-gray-800 flex justify-between items-center bg-gray-900/50">
                <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Saved Repositories</h2>
                {#if loading}
                    <span class="text-xs text-gray-500">Loading...</span>
                {/if}
            </div>
            
            {#if settings && settings.repos.length > 0}
                <ul class="divide-y divide-gray-800">
                    {#each settings.repos as repo (repo.id)}
                        <li class="p-4 flex items-center justify-between hover:bg-gray-800/50 transition-colors {settings.active_repo_id === repo.id ? 'bg-blue-900/10' : ''}">
                            <div class="flex items-center gap-4">
                                <div class="w-10 h-10 rounded-full flex items-center justify-center bg-gray-800 text-lg">
                                    📁
                                </div>
                                <div>
                                    <div class="font-medium text-gray-200 flex items-center gap-2">
                                        {repo.name}
                                        {#if settings.active_repo_id === repo.id}
                                            <span class="px-2 py-0.5 rounded-full bg-green-900/50 text-green-400 text-[10px] font-bold border border-green-900">ACTIVE</span>
                                        {/if}
                                    </div>
                                    <div class="text-sm text-gray-500 font-mono">{repo.path}</div>
                                </div>
                            </div>
                            <div class="flex items-center gap-3">
                                {#if settings.active_repo_id !== repo.id}
                                    <button 
                                        class="px-3 py-1.5 text-xs font-medium bg-gray-800 hover:bg-gray-700 text-gray-300 rounded border border-gray-700"
                                        onclick={() => setActive(repo.id)}
                                    >
                                        Set Active
                                    </button>
                                {/if}
                                <button 
                                    class="p-2 text-gray-500 hover:text-red-400 rounded hover:bg-red-900/20"
                                    title="Remove Repository"
                                    onclick={() => removeRepo(repo.id)}
                                >
                                    🗑
                                </button>
                            </div>
                        </li>
                    {/each}
                </ul>
            {:else if !loading}
                <div class="p-8 text-center text-gray-500">
                    <div class="text-4xl mb-3">📭</div>
                    <p>No repositories found. Add one above to get started!</p>
                </div>
            {/if}
        </div>
    </div>
</div>
