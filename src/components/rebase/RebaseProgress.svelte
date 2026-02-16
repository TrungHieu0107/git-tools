<script lang="ts">
  import { rebaseStore } from "../../lib/rebaseStore";
  import { toast } from "../../lib/toast.svelte";

  async function handleContinue() {
    try {
      await rebaseStore.continue();
    } catch (e: any) {
      toast.error(`Failed to continue: ${e}`);
    }
  }

  async function handleAbort() {
    try {
      await rebaseStore.abort();
    } catch (e: any) {
      toast.error(`Failed to abort: ${e}`);
    }
  }

  async function handleSkip() {
    try {
      await rebaseStore.skip();
    } catch (e: any) {
      toast.error(`Failed to skip: ${e}`);
    }
  }

  let progressPercent = $derived.by(() => {
    if (!$rebaseStore.step) return 0;
    return Math.round(($rebaseStore.step.current / $rebaseStore.step.total) * 100);
  });
</script>

{#if $rebaseStore.status !== "idle" && $rebaseStore.status !== "editingTodo"}
  <div class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[60] w-full max-w-xl">
    <div class="bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl p-4 flex flex-col gap-3">
      <!-- Status Row -->
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          {#if $rebaseStore.status === "conflicted"}
            <span class="flex h-2 w-2 rounded-full bg-red-500 animate-pulse"></span>
            <span class="text-xs font-bold text-red-400 uppercase tracking-wider">Conflict Detected</span>
          {:else}
            <span class="flex h-2 w-2 rounded-full bg-blue-500 animate-pulse"></span>
            <span class="text-xs font-medium text-blue-400 uppercase tracking-wider">Rebasing...</span>
          {/if}
        </div>
        {#if $rebaseStore.step}
          <div class="text-[10px] text-[#8b949e] font-mono">
            Step {$rebaseStore.step.current} of {$rebaseStore.step.total}
          </div>
        {/if}
      </div>

      <!-- Progress Details -->
      {#if $rebaseStore.step}
        <div class="bg-[#0d1117] rounded-md p-3 border border-[#30363d]">
          <div class="flex items-baseline justify-between gap-4">
            <span class="text-xs font-mono text-blue-400 shrink-0">{$rebaseStore.step.commitHash.slice(0, 7)}</span>
            <span class="text-xs text-[#c9d1d9] truncate">{$rebaseStore.step.commitMessage}</span>
          </div>
          <!-- Progress Bar -->
          <div class="mt-2 h-1 w-full bg-[#30363d] rounded-full overflow-hidden">
            <div 
              class="h-full bg-blue-500 transition-all duration-500" 
              style="width: {progressPercent}%"
            ></div>
          </div>
        </div>
      {/if}

      <!-- Actions -->
      <div class="flex items-center justify-between gap-4">
        <button 
          class="text-xs text-[#f85149] hover:bg-[#3b1f2c] px-3 py-1.5 rounded transition-colors"
          onclick={handleAbort}
        >
          Abort Rebase
        </button>

        <div class="flex gap-2">
          <button 
            class="text-xs text-[#c9d1d9] hover:bg-[#30363d] px-4 py-1.5 rounded transition-colors"
            onclick={handleSkip}
          >
            Skip Commit
          </button>
          <button 
            class="text-xs text-white bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed px-6 py-1.5 rounded-md font-semibold transition-colors"
            onclick={handleContinue}
            disabled={$rebaseStore.status === "conflicted"}
            title={$rebaseStore.status === "conflicted" ? "Resolve conflicts before continuing" : "Continue to next step"}
          >
            Continue
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
