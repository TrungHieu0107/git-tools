<script lang="ts">
  import ConflictList from './ConflictList.svelte';
  import ConflictEditor from './ConflictEditor.svelte';
  import { GitService } from '../lib/GitService';
  import { toast } from '../lib/toast.svelte';
  import { triggerGraphReload } from '../lib/stores/git-events';

  let { repoPath } = $props<{ repoPath?: string }>();

  let selectedFile = $state<string | null>(null);
  let reloadTrigger = $state(0);
  let aborting = $state(false);
  let committing = $state(false);

  function handleSelect(event: CustomEvent<string>) {
      selectedFile = event.detail;
  }

  function handleResolved(event: CustomEvent<string>) {
      selectedFile = null;
      reloadTrigger++;
  }

  async function continueOp() {
      if (committing) return;
      committing = true;
      try {
          // Read the operation state to get merge message and check for unresolved conflicts
          const opState = await GitService.getOperationState(repoPath);

          if (opState.hasConflicts && opState.conflictPaths.length > 0) {
              toast.error(`Cannot commit: ${opState.conflictPaths.length} file(s) still have conflicts.`);
              return;
          }

          // For rebase: delegate to rebase controls
          if (opState.isRebasing) {
              toast.info("Use the rebase controls to continue the rebase.");
              return;
          }

          // Build merge message from operation state
          let mergeMessage = "Merge commit";
          if (opState.isMerging && opState.theirsBranch) {
              mergeMessage = `Merge branch '${opState.theirsBranch}'`;
          }

          const result = await GitService.commit(mergeMessage, repoPath);
          if (result.success) {
              toast.success("Merge committed successfully");
              triggerGraphReload();
          } else {
              toast.error(`Commit failed: ${result.stderr}`);
          }
      } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          toast.error(`Continue failed: ${msg}`);
      } finally {
          committing = false;
      }
  }

  async function abortOp() {
     if (aborting) return;
     if (!confirm("Are you sure you want to abort? All resolution progress will be lost.")) return;
     aborting = true;
     try {
         const result = await GitService.abortOperation(repoPath);
         if (result.success) {
             toast.success("Operation aborted");
             triggerGraphReload();
         } else {
             toast.error(`Abort failed: ${result.stderr}`);
         }
     } catch (e) {
         const msg = e instanceof Error ? e.message : String(e);
         toast.error(`Abort failed: ${msg}`);
     } finally {
         aborting = false;
     }
  }
</script>

<div class="flex flex-col h-full bg-gray-950">
    <!-- Top Banner -->
    <div class="bg-amber-900/30 border-b border-amber-800 p-2 px-4 flex flex-wrap justify-between items-center gap-2">
        <div class="flex items-center gap-2 text-amber-200 text-sm">
            <span>⚠️</span>
            <span class="font-medium">You are in the middle of a merge/rebase</span>
        </div>
        <div class="flex flex-wrap gap-2">
            <button
                class="px-3 py-1 text-xs font-medium bg-amber-700 text-white rounded hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={continueOp}
                disabled={committing || aborting}
            >
                {committing ? 'Committing...' : 'Commit Merge'}
            </button>
            <button
                class="px-3 py-1 text-xs font-medium bg-red-900/50 text-red-200 border border-red-800 rounded hover:bg-red-900 disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={abortOp}
                disabled={aborting || committing}
            >
                {aborting ? 'Aborting...' : 'Abort'}
            </button>
        </div>
    </div>

    <div class="flex flex-1 overflow-hidden max-[900px]:flex-col">
        <!-- List Pane -->
        <div class="h-full max-[900px]:h-[42%]">
            {#key reloadTrigger}
                <ConflictList {repoPath} on:select={handleSelect} />
            {/key}
        </div>

        <!-- Editor Pane -->
        <div class="flex-1 h-full border-l border-gray-800 max-[900px]:border-l-0 max-[900px]:border-t">
            <ConflictEditor
                {repoPath}
                filePath={selectedFile}
                on:resolved={handleResolved}
            />
        </div>
    </div>
</div>
