<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { GitService, type AppSettings } from '../lib/GitService';

  let settings = $state<AppSettings | null>(null);
  let activeRepoName = $state('Select Repo');

  const dispatch = createEventDispatcher<{ change: void }>();

  async function loadSettings() {
    try {
      settings = await GitService.getSettings();
      if (settings && settings.active_repo_id) {
          const active = settings.repos.find(r => r.id === settings.active_repo_id);
          activeRepoName = active ? active.name : 'Select Repo';
      } else {
          activeRepoName = 'Select Repo';
      }
    } catch (e) {
      console.error(e);
    }
  }

  onMount(() => {
    loadSettings();
  });

  async function switchRepo(e: Event) {
      const select = e.target as HTMLSelectElement;
      const id = select.value;
      if (!id) return;
      
      try {
          await GitService.setActiveRepo(id);
          await loadSettings();
          dispatch('change');
          window.location.reload(); // Simple way to reset state for now, or we can use events
      } catch (e) {
          console.error(e);
      }
  }
</script>

<div class="relative inline-block text-left">
    {#if settings}
        <select 
            class="bg-gray-800 text-gray-200 text-xs font-medium rounded px-3 py-1.5 border border-gray-700 hover:bg-gray-700 outline-none focus:ring-1 focus:ring-blue-500 cursor-pointer appearance-none pr-8"
            onchange={switchRepo}
            value={settings.active_repo_id || ""}
        >
            <option value="" disabled>Switch Repository...</option>
            {#each settings.repos as repo}
                <option value={repo.id}>
                    {repo.name}
                </option>
            {/each}
        </select>
        <!-- Custom arrow since we removed appearance -->
        <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-400">
             <svg class="fill-current h-3 w-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20"><path d="M5.293 7.293a1 1 0 0 1 1.414 0L10 10.586l3.293-3.293a1 1 0 1 1 1.414 1.414l-4 4a1 1 0 0 1-1.414 0l-4-4a1 1 0 0 1 0-1.414z"/></svg>
        </div>
    {/if}
</div>
