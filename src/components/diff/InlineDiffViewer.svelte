<script lang="ts">
  import {
    toInlineView,
    type DiffHunk,
    type InlineDiffLine,
    type DiffStageLineTarget,
    mapBackendHunksToInline,
    escapeHtml,
  } from "../../lib/diff";
  import type { DiffViewerBaseProps } from "./diff-viewer-types";
  import { toast } from "../../lib/toast.svelte";

  type LineContextMenuState = {
    visible: boolean;
    x: number;
    y: number;
    copyText: string;
    stageTarget: DiffStageLineTarget | null;
  };

  const CONTEXT_MENU_WIDTH = 190;
  const CONTEXT_MENU_ITEM_HEIGHT = 32;
  const CONTEXT_MENU_PADDING_Y = 4;

  interface Props extends DiffViewerBaseProps {
    hunks?: DiffHunk[];
  }
  let {
    diffResult,
    hunks = [],
    commitHunks = [],
    canStageLine = false,
    onStageLine,
    canUnstageLine = false,
    onUnstageLine,
  }: Props = $props();

  let inlineLines = $derived.by<InlineDiffLine[]>(() => {
    if (commitHunks.length > 0) {
      return mapBackendHunksToInline(commitHunks);
    }
    if (diffResult) {
      return toInlineView(diffResult);
    }
    return [];
  });

  let lineContextMenu = $state<LineContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    copyText: "",
    stageTarget: null,
  });

  let hunkFirstLineMap = $derived.by<Map<string | number, string>>(() => {
    const map = new Map<string | number, string>();

    if (commitHunks.length > 0) {
      let currentLineIdx = 0;
      for (const hunk of commitHunks) {
        map.set(currentLineIdx, hunk.id);
        currentLineIdx += 1 + hunk.lines.length;
      }
      return map;
    }

    if (hunks.length === 0) return map;

    let hunkIdx = 0;
    for (let i = 0; i < inlineLines.length && hunkIdx < hunks.length; i++) {
      const hunk = hunks[hunkIdx];
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

  function closeLineContextMenu(): void {
    lineContextMenu = {
      visible: false,
      x: 0,
      y: 0,
      copyText: "",
      stageTarget: null,
    };
  }

  function getContextMenuHeight(): number {
    const actionCount = 1 + (canStageLine ? 1 : 0) + (canUnstageLine ? 1 : 0);
    return actionCount * CONTEXT_MENU_ITEM_HEIGHT + CONTEXT_MENU_PADDING_Y * 2;
  }

  function getContextMenuPosition(clientX: number, clientY: number): { x: number; y: number } {
    const menuHeight = getContextMenuHeight();
    const maxX = Math.max(8, window.innerWidth - CONTEXT_MENU_WIDTH - 8);
    const maxY = Math.max(8, window.innerHeight - menuHeight - 8);
    return {
      x: Math.min(Math.max(8, clientX), maxX),
      y: Math.min(Math.max(8, clientY), maxY),
    };
  }

  function buildStageTarget(line: InlineDiffLine): DiffStageLineTarget | null {
    if (line.type !== "added" && line.type !== "removed") return null;

    if (diffResult) {
      const left = diffResult.left[line.sourceIndex];
      const right = diffResult.right[line.sourceIndex];
      if (
        left?.type === "modified" &&
        right?.type === "modified" &&
        left.lineNumber !== null &&
        right.lineNumber !== null
      ) {
        return {
          oldLineNumber: left.lineNumber,
          newLineNumber: right.lineNumber,
        };
      }
    }

    if (line.type === "removed" && line.oldLineNumber !== null) {
      return {
        oldLineNumber: line.oldLineNumber,
        newLineNumber: null,
      };
    }

    if (line.type === "added" && line.newLineNumber !== null) {
      return {
        oldLineNumber: null,
        newLineNumber: line.newLineNumber,
      };
    }

    return null;
  }

  function handleLineContextMenu(event: MouseEvent, line: InlineDiffLine): void {
    event.preventDefault();
    event.stopPropagation();

    const pos = getContextMenuPosition(event.clientX, event.clientY);
    lineContextMenu = {
      visible: true,
      x: pos.x,
      y: pos.y,
      copyText: line.content,
      stageTarget: buildStageTarget(line),
    };
  }

  function handleWindowMouseDown(event: MouseEvent): void {
    if (!lineContextMenu.visible) return;
    const target = event.target as Element | null;
    if (target?.closest(".diff-line-context-menu")) return;
    closeLineContextMenu();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (!lineContextMenu.visible) return;
    if (event.key === "Escape") {
      event.preventDefault();
      closeLineContextMenu();
    }
  }

  async function handleCopyLine(): Promise<void> {
    const text = lineContextMenu.copyText;
    closeLineContextMenu();
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied line");
    } catch (e) {
      console.error("Copy line failed", e);
      toast.error("Copy line failed");
    }
  }

  async function handleStageThisLine(): Promise<void> {
    if (!canStageLine || !onStageLine || !lineContextMenu.stageTarget) return;
    const target = lineContextMenu.stageTarget;
    closeLineContextMenu();

    try {
      await onStageLine(target);
    } catch (e) {
      console.error("Stage line failed", e);
    }
  }

  async function handleUnstageThisLine(): Promise<void> {
    if (!canUnstageLine || !onUnstageLine || !lineContextMenu.stageTarget) return;
    const target = lineContextMenu.stageTarget;
    closeLineContextMenu();

    try {
      await onUnstageLine(target);
    } catch (e) {
      console.error("Unstage line failed", e);
    }
  }

  function isLineActionable(line: InlineDiffLine): boolean {
    return buildStageTarget(line) !== null;
  }

  async function handleLineClick(line: InlineDiffLine): Promise<void> {
    if (!canUnstageLine || !onUnstageLine) return;
    const target = buildStageTarget(line);
    if (!target) return;

    try {
      await onUnstageLine(target);
    } catch (e) {
      console.error("Unstage line failed", e);
    }
  }
</script>

<svelte:window onmousedown={handleWindowMouseDown} onkeydown={handleWindowKeydown} />

<div class="flex-1 overflow-hidden bg-[#0d1117] h-full relative flex flex-col">
  <div
    class="flex shrink-0 border-b border-[#30363d] text-[10px] uppercase tracking-wider text-[#8b949e] font-semibold"
  >
    <div class="px-3 py-1 bg-[#161b22]">Unified Diff</div>
  </div>

  <div class="flex-1 overflow-auto custom-scrollbar">
    <table class="w-full text-xs font-mono border-collapse">
      <tbody>
        {#each inlineLines as line, i}
          <tr
            class={`${getRowClass(line)} ${canUnstageLine && isLineActionable(line) ? "cursor-pointer" : ""}`}
            data-hunk-id={hunkFirstLineMap.get(i) ?? undefined}
            onclick={() => void handleLineClick(line)}
            oncontextmenu={(event) => handleLineContextMenu(event, line)}
          >
            <td
              class="w-10 text-right pr-1 select-none text-[#484f58] border-r border-[#30363d]/50 align-top"
            >
              {line.oldLineNumber ?? ""}
            </td>
            <td
              class="w-10 text-right pr-1 select-none text-[#484f58] border-r border-[#30363d]/50 align-top"
            >
              {line.newLineNumber ?? ""}
            </td>
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
            <td class="pl-2 whitespace-pre align-top"
              >{@html escapeHtml(line.content)}</td
            >
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

{#if lineContextMenu.visible}
  <div
    class="diff-line-context-menu fixed z-[130] min-w-[180px] rounded-md border border-[#30363d] bg-[#161b22] shadow-2xl overflow-hidden"
    style={`left: ${lineContextMenu.x}px; top: ${lineContextMenu.y}px;`}
    role="menu"
  >
    <button
      type="button"
      class="w-full text-left px-3 py-2 text-xs text-[#c9d1d9] hover:bg-[#21262d] hover:text-white transition-colors"
      onclick={() => void handleCopyLine()}
      role="menuitem"
    >
      Copy this line
    </button>
    {#if canStageLine}
      <button
        type="button"
        class="w-full text-left px-3 py-2 text-xs transition-colors disabled:cursor-not-allowed disabled:text-[#6e7681] disabled:bg-[#161b22] {lineContextMenu.stageTarget ? 'text-[#58a6ff] hover:bg-[#1f2f45] hover:text-[#79c0ff]' : ''}"
        onclick={() => void handleStageThisLine()}
        disabled={!lineContextMenu.stageTarget}
        role="menuitem"
      >
        Stage this line
      </button>
    {/if}
    {#if canUnstageLine}
      <button
        type="button"
        class="w-full text-left px-3 py-2 text-xs transition-colors disabled:cursor-not-allowed disabled:text-[#6e7681] disabled:bg-[#161b22] {lineContextMenu.stageTarget ? 'text-[#f0883e] hover:bg-[#3b2b1f] hover:text-[#ffab70]' : ''}"
        onclick={() => void handleUnstageThisLine()}
        disabled={!lineContextMenu.stageTarget}
        role="menuitem"
      >
        Unstage this line
      </button>
    {/if}
  </div>
{/if}

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
