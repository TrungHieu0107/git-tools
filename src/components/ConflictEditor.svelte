<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { GitService, type ConflictFile } from '../lib/GitService';

  // repoPath removed from logic, kept in props only if parent passes it for display
  let { filePath } = $props<{ repoPath?: string; filePath: string | null }>();

  let conflictFile = $state<ConflictFile | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let manualContent = $state('');
  let isEditing = $state(false);
  let saving = $state(false);

  // Re-fetch when filePath changes
  $effect(() => {
    if (filePath) {
        loadConflict();
    } else {
        conflictFile = null;
    }
  });

  async function loadConflict() {
    if (!filePath) return;
    loading = true;
    error = null;
    isEditing = false;
    try {
      conflictFile = await GitService.getConflictFile(filePath);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  const dispatch = createEventDispatcher<{ resolved: string }>();

  async function resolveOurs() {
     if (!filePath) return;
     try {
         await GitService.resolveOurs(filePath);
         await GitService.markResolved(filePath);
         dispatch('resolved', filePath);
     } catch(e) {
         error = String(e);
     }
  }

  async function resolveTheirs() {
     if (!filePath) return;
     try {
         await GitService.resolveTheirs(filePath);
         await GitService.markResolved(filePath);
         dispatch('resolved', filePath);
     } catch(e) {
         error = String(e);
     }
  }

  function startManualEdit(initialContent: string) {
      manualContent = initialContent;
      isEditing = true;
  }

  function combineForEdit() {
      if (!conflictFile) return;
      // Simple concatenation as a starting point for manual merge
      const combined = `<<<<<<< OURS\n${conflictFile.ours}\n=======\n${conflictFile.theirs}\n>>>>>>> THEIRS`;
      startManualEdit(combined);
  }

  async function saveManual() {
      if (!filePath) return;
      saving = true;
      try {
          await GitService.writeFile(filePath, manualContent);
          await GitService.markResolved(filePath);
          dispatch('resolved', filePath);
          isEditing = false;
      } catch(e) {
          error = String(e);
      } finally {
          saving = false;
      }
  }

  function cancelEdit() {
      isEditing = false;
  }
</script>

<div class="flex flex-col h-full bg-gray-950 text-gray-200">
    {#if !filePath}
        <div class="flex items-center justify-center flex-1 text-gray-500">
            Select a file to resolve conflicts
        </div>
    {:else if loading}
        <div class="flex items-center justify-center flex-1 text-gray-400">
            Loading conflicts...
        </div>
    {:else if error}
        <div class="p-4 text-red-400">
            Error: {error}
        </div>
    {:else if isEditing}
        <div class="flex flex-col flex-1 p-3 sm:p-4 min-w-0">
            <h3 class="mb-2 text-lg font-semibold">Manual Resolution: {filePath}</h3>
            <textarea 
                class="flex-1 w-full p-4 mb-4 font-mono text-sm text-gray-200 bg-gray-800 border border-gray-700 rounded resize-none focus:outline-none focus:border-blue-500"
                bind:value={manualContent}
            ></textarea>
            <div class="flex flex-wrap gap-3">
                <button 
                    class="px-4 py-2 font-medium text-white bg-green-600 rounded hover:bg-green-700 disabled:opacity-50"
                    onclick={saveManual}
                    disabled={saving}
                >
                    {saving ? 'Saving...' : 'Save & Mark Resolved'}
                </button>
                <button 
                    class="px-4 py-2 font-medium text-gray-300 bg-gray-700 rounded hover:bg-gray-600"
                    onclick={cancelEdit}
                >
                    Cancel
                </button>
            </div>
        </div>
    {:else if conflictFile}
        <div class="flex flex-col h-full">
            <!-- Toolbar -->
            <div class="flex flex-wrap justify-between items-center gap-2 p-3 sm:p-4 border-b border-gray-800 bg-gray-900">
                <h3 class="font-medium truncate min-w-0" title={filePath}>{filePath}</h3>
                <div class="flex flex-wrap gap-2">
                     <button class="px-3 py-1.5 text-xs font-medium bg-green-900 text-green-100 rounded hover:bg-green-800 border border-green-700" onclick={resolveOurs}>
                        Accept Ours (Curr)
                     </button>
                     <button class="px-3 py-1.5 text-xs font-medium bg-blue-900 text-blue-100 rounded hover:bg-blue-800 border border-blue-700" onclick={resolveTheirs}>
                        Accept Theirs (Inc)
                     </button>
                     <button class="px-3 py-1.5 text-xs font-medium bg-yellow-900 text-yellow-100 rounded hover:bg-yellow-800 border border-yellow-700" onclick={combineForEdit}>
                        Merge Both...
                     </button>
                     <button class="px-3 py-1.5 text-xs font-medium bg-gray-700 text-gray-200 rounded hover:bg-gray-600" onclick={() => startManualEdit(conflictFile?.ours || '')}>
                        Edit Manually
                     </button>
                </div>
            </div>

            <!-- Content Columns -->
            <div class="flex-1 grid grid-cols-2 divide-x divide-gray-800 overflow-hidden max-[900px]:grid-cols-1 max-[900px]:divide-x-0 max-[900px]:divide-y">
                <!-- OURS -->
                <div class="flex flex-col overflow-hidden">
                    <div class="p-2 text-xs font-bold text-center text-green-400 bg-green-900/10 border-b border-gray-800">
                        OURS (current)
                    </div>
                    <pre class="flex-1 p-4 overflow-auto font-mono text-xs whitespace-pre-wrap">{conflictFile.ours}</pre>
                </div>
                <!-- THEIRS -->
                <div class="flex flex-col overflow-hidden">
                    <div class="p-2 text-xs font-bold text-center text-blue-400 bg-blue-900/10 border-b border-gray-800">
                        THEIRS (incoming)
                    </div>
                     <pre class="flex-1 p-4 overflow-auto font-mono text-xs whitespace-pre-wrap">{conflictFile.theirs}</pre>
                </div>
            </div>
            
            {#if conflictFile.base}
                 <!-- Optional: Base view at bottom or toggleable, not requested explicitly as main view but good to handle if desired. For now, 2 columns is standard. -->
            {/if}
        </div>
    {/if}
</div>
