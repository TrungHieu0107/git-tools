<script lang="ts">
  interface Props {
    stagedCount: number;
    busy: boolean;
    generating: boolean;
    onCommit: (message: string, push: boolean) => Promise<void> | void;
    onGenerate: () => Promise<void> | void;
    message?: string;
  }

  let {
    stagedCount,
    busy,
    generating,
    onCommit,
    onGenerate,
    message = $bindable(""),
  }: Props = $props();

  let pushAfterCommit = $state(false);

  let canGenerate = $derived(stagedCount > 0 && !busy && !generating);
  let canCommit = $derived(stagedCount > 0 && !!message.trim() && !busy && !generating);

  async function handleCommit() {
    if (!canCommit) return;
    await onCommit(message, pushAfterCommit);
    message = "";
  }

  async function handleGenerate() {
    if (!canGenerate) return;
    await onGenerate();
  }
</script>

<div class="border-t border-[#30363d] bg-[#161b22] p-2.5 flex flex-col gap-2">
  <div class="flex items-center justify-between">
    <span class="text-[10px] uppercase tracking-wider text-[#8b949e] font-semibold">Commit Message</span>
    <span class="text-[10px] px-1.5 py-0.5 rounded border border-[#30363d] bg-[#0d1117] text-[#8b949e]">
      {stagedCount} staged
    </span>
  </div>

  <textarea
    bind:value={message}
    class="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-2.5 py-2 text-[12px] leading-relaxed text-[#c9d1d9] placeholder-[#6e7681] focus:border-[#58a6ff] focus:outline-none focus:ring-1 focus:ring-[#58a6ff] resize-none h-16 transition-colors"
    placeholder="Describe staged changes..."
    disabled={busy || generating}
  ></textarea>

  {#if generating}
    <div class="flex items-center gap-2 px-2 py-1.5 rounded-md border border-[#1f6feb]/40 bg-[#0b1f3a] text-[#8ab4f8] text-[11px]">
      <svg class="animate-spin h-3.5 w-3.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
      </svg>
      <span>Gemini is generating commit message...</span>
    </div>
  {/if}

  <div class="flex items-center justify-between gap-2">
    <label class="inline-flex items-center gap-2 text-xs text-[#8b949e] cursor-pointer select-none px-2 py-1 rounded-md border border-[#30363d] bg-[#0d1117] hover:bg-[#111822] transition-colors">
      <input
        type="checkbox"
        bind:checked={pushAfterCommit}
        disabled={busy || generating}
        class="rounded border-[#30363d] bg-[#0d1117] text-[#238636] focus:ring-0 focus:ring-offset-0"
      />
      <span>Commit & Push</span>
    </label>

    {#if stagedCount === 0}
      <span class="text-[10px] text-[#8b949e]">Stage files to enable actions</span>
    {/if}
  </div>

  <div class="grid grid-cols-2 gap-2">
    <button
      class="h-9 rounded-md bg-[#1f2937] text-[#c9d1d9] text-xs font-semibold hover:bg-[#263446] disabled:opacity-45 disabled:cursor-not-allowed transition-colors border border-[#30363d] shadow-sm flex items-center justify-center gap-2"
      disabled={!canGenerate}
      onclick={handleGenerate}
      title="Generate commit message from staged changes"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 3l1.8 4.2L18 9l-4.2 1.8L12 15l-1.8-4.2L6 9l4.2-1.8L12 3z"></path>
        <path d="M19 14l.9 2.1L22 17l-2.1.9L19 20l-.9-2.1L16 17l2.1-.9L19 14z"></path>
      </svg>
      {#if generating}
        Generating...
      {:else}
        Generate
      {/if}
    </button>

    <button
      class="h-9 rounded-md bg-[#238636] text-white text-xs font-semibold hover:bg-[#2ea043] disabled:opacity-50 disabled:cursor-not-allowed transition-colors border border-[rgba(240,246,252,0.12)] shadow-sm flex items-center justify-center gap-2"
      disabled={!canCommit}
      onclick={handleCommit}
    >
      {#if busy}
        <span class="flex items-center gap-2">
          <svg class="animate-spin h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
          Working...
        </span>
      {:else if generating}
        <span class="flex items-center gap-2">
          <svg class="animate-spin h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
          Waiting...
        </span>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M5 12l4 4L19 6"></path>
        </svg>
        Commit
      {/if}
    </button>
  </div>
</div>
