<script lang="ts">
  import type { CommitDiff } from "../../lib/types";
  import InlineDiffViewer from "./InlineDiffViewer.svelte";
  import SideBySideDiffViewer from "./SideBySideDiffViewer.svelte";
  import DiffToolbar from "./DiffToolbar.svelte";
  import type { ViewMode } from "./DiffToolbar.svelte";

  interface Props {
      diff: CommitDiff | null;
      loading?: boolean;
  }
  let { diff, loading = false }: Props = $props();

  let viewMode = $state<ViewMode>("inline");
  let currentHunkIndex = $state(0);

  // Flatten all hunks across all files for cross-file navigation
  let allHunks = $derived.by(() => {
      if (!diff) return [];
      const result: { fileIdx: number; hunkId: string }[] = [];
      for (let fi = 0; fi < diff.files.length; fi++) {
          for (const hunk of diff.files[fi].hunks) {
              result.push({ fileIdx: fi, hunkId: hunk.id });
          }
      }
      return result;
  });

  let totalHunks = $derived(allHunks.length);

  // Reset navigation when diff changes
  $effect(() => {
      if (diff) {
          currentHunkIndex = 0;
      }
  });

  function prevHunk() {
      if (currentHunkIndex > 0) {
          currentHunkIndex--;
          scrollToHunk(currentHunkIndex);
      }
  }

  function nextHunk() {
      if (currentHunkIndex < totalHunks - 1) {
          currentHunkIndex++;
          scrollToHunk(currentHunkIndex);
      }
  }

  function scrollToHunk(index: number) {
      const hunk = allHunks[index];
      if (!hunk) return;
      const el = document.querySelector(`[data-hunk-id="${hunk.hunkId}"]`);
      el?.scrollIntoView({ behavior: "smooth", block: "start" });
  }
</script>

<div class="flex flex-col h-full bg-[#0d1117] overflow-auto custom-scrollbar">
    {#if loading}
        <div class="flex items-center justify-center p-8 text-[#8b949e]">
            <svg class="animate-spin h-6 w-6 mr-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
            Loading diff...
        </div>
    {:else if diff}
        <div class="sticky top-0 z-20 bg-[#161b22] border-b border-[#30363d] px-4 py-2 flex items-center justify-between shrink-0">
             <div class="text-xs font-mono text-[#8b949e]">
                 Commit {diff.commitHash.substring(0, 7)}
             </div>
             <DiffToolbar
                 {viewMode}
                 onViewModeChange={(m) => { viewMode = m; currentHunkIndex = 0; }}
                 {currentHunkIndex}
                 {totalHunks}
                 onPrevHunk={prevHunk}
                 onNextHunk={nextHunk}
             />
        </div>

        <div class="flex-1">
             {#each diff.files as file}
                <div class="border-b border-[#30363d] last:border-b-0">
                    <!-- File Header -->
                    <div class="px-3 py-2 bg-[#161b22] border-b border-[#30363d]/50 flex items-center justify-between sticky top-10 z-10">
                        <div class="flex items-center gap-2">
                             <span class="text-xs font-mono font-semibold text-[#c9d1d9]">{file.path}</span>
                             <span class="text-[10px] px-1.5 rounded border border-[#30363d]
                                {file.status === 'M' ? 'text-[#e3b341]' :
                                 file.status === 'A' ? 'text-[#3fb950]' :
                                 file.status === 'D' ? 'text-[#f85149]' : 'text-[#8b949e]'}">
                                 {file.status}
                             </span>
                        </div>
                    </div>

                    <!-- Diff Viewer -->
                    <div>
                        {#if viewMode === 'inline'}
                            <InlineDiffViewer
                                commitHunks={file.hunks}
                                loading={false}
                            />
                        {:else}
                            <SideBySideDiffViewer
                                commitHunks={file.hunks}
                                loading={false}
                                diffResult={null}
                                autoHeight={true}
                            />
                        {/if}
                    </div>
                </div>
             {/each}
        </div>
    {:else}
         <div class="flex-1 flex items-center justify-center text-[#8b949e] text-sm italic">
             Select a commit to view changes
         </div>
    {/if}
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
