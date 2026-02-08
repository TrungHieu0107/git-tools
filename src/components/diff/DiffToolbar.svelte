<script context="module" lang="ts">
  export type ViewMode = "side-by-side" | "hunk" | "inline";
</script>

<script lang="ts">
  interface Props {
    viewMode: ViewMode;
    onViewModeChange: (mode: ViewMode) => void;
    currentHunkIndex: number;
    totalHunks: number;
    onPrevHunk: () => void;
    onNextHunk: () => void;
  }
  let {
    viewMode,
    onViewModeChange,
    currentHunkIndex,
    totalHunks,
    onPrevHunk,
    onNextHunk,
  }: Props = $props();

  const modes: { value: ViewMode; label: string }[] = [
    { value: "side-by-side", label: "Side-by-Side" },
    { value: "hunk", label: "Hunks" },
    { value: "inline", label: "Inline" },
  ];

  let isFirstHunk = $derived(currentHunkIndex <= 0);
  let isLastHunk = $derived(currentHunkIndex >= totalHunks - 1);
</script>

<div class="h-8 px-3 flex items-center gap-3 border-b border-[#30363d] bg-[#161b22] shrink-0">
  <!-- View Mode Toggle -->
  <div class="flex items-center rounded-md border border-[#30363d] overflow-hidden">
    {#each modes as mode}
      <button
        class="px-2.5 py-1 text-[10px] font-medium transition-colors
               {viewMode === mode.value
                 ? 'bg-[#30363d] text-white'
                 : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
        onclick={() => onViewModeChange(mode.value)}
      >
        {mode.label}
      </button>
    {/each}
  </div>

  <!-- Separator -->
  <div class="w-px h-4 bg-[#30363d]"></div>

  <!-- Hunk Navigation -->
  <div class="flex items-center gap-1.5">
    <button
      class="p-1 rounded text-[#8b949e] hover:text-white hover:bg-[#21262d] disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:text-[#8b949e] transition-colors"
      disabled={isFirstHunk || totalHunks === 0}
      onclick={onPrevHunk}
      title="Previous change"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6"></polyline>
      </svg>
    </button>

    <span class="text-[10px] font-mono text-[#8b949e] min-w-[3ch] text-center select-none">
      {#if totalHunks > 0}
        {currentHunkIndex + 1}/{totalHunks}
      {:else}
        0/0
      {/if}
    </span>

    <button
      class="p-1 rounded text-[#8b949e] hover:text-white hover:bg-[#21262d] disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:text-[#8b949e] transition-colors"
      disabled={isLastHunk || totalHunks === 0}
      onclick={onNextHunk}
      title="Next change"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="9 18 15 12 9 6"></polyline>
      </svg>
    </button>
  </div>
</div>
