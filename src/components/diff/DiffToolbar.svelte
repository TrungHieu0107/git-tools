<script lang="ts">
  import EncodingSelector from "../../lib/components/EncodingSelector.svelte";
  
  export type ViewMode = "side-by-side" | "inline";

  interface Props {
    viewMode: ViewMode;
    onViewModeChange: (mode: ViewMode) => void;
    selectedEncoding?: string;
    onEncodingChange?: (encoding: string) => void;
  }
  let {
    viewMode,
    onViewModeChange,
    selectedEncoding,
    onEncodingChange,
  }: Props = $props();

  const modes: { value: ViewMode; label: string }[] = [
    { value: "side-by-side", label: "Side-by-Side" },
    { value: "inline", label: "Inline" },
  ];
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

  {#if onEncodingChange}
      <div class="w-px h-4 bg-[#30363d]"></div>
      <EncodingSelector 
          selectedEncoding={selectedEncoding} 
          on:change={(e: CustomEvent<string>) => onEncodingChange(e.detail)} 
      />
  {/if}
</div>
