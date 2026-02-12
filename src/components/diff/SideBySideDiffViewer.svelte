<script lang="ts">
  import {
    type DiffLine,
    type DiffResult,
    type DiffHunk,
    type DiffStageLineTarget,
    mapBackendHunksToSideBySide,
    escapeHtml,
  } from "../../lib/diff";
  import type { DiffHunk as BackendDiffHunk } from "../../lib/types";
  import { toast } from "../../lib/toast.svelte";
  import ResizablePanes from "../resize/ResizablePanes.svelte";

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

  interface Props {
    diffResult: DiffResult | null;
    isTooLarge?: boolean;
    hunks?: DiffHunk[] | null;
    commitHunks?: BackendDiffHunk[];
    autoHeight?: boolean;
    navigationHunks?: DiffHunk[];
    canStageLine?: boolean;
    onStageLine?: (line: DiffStageLineTarget) => void | Promise<void>;
    canUnstageLine?: boolean;
    onUnstageLine?: (line: DiffStageLineTarget) => void | Promise<void>;
  }

  let {
    diffResult,
    isTooLarge = false,
    hunks = null,
    commitHunks = [],
    autoHeight = false,
    navigationHunks,
    canStageLine = false,
    onStageLine,
    canUnstageLine = false,
    onUnstageLine,
  }: Props = $props();

  let effectiveHunks = $derived.by<DiffHunk[] | null>(() => {
    if (commitHunks.length > 0) {
      return mapBackendHunksToSideBySide(commitHunks);
    }
    return hunks;
  });

  let hunkStartMap = $derived.by<Map<number, string>>(() => {
    const map = new Map<number, string>();
    if (!navigationHunks) return map;
    for (const hunk of navigationHunks) {
      map.set(hunk.startIndex, hunk.id);
    }
    return map;
  });

  let leftPanel: HTMLDivElement | undefined = $state();
  let rightPanel: HTMLDivElement | undefined = $state();
  let syncing = false;

  let lineContextMenu = $state<LineContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    copyText: "",
    stageTarget: null,
  });

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

  function getRowClass(line: DiffLine, side: "left" | "right"): string {
    switch (line.type) {
      case "equal":
        return "text-[#c9d1d9]";
      case "removed":
        return side === "left" ? "bg-[#da3633]/15 text-[#f85149]" : "bg-[#161b22]";
      case "added":
        return side === "right" ? "bg-[#2ea043]/15 text-[#3fb950]" : "bg-[#161b22]";
      case "modified":
        return side === "left" ? "bg-[#da3633]/10 text-[#f0883e]" : "bg-[#2ea043]/10 text-[#79c0ff]";
      default:
        return "text-[#c9d1d9]";
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

  function buildStageTarget(
    line: DiffLine,
    counterpart: DiffLine | null,
    side: "left" | "right"
  ): DiffStageLineTarget | null {
    if (line.type === "modified" && counterpart?.type === "modified") {
      if (line.lineNumber === null || counterpart.lineNumber === null) return null;
      if (side === "left") {
        return { oldLineNumber: line.lineNumber, newLineNumber: counterpart.lineNumber };
      }
      return { oldLineNumber: counterpart.lineNumber, newLineNumber: line.lineNumber };
    }

    if (side === "left" && line.type === "removed" && line.lineNumber !== null) {
      return { oldLineNumber: line.lineNumber, newLineNumber: null };
    }

    if (side === "right" && line.type === "added" && line.lineNumber !== null) {
      return { oldLineNumber: null, newLineNumber: line.lineNumber };
    }

    return null;
  }

  function handleLineContextMenu(
    event: MouseEvent,
    line: DiffLine,
    counterpart: DiffLine | null,
    side: "left" | "right"
  ): void {
    event.preventDefault();
    event.stopPropagation();

    const pos = getContextMenuPosition(event.clientX, event.clientY);
    lineContextMenu = {
      visible: true,
      x: pos.x,
      y: pos.y,
      copyText: line.content,
      stageTarget: buildStageTarget(line, counterpart, side),
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

  function isLineActionable(
    line: DiffLine,
    counterpart: DiffLine | null,
    side: "left" | "right"
  ): boolean {
    return buildStageTarget(line, counterpart, side) !== null;
  }

  async function handleLineClick(
    line: DiffLine,
    counterpart: DiffLine | null,
    side: "left" | "right"
  ): Promise<void> {
    if (!canUnstageLine || !onUnstageLine) return;
    const target = buildStageTarget(line, counterpart, side);
    if (!target) return;

    try {
      await onUnstageLine(target);
    } catch (e) {
      console.error("Unstage line failed", e);
    }
  }
</script>

<svelte:window onmousedown={handleWindowMouseDown} onkeydown={handleWindowKeydown} />

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
    <div class="flex shrink-0 border-b border-[#30363d] text-[10px] uppercase tracking-wider text-[#8b949e] font-semibold">
      <div class="flex-1 px-3 py-1 bg-[#161b22] border-r border-[#30363d]">
        Base (HEAD)
      </div>
      <div class="flex-1 px-3 py-1 bg-[#161b22]">Modified</div>
    </div>
    {#if effectiveHunks && effectiveHunks.length > 0}
      <ResizablePanes initialLeftPercent={50} minLeftPercent={25} maxLeftPercent={75}>
        {#snippet leftContent()}
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
                      class={`${getRowClass(pair.left, "left")} ${canUnstageLine && isLineActionable(pair.left, pair.right, "left") ? "cursor-pointer" : ""}`}
                      data-hunk-id={lineIdx === 0 ? hunk.id : undefined}
                      onclick={() => void handleLineClick(pair.left, pair.right, "left")}
                      oncontextmenu={(event) => handleLineContextMenu(event, pair.left, pair.right, "left")}
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
                      class={`${getRowClass(pair.right, "right")} ${canUnstageLine && isLineActionable(pair.right, pair.left, "right") ? "cursor-pointer" : ""}`}
                      data-hunk-id={lineIdx === 0 ? hunk.id : undefined}
                      onclick={() => void handleLineClick(pair.right, pair.left, "right")}
                      oncontextmenu={(event) => handleLineContextMenu(event, pair.right, pair.left, "right")}
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
      <ResizablePanes initialLeftPercent={50} minLeftPercent={25} maxLeftPercent={75}>
        {#snippet leftContent()}
          <div
            class="h-full overflow-auto custom-scrollbar border-r border-[#30363d]"
            bind:this={leftPanel}
            onscroll={() => handleScroll("left")}
          >
            <table class="w-full text-xs font-mono border-collapse">
              <tbody>
                {#each diffResult.left as line, i}
                  <tr
                    class={`${getRowClass(line, "left")} ${canUnstageLine && isLineActionable(line, diffResult.right[i] ?? null, "left") ? "cursor-pointer" : ""}`}
                    data-hunk-id={hunkStartMap.get(i) ?? undefined}
                    onclick={() => void handleLineClick(line, diffResult.right[i] ?? null, "left")}
                    oncontextmenu={(event) => handleLineContextMenu(event, line, diffResult.right[i] ?? null, "left")}
                  >
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
          <div
            class="h-full overflow-auto custom-scrollbar"
            bind:this={rightPanel}
            onscroll={() => handleScroll("right")}
          >
            <table class="w-full text-xs font-mono border-collapse">
              <tbody>
                {#each diffResult.right as line, i}
                  <tr
                    class={`${getRowClass(line, "right")} ${canUnstageLine && isLineActionable(line, diffResult.left[i] ?? null, "right") ? "cursor-pointer" : ""}`}
                    data-hunk-id={hunkStartMap.get(i) ?? undefined}
                    onclick={() => void handleLineClick(line, diffResult.left[i] ?? null, "right")}
                    oncontextmenu={(event) => handleLineContextMenu(event, line, diffResult.left[i] ?? null, "right")}
                  >
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
