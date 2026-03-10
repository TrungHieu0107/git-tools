<script context="module" lang="ts">
  export type ViewMode = "side-by-side" | "inline";
</script>

<script lang="ts">
  import type { DiffStageLineTarget } from "../../lib/diff";
  import MonacoDiffEditor from "./MonacoDiffEditor.svelte";
  import EncodingSelector from "../../lib/components/EncodingSelector.svelte";

  interface Props {
    originalContent?: string;
    modifiedContent?: string;
    loading?: boolean;
    isTooLarge?: boolean;
    filePath?: string;
    header?: import("svelte").Snippet<[any]>;
    selectedEncoding?: string;
    onEncodingChange?: (encoding: string) => void;
    canStageLine?: boolean;
    onStageLine?: (line: DiffStageLineTarget) => void | Promise<void>;
    canUnstageLine?: boolean;
    onUnstageLine?: (line: DiffStageLineTarget) => void | Promise<void>;
  }

  let {
    originalContent = "",
    modifiedContent = "",
    loading = false,
    isTooLarge = false,
    filePath = "",
    header,
    selectedEncoding,
    onEncodingChange,
  }: Props = $props();

  let viewMode = $state<ViewMode>("side-by-side");

  function handleViewModeChange(mode: ViewMode) {
    viewMode = mode;
  }

  const modes: { value: ViewMode; label: string }[] = [
    { value: "side-by-side", label: "Side-by-Side" },
    { value: "inline", label: "Inline" },
  ];

  let toolbarProps = $derived({
    viewMode,
    onViewModeChange: handleViewModeChange,
    selectedEncoding,
    onEncodingChange,
  });

  let hasContent = $derived(originalContent.length > 0 || modifiedContent.length > 0);
</script>

<div class="flex flex-col h-full bg-[#0d1117] overflow-hidden">
  <!-- Header / Toolbar Area -->
  <div class="shrink-0">
    {#if header}
      {@render header(toolbarProps)}
    {:else}
      <!-- Default Toolbar Layout -->
      <div class="h-8 px-3 flex items-center gap-3 border-b border-[#30363d] bg-[#161b22] shrink-0">
        <!-- View Mode Toggle -->
        <div class="flex items-center rounded-md border border-[#30363d] overflow-hidden">
          {#each modes as mode}
            <button
              class="px-2.5 py-1 text-[10px] font-medium transition-colors
                     {viewMode === mode.value
                       ? 'bg-[#30363d] text-white'
                       : 'text-[#8b949e] hover:bg-[#21262d] hover:text-[#c9d1d9]'}"
              onclick={() => handleViewModeChange(mode.value)}
            >
              {mode.label}
            </button>
          {/each}
        </div>

        {#if onEncodingChange}
          <div class="w-px h-4 bg-[#30363d]"></div>
          <EncodingSelector 
            selectedEncoding={selectedEncoding} 
            on:change={(e: CustomEvent<string>) => onEncodingChange(e.detail)} 
          />
        {/if}
      </div>
    {/if}
  </div>

  <!-- Viewer Area -->
  <div class="flex-1 overflow-hidden relative">
    {#if loading}
      <div class="absolute inset-0 flex items-center justify-center text-[#8b949e] gap-2">
        <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
        <span class="text-xs">Loading diff...</span>
      </div>
    {:else if hasContent}
      <MonacoDiffEditor
        {originalContent}
        {modifiedContent}
        inlineView={viewMode === "inline"}
        {filePath}
      />
    {:else if isTooLarge}
      <div class="absolute inset-0 flex items-center justify-center text-[#8b949e] text-xs italic">
        File too large for diff view
      </div>
    {:else}
      <div class="absolute inset-0 flex items-center justify-center text-[#8b949e] text-xs">
        No diff content
      </div>
    {/if}
  </div>
</div>
