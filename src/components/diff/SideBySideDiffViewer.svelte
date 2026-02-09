<script lang="ts">
  import { type DiffLine, type DiffResult, type DiffHunk, mapBackendHunksToSideBySide, escapeHtml } from "../../lib/diff";
  import type { DiffHunk as BackendDiffHunk } from "../../lib/types";
  import ResizablePanes from "../resize/ResizablePanes.svelte";

  interface Props {
    diffResult: DiffResult | null;
    isTooLarge?: boolean;
    hunks?: DiffHunk[] | null; // When non-null, render hunk view instead of full diff
    commitHunks?: BackendDiffHunk[]; // New backend hunks
    autoHeight?: boolean; // New prop for global viewer
    navigationHunks?: DiffHunk[]; // Optional hunks for placing data-hunk-id markers in full side-by-side view
  }
  let { diffResult, isTooLarge = false, hunks = null, commitHunks = [], autoHeight = false, navigationHunks }: Props = $props();

  let effectiveHunks = $derived.by<DiffHunk[] | null>(() => {
      if (commitHunks.length > 0) {
          return mapBackendHunksToSideBySide(commitHunks);
      }
      return hunks;
  });

  // Lookup map: diffResult line index → hunk ID for placing data-hunk-id in full side-by-side view
  let hunkStartMap = $derived.by<Map<number, string>>(() => {
      const map = new Map<number, string>();
      if (!navigationHunks) return map;
      for (const hunk of navigationHunks) {
          map.set(hunk.startIndex, hunk.id);
      }
      return map;
  });

  // ── Synchronized scrolling ──────────────────────────────────────
  let leftPanel: HTMLDivElement | undefined = $state();
  let rightPanel: HTMLDivElement | undefined = $state();
  let syncing = false;

  function handleScroll(source: "left" | "right") {
    if (syncing) return;
    syncing = true;
    const from = source === "left" ? leftPanel : rightPanel;
    const to = source === "left" ? rightPanel : leftPanel;
    if (from && to) {
      to.scrollTop = from.scrollTop;
      to.scrollLeft = from.scrollLeft;
    }
    requestAnimationFrame(() => {
      syncing = false;
    });
  }

  // ── Row styling by line type and panel side ─────────────────────
  function getRowClass(line: DiffLine, side: "left" | "right"): string {
    switch (line.type) {
      case "equal":
        return "text-[#c9d1d9]";
      case "removed":
        return side === "left"
          ? "bg-[#da3633]/15 text-[#f85149]"
          : "bg-[#161b22]";
      case "added":
        return side === "right"
          ? "bg-[#2ea043]/15 text-[#3fb950]"
          : "bg-[#161b22]";
      case "modified":
        return side === "left"
          ? "bg-[#da3633]/10 text-[#f0883e]"
          : "bg-[#2ea043]/10 text-[#79c0ff]";
      default:
        return "text-[#c9d1d9]";
    }
  }

</script>

<div class="flex-1 overflow-hidden bg-[#0d1117] relative flex flex-col" class:h-full={!autoHeight} class:h-auto={autoHeight} class:overflow-visible={autoHeight}>
  {#if (!diffResult && !effectiveHunks && !isTooLarge) || (effectiveHunks && effectiveHunks.length === 0 && !diffResult)}
    <div
      class="flex items-center justify-center p-8 text-[#8b949e] text-sm italic flex-1"
    >
      No diff content
    </div>
  {:else if isTooLarge && !effectiveHunks}
    <div
      class="flex items-center justify-center p-8 text-[#8b949e] text-sm italic flex-1"
    >
      File too large for side-by-side diff view
    </div>
  {:else if diffResult || effectiveHunks}
    <!-- Panel headers -->
    <div class="flex shrink-0 border-b border-[#30363d] text-[10px] uppercase tracking-wider text-[#8b949e] font-semibold">
      <div class="flex-1 px-3 py-1 bg-[#161b22] border-r border-[#30363d]">
        Base (HEAD)
      </div>
      <div class="flex-1 px-3 py-1 bg-[#161b22]">Modified</div>
    </div>
    {#if effectiveHunks && effectiveHunks.length > 0}
      <!-- ── HUNK VIEW ─────────────────────────────────────────── -->
      <ResizablePanes initialLeftPercent={50} minLeftPercent={25} maxLeftPercent={75}>
        {#snippet leftContent()}
          <!-- Left panel (base) -->
          <div
            class="h-full overflow-auto custom-scrollbar border-r border-[#30363d]"
            bind:this={leftPanel}
            class:overflow-visible={autoHeight}
            onscroll={() => handleScroll("left")}
          >
            <table class="w-full text-xs font-mono border-collapse">
              <tbody>
                {#each effectiveHunks as hunk, hunkIdx}
                  {#if hunkIdx > 0}
                    <tr>
                      <td colspan="2" class="h-6 bg-[#161b22] border-y border-[#30363d]/50">
                        <div class="flex items-center justify-center">
                          <div class="flex-1 border-t border-dashed border-[#30363d]"></div>
                          <span class="px-2 text-[9px] text-[#484f58] select-none">&ctdot;</span>
                          <div class="flex-1 border-t border-dashed border-[#30363d]"></div>
                        </div>
                      </td>
                    </tr>
                  {/if}
                  {#each hunk.lines as pair, lineIdx}
                    <tr
                      class={getRowClass(pair.left, "left")}
                      data-hunk-id={lineIdx === 0 ? hunk.id : undefined}
                    >
                      <td class="w-10 text-right pr-2 select-none text-[#484f58] border-r border-[#30363d]/50 align-top">
                        {pair.left.lineNumber ?? ""}
                      </td>
                      <td class="pl-2 whitespace-pre align-top">{@html escapeHtml(pair.left.content)}</td>
                    </tr>
                  {/each}
                {/each}
              </tbody>
            </table>
          </div>
        {/snippet}

        {#snippet rightContent()}
          <!-- Right panel (modified) -->
          <div
            class="h-full overflow-auto custom-scrollbar"
            bind:this={rightPanel}
            class:overflow-visible={autoHeight}
            onscroll={() => handleScroll("right")}
          >
            <table class="w-full text-xs font-mono border-collapse">
              <tbody>
                {#each effectiveHunks as hunk, hunkIdx}
                  {#if hunkIdx > 0}
                    <tr>
                      <td colspan="2" class="h-6 bg-[#161b22] border-y border-[#30363d]/50">
                        <div class="flex items-center justify-center">
                          <div class="flex-1 border-t border-dashed border-[#30363d]"></div>
                          <span class="px-2 text-[9px] text-[#484f58] select-none">&ctdot;</span>
                          <div class="flex-1 border-t border-dashed border-[#30363d]"></div>
                        </div>
                      </td>
                    </tr>
                  {/if}
                  {#each hunk.lines as pair, lineIdx}
                    <tr
                      class={getRowClass(pair.right, "right")}
                      data-hunk-id={lineIdx === 0 ? hunk.id : undefined}
                    >
                      <td class="w-10 text-right pr-2 select-none text-[#484f58] border-r border-[#30363d]/50 align-top">
                        {pair.right.lineNumber ?? ""}
                      </td>
                      <td class="pl-2 whitespace-pre align-top">{@html escapeHtml(pair.right.content)}</td>
                    </tr>
                  {/each}
                {/each}
              </tbody>
            </table>
          </div>
        {/snippet}
      </ResizablePanes>
    {:else if diffResult}
      <!-- ── FULL SIDE-BY-SIDE VIEW ────────────────────────────── -->
      <ResizablePanes initialLeftPercent={50} minLeftPercent={25} maxLeftPercent={75}>
        {#snippet leftContent()}
          <!-- Left panel (base version) -->
          <div
            class="h-full overflow-auto custom-scrollbar border-r border-[#30363d]"
            bind:this={leftPanel}
            onscroll={() => handleScroll("left")}
          >
            <table class="w-full text-xs font-mono border-collapse">
              <tbody>
                {#each diffResult.left as line, i}
                  <tr class={getRowClass(line, "left")} data-hunk-id={hunkStartMap.get(i) ?? undefined}>
                    <td
                      class="w-10 text-right pr-2 select-none text-[#484f58] border-r border-[#30363d]/50 align-top"
                    >
                      {line.lineNumber ?? ""}
                    </td>
                    <td class="pl-2 whitespace-pre align-top"
                      >{@html escapeHtml(line.content)}</td
                    >
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/snippet}

        {#snippet rightContent()}
          <!-- Right panel (modified version) -->
          <div
            class="h-full overflow-auto custom-scrollbar"
            bind:this={rightPanel}
            onscroll={() => handleScroll("right")}
          >
            <table class="w-full text-xs font-mono border-collapse">
              <tbody>
                {#each diffResult.right as line, i}
                  <tr class={getRowClass(line, "right")} data-hunk-id={hunkStartMap.get(i) ?? undefined}>
                    <td
                      class="w-10 text-right pr-2 select-none text-[#484f58] border-r border-[#30363d]/50 align-top"
                    >
                      {line.lineNumber ?? ""}
                    </td>
                    <td class="pl-2 whitespace-pre align-top"
                      >{@html escapeHtml(line.content)}</td
                    >
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/snippet}
      </ResizablePanes>
    {/if}
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
