<script lang="ts">
  import { type DiffLine, type DiffResult, type DiffHunk } from "../../lib/diff";

  interface Props {
    diffResult: DiffResult | null;
    loading: boolean;
    isTooLarge?: boolean;
    hunks?: DiffHunk[] | null; // When non-null, render hunk view instead of full diff
  }
  let { diffResult, loading, isTooLarge = false, hunks = null }: Props = $props();

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

  function escapeHtml(unsafe: string): string {
    return unsafe
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }
</script>

<div class="flex-1 overflow-hidden bg-[#0d1117] h-full relative flex flex-col">
  {#if loading}
    <div class="absolute inset-0 flex items-center justify-center bg-[#0d1117]/50 z-10">
      <svg
        class="animate-spin h-6 w-6 text-[#8b949e]"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <path
          d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"
        />
      </svg>
    </div>
  {/if}

  {#if !diffResult && !loading && !isTooLarge}
    <div
      class="flex items-center justify-center p-8 text-[#8b949e] text-sm italic flex-1"
    >
      No diff content
    </div>
  {:else if isTooLarge}
    <div
      class="flex items-center justify-center p-8 text-[#8b949e] text-sm italic flex-1"
    >
      File too large for side-by-side diff view
    </div>
  {:else if diffResult}
    <!-- Panel headers -->
    <div class="flex shrink-0 border-b border-[#30363d] text-[10px] uppercase tracking-wider text-[#8b949e] font-semibold">
      <div class="w-1/2 px-3 py-1 bg-[#161b22] border-r border-[#30363d]">
        Base (HEAD)
      </div>
      <div class="w-1/2 px-3 py-1 bg-[#161b22]">Modified</div>
    </div>

    {#if hunks && hunks.length > 0}
      <!-- ── HUNK VIEW ─────────────────────────────────────────── -->
      <div class="flex flex-1 overflow-hidden min-h-0">
        <!-- Left panel (base) -->
        <div
          class="w-1/2 overflow-auto custom-scrollbar border-r border-[#30363d]"
          bind:this={leftPanel}
          onscroll={() => handleScroll("left")}
        >
          <table class="w-full text-xs font-mono border-collapse">
            <tbody>
              {#each hunks as hunk, hunkIdx}
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

        <!-- Right panel (modified) -->
        <div
          class="w-1/2 overflow-auto custom-scrollbar"
          bind:this={rightPanel}
          onscroll={() => handleScroll("right")}
        >
          <table class="w-full text-xs font-mono border-collapse">
            <tbody>
              {#each hunks as hunk, hunkIdx}
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
      </div>
    {:else}
      <!-- ── FULL SIDE-BY-SIDE VIEW ────────────────────────────── -->
      <div class="flex flex-1 overflow-hidden min-h-0">
        <!-- Left panel (base version) -->
        <div
          class="w-1/2 overflow-auto custom-scrollbar border-r border-[#30363d]"
          bind:this={leftPanel}
          onscroll={() => handleScroll("left")}
        >
          <table class="w-full text-xs font-mono border-collapse">
            <tbody>
              {#each diffResult.left as line}
                <tr class={getRowClass(line, "left")}>
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

        <!-- Right panel (modified version) -->
        <div
          class="w-1/2 overflow-auto custom-scrollbar"
          bind:this={rightPanel}
          onscroll={() => handleScroll("right")}
        >
          <table class="w-full text-xs font-mono border-collapse">
            <tbody>
              {#each diffResult.right as line}
                <tr class={getRowClass(line, "right")}>
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
      </div>
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
