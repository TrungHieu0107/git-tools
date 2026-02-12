<script lang="ts">
  import { tick } from "svelte";
  import type { DiffResult, DiffHunk, DiffStageLineTarget } from "../../lib/diff";
  import InlineDiffViewer from "./InlineDiffViewer.svelte";
  import SideBySideDiffViewer from "./SideBySideDiffViewer.svelte";
  import DiffToolbar, { type ViewMode } from "./DiffToolbar.svelte";

  interface Props {
    diffResult: DiffResult | null;
    hunks?: DiffHunk[];
    loading?: boolean;
    isTooLarge?: boolean;
    // Optional snippet for custom header/toolbar layout
    // If not provided, a default basic toolbar is rendered
    header?: import("svelte").Snippet<[any]>;
    selectedEncoding?: string;
    onEncodingChange?: (encoding: string) => void;
    canStageLine?: boolean;
    onStageLine?: (line: DiffStageLineTarget) => void | Promise<void>;
  }

  let {
    diffResult,
    hunks = [],
    loading = false,
    isTooLarge = false,
    header,
    selectedEncoding,
    onEncodingChange,
    canStageLine = false,
    onStageLine,
  }: Props = $props();

  // -- State --
  let viewMode = $state<ViewMode>("side-by-side");
  let currentHunkIndex = $state(0);
  let containerEl = $state<HTMLDivElement | undefined>();

  // -- Logic --
  function handleViewModeChange(mode: ViewMode) {
    viewMode = mode;
    currentHunkIndex = 0;
  }

  // Reset hunk index when diff changes (e.g. file selection changes)
  $effect(() => {
    // We depend on diffResult to trigger the reset
    if (diffResult) {
        currentHunkIndex = 0;
    }
  });

  function handlePrevHunk() {
    if (currentHunkIndex > 0) {
      currentHunkIndex--;
      scrollToHunk(currentHunkIndex);
    }
  }

  function handleNextHunk() {
    if (currentHunkIndex < hunks.length - 1) {
      currentHunkIndex++;
      scrollToHunk(currentHunkIndex);
    }
  }

  async function scrollToHunk(index: number) {
    await tick();
    const scope = containerEl ?? document;
    const el = scope.querySelector(`[data-hunk-id="hunk-${index}"]`);
    el?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  // Exposed props for the header snippet
  let toolbarProps = $derived({
    viewMode,
    onViewModeChange: handleViewModeChange,
    currentHunkIndex,
    totalHunks: hunks.length,
    onPrevHunk: handlePrevHunk,
    onNextHunk: handleNextHunk,
    selectedEncoding,
    onEncodingChange
  });
</script>

<div class="flex flex-col h-full bg-[#0d1117] overflow-hidden" bind:this={containerEl}>
  <!-- Header / Toolbar Area -->
  <div class="shrink-0">
    {#if header}
        {@render header(toolbarProps)}
    {:else}
        <!-- Default Toolbar Layout if no custom header provided -->
        <div class="flex items-center justify-end px-2 py-1 bg-[#161b22] border-b border-[#30363d]">
            <DiffToolbar 
                {viewMode}
                onViewModeChange={handleViewModeChange}
                {currentHunkIndex}
                totalHunks={hunks.length}
                onPrevHunk={handlePrevHunk}
                onNextHunk={handleNextHunk}
                selectedEncoding={selectedEncoding}
                onEncodingChange={onEncodingChange}
            />
        </div>
    {/if}
  </div>

  <!-- Viewer Area -->
  <div class="flex-1 overflow-auto custom-scrollbar relative">
    {#if loading}
        <div class="absolute inset-0 flex items-center justify-center text-[#8b949e] gap-2">
            <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
            <span class="text-xs">Loading diff...</span>
        </div>
    {:else if diffResult}
        {#if viewMode === 'inline'}
            <InlineDiffViewer
                {diffResult}
                {hunks}
                {canStageLine}
                {onStageLine}
            />
        {:else}
            <!-- Side-By-Side & Hunk View -->
            <SideBySideDiffViewer
                {diffResult}
                isTooLarge={isTooLarge}
                hunks={viewMode === 'hunk' ? hunks : null}
                navigationHunks={hunks}
                {canStageLine}
                {onStageLine}
            />
        {/if}
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

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 10px;
    height: 10px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: #0d1117;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #30363d;
    border: 2px solid #0d1117;
    border-radius: 99px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #484f58;
  }
</style>
