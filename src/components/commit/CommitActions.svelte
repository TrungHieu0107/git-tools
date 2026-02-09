<script lang="ts">
  interface Props {
      stagedCount: number;
      loading: boolean;
      onCommit: (message: string, push: boolean) => void;
  }
  let { stagedCount, loading, onCommit }: Props = $props();

  let message = $state("");
  let pushAfterCommit = $state(false);

  function handleCommit() {
      if (stagedCount === 0 || !message.trim() || loading) return;
      onCommit(message, pushAfterCommit);
      message = ""; // Reset message on success? Parent should handle clears, but simple clear here
  }
</script>

<div class="border-t border-[#30363d] bg-[#161b22] p-3 flex flex-col gap-3">
    <!-- Message Input -->
    <textarea
        bind:value={message}
        class="w-full bg-[#0d1117] border border-[#30363d] rounded p-2 text-xs text-[#c9d1d9] placeholder-[#484f58] focus:border-[#58a6ff] focus:outline-none resize-none h-20"
        placeholder="Commit message"
        disabled={loading}
    ></textarea>

    <!-- Toolbar -->
    <div class="flex items-center justify-between">
        <label class="flex items-center gap-2 text-xs text-[#8b949e] cursor-pointer select-none">
            <input type="checkbox" bind:checked={pushAfterCommit} disabled={loading} class="rounded border-[#30363d] bg-[#0d1117] text-[#238636] focus:ring-0 focus:ring-offset-0" />
            <span>Commit & Push</span>
        </label>

        <button
            class="px-4 py-1.5 rounded bg-[#238636] text-white text-xs font-semibold hover:bg-[#2ea043] disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm border border-[rgba(240,246,252,0.1)]"
            disabled={stagedCount === 0 || !message.trim() || loading}
            onclick={handleCommit}
        >
            {#if loading}
                <span class="flex items-center gap-2">
                     <svg class="animate-spin h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
                     Working...
                </span>
            {:else}
                Commit {stagedCount > 0 ? `(${stagedCount})` : ''}
            {/if}
        </button>
    </div>
</div>
