<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import ConflictList from './ConflictList.svelte';
  import ConflictEditor from './ConflictEditor.svelte';
  import { GitService } from '../lib/GitService';

  let { repoPath } = $props<{ repoPath?: string }>(); // kept for compatibility/display

  let selectedFile = $state<string | null>(null);
  
  // Ideally this state comes from backend
  let mergeState = $state<{ isMerging: boolean, isRebasing: boolean } | null>(null);

  onMount(async () => {
      // Mock or fetch merge state
      // mergeState = await GitService.getMergeState(); 
  });

  function handleSelect(event: CustomEvent<string>) {
      selectedFile = event.detail;
  }

  function handleResolved(event: CustomEvent<string>) {
      console.log("Resolved:", event.detail);
      selectedFile = null;
      reloadTrigger++;
  }

  let reloadTrigger = $state(0);

  async function continueOp() {
      try {
          await GitService.continueOp();
          alert("Continue operation started...");
      } catch(e) {
          alert("Error: " + e);
      }
  }

  async function abortOp() {
     if(!confirm("Are you sure you want to abort? All resolution progress will be lost.")) return;
      try {
          await GitService.abortOp();
          alert("Operation aborted.");
      } catch(e) {
            alert("Error: " + e);
      }
  }
</script>

<div class="flex flex-col h-full bg-gray-950">
    <!-- Top Banner -->
    <div class="bg-amber-900/30 border-b border-amber-800 p-2 flex justify-between items-center px-4">
        <div class="flex items-center gap-2 text-amber-200 text-sm">
            <span>⚠️</span>
            <span class="font-medium">You are in the middle of a merge/rebase</span>
        </div>
        <div class="flex gap-2">
            <button class="px-3 py-1 text-xs font-medium bg-amber-700 text-white rounded hover:bg-amber-600" onclick={continueOp}>
                Continue
            </button>
            <button class="px-3 py-1 text-xs font-medium bg-red-900/50 text-red-200 border border-red-800 rounded hover:bg-red-900" onclick={abortOp}>
                Abort
            </button>
        </div>
    </div>

    <div class="flex flex-1 overflow-hidden">
        <!-- List Pane -->
        <div class="h-full">
            {#key reloadTrigger} <!-- Simple way to force reload list -->
                <ConflictList {repoPath} on:select={handleSelect} />
            {/key}
        </div>

        <!-- Editor Pane -->
        <div class="flex-1 h-full border-l border-gray-800">
            <ConflictEditor 
                {repoPath} 
                filePath={selectedFile} 
                on:resolved={handleResolved} 
            />
        </div>
    </div>
</div>
