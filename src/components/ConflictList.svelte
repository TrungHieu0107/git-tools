<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { GitService } from '../lib/GitService';

  // repoPath prop removed, using backend context
  let { repoPath } = $props<{ repoPath?: string }>(); // kept optional for display/backward compat if needed, but logic uses context
  
  let conflicts = $state<string[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const dispatch = createEventDispatcher<{ select: string }>();

  async function loadConflicts() {
    loading = true;
    error = null;
    try {
      conflicts = await GitService.getConflicts();
    } catch (e) {
      error = String(e);
      console.error("Failed to load conflicts:", e);
    } finally {
      loading = false;
    }
  }

  // Reload when component mounts or if we want to force it
  $effect(() => {
      loadConflicts();
  });

  function selectFile(file: string) {
    dispatch('select', file);
  }
</script>

<div class="flex flex-col h-full bg-gray-900 border-r border-gray-800 w-64 max-[900px]:w-full max-[900px]:border-r-0 max-[900px]:border-b">
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-gray-800">
        <h2 class="text-sm font-semibold text-gray-200">Changes</h2>
        {#if !loading && conflicts.length > 0}
             <span class="px-2 py-0.5 text-xs font-bold text-white bg-red-600 rounded-full">
                {conflicts.length}
            </span>
        {/if}
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
        {#if loading}
            <div class="flex items-center justify-center py-8 text-xs text-gray-500">
                Loading conflicts...
            </div>
        {:else if error}
             <div class="p-3 text-xs text-red-400 bg-red-900/20 m-2 rounded">
                Error: {error}
            </div>
        {:else if conflicts.length === 0}
            <div class="flex items-center justify-center py-8 text-sm text-green-500">
                No conflicts 🎉
            </div>
        {:else}
            <ul class="py-2">
                {#each conflicts as file}
                    <li>
                        <button 
                            class="w-full text-left px-4 py-2 text-sm text-gray-400 hover:bg-gray-800 hover:text-white transition-colors flex items-center gap-2"
                            onclick={() => selectFile(file)}
                        >
                            <span class="text-red-500 font-bold">C</span>
                            <span class="truncate">{file}</span>
                        </button>
                    </li>
                {/each}
            </ul>
        {/if}
    </div>
</div>
