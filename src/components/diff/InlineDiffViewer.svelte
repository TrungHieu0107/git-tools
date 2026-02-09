<script lang="ts">
  import {
    toInlineView,
    type DiffResult,
    type DiffHunk,
    type InlineDiffLine,
    mapBackendHunksToInline,
    escapeHtml,
  } from "../../lib/diff";
  import type { DiffHunk as BackendDiffHunk } from "../../lib/types";

  interface Props {
    diffResult?: DiffResult | null;
    hunks?: DiffHunk[];
    commitHunks?: BackendDiffHunk[];
  }
  let { diffResult, hunks = [], commitHunks = [] }: Props = $props();

  let inlineLines = $derived.by<InlineDiffLine[]>(() => {
    if (commitHunks.length > 0) {
        return mapBackendHunksToInline(commitHunks); // Use mapper from diff.ts
    }
    if (diffResult) {
      return toInlineView(diffResult);
    }
    return [];
  });

  // Build a set of inline-line indices that are the first line of each hunk,
  // so we can place data-hunk-id attributes for scrollIntoView targeting.
  // We map from DiffResult sourceIndex ranges (hunk.startIndex..endIndex)
  // to the first inline line whose sourceIndex falls in that range.
  let hunkFirstLineMap = $derived.by<Map<string | number, string>>(() => {
    const map = new Map<string | number, string>(); // inline line index → hunk id
    
    // Logic for backend hunks (simpler, just count lines)
    if (commitHunks.length > 0) {
        let currentLineIdx = 0;
        for (const hunk of commitHunks) {
            map.set(currentLineIdx, hunk.id);
            // header line + lines
            currentLineIdx += 1 + hunk.lines.length;
        }
        return map;
    }

    // Logic for old client-side hunks
    if (hunks.length === 0) return map;

    let hunkIdx = 0;
    for (let i = 0; i < inlineLines.length && hunkIdx < hunks.length; i++) {
      const hunk = hunks[hunkIdx];
      // Check if this line belongs to the hunk
      if (
        inlineLines[i].sourceIndex >= hunk.startIndex &&
        inlineLines[i].sourceIndex < hunk.endIndex
      ) {
        map.set(i, hunk.id);
        hunkIdx++;
      }
    }
    return map;
  });

  export function scrollToHunk(index: number) {
     if (commitHunks.length > 0) {
         if (commitHunks[index]) {
             scrollToId(commitHunks[index].id);
         }
         return;
     }
     
     if (hunks && hunks[index]) {
         scrollToId(hunks[index].id);
     }
  }

  function scrollToId(id: string) {
      const el = document.querySelector(`[data-hunk-id="${id}"]`);
      el?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  function getRowClass(line: InlineDiffLine): string {
    switch (line.type) {
      case "equal":
        return "text-[#c9d1d9]";
      case "removed":
        return "bg-[#da3633]/15 text-[#f85149]";
      case "added":
        return "bg-[#2ea043]/15 text-[#3fb950]";
      default:
        return "text-[#c9d1d9]";
    }
  }

  function getGutterPrefix(line: InlineDiffLine): string {
    switch (line.type) {
      case "removed":
        return "-";
      case "added":
        return "+";
      default:
        return " ";
    }
  }

</script>

<div class="flex-1 overflow-hidden bg-[#0d1117] h-full relative flex flex-col">
  <div
    class="flex shrink-0 border-b border-[#30363d] text-[10px] uppercase tracking-wider text-[#8b949e] font-semibold"
  >
    <div class="px-3 py-1 bg-[#161b22]">Unified Diff</div>
  </div>

  <!-- Inline diff content -->
  <div class="flex-1 overflow-auto custom-scrollbar">
    <table class="w-full text-xs font-mono border-collapse">
      <tbody>
        {#each inlineLines as line, i}
          <tr
            class={getRowClass(line)}
            data-hunk-id={hunkFirstLineMap.get(i) ?? undefined}
          >
            <!-- Old line number -->
            <td
              class="w-10 text-right pr-1 select-none text-[#484f58] border-r border-[#30363d]/50 align-top"
            >
              {line.oldLineNumber ?? ""}
            </td>
            <!-- New line number -->
            <td
              class="w-10 text-right pr-1 select-none text-[#484f58] border-r border-[#30363d]/50 align-top"
            >
              {line.newLineNumber ?? ""}
            </td>
            <!-- +/- prefix gutter -->
            <td
              class="w-4 text-center select-none align-top {line.type ===
              'removed'
                ? 'text-[#f85149]'
                : line.type === 'added'
                  ? 'text-[#3fb950]'
                  : 'text-[#484f58]'}"
            >
              {getGutterPrefix(line)}
            </td>
            <!-- Content -->
            <td class="pl-2 whitespace-pre align-top"
              >{@html escapeHtml(line.content)}</td
            >
          </tr>
        {/each}
      </tbody>
    </table>
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
