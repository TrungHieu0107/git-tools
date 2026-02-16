<script lang="ts">
  import { GitService, type ConflictFile, type GitOperationState } from "../../lib/GitService";
  import { toast } from "../../lib/toast.svelte";
  import EncodingSelector from "../../lib/components/EncodingSelector.svelte";

  type PlainSegment = {
    id: string;
    kind: "plain";
    text: string;
  };

  type ConflictSegment = {
    id: string;
    kind: "conflict";
    ours: string;
    theirs: string;
    resolved: string;
  };

  type ResolutionSegment = PlainSegment | ConflictSegment;

  type DiffLineRow = {
    lineNo: number;
    text: string;
    highlighted: boolean;
    conflictIndex?: number;
    conflictLineIndex?: number;
    sourceSide?: "ours" | "theirs";
  };

  type StackEntry = { side: "ours" | "theirs"; lineIndex: number };

  interface Props {
    repoPath?: string;
    filePath: string | null;
    operationState?: GitOperationState | null;
    onClose: () => void;
    onResolved?: (filePath: string) => void | Promise<void>;
  }

  let { repoPath, filePath, operationState, onClose, onResolved }: Props = $props();

  let loading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let parseWarning = $state<string | null>(null);
  let conflictFile = $state<ConflictFile | null>(null);
  let segments = $state<ResolutionSegment[]>([]);
  let manualContent = $state("");
  let useManualEditor = $state(false);
  let newlineStyle = $state<"\n" | "\r\n">("\n");
  let activeConflictIndex = $state(0);
  let selectionStacks = $state<Record<number, StackEntry[]>>({});
  let resolvedLineSources = $state<Record<number, ("ours" | "theirs")[]>>({});
  let selectedEncoding = $state<string | undefined>(undefined);
  let loadToken = 0;

  // Panel scroll containers
  let panelOursEl = $state<HTMLDivElement | null>(null);
  let panelTheirsEl = $state<HTMLDivElement | null>(null);
  let panelOutputEl = $state<HTMLDivElement | null>(null);

  const CONFLICT_BLOCK_REGEX =
    /^<<<<<<<[^\n]*\n([\s\S]*?)(?:^\|\|\|\|\|\|\|[^\n]*\n[\s\S]*?)?^=======\n([\s\S]*?)^>>>>>>>[^\n]*(?:\n|$)/gm;

  // Derived header labels
  let oursLabel = $derived.by(() => {
    const commit = operationState?.oursCommit;
    const branch = operationState?.oursBranch;
    if (commit && branch) return `Commit ${commit} on ${branch}`;
    if (commit) return `Commit ${commit}`;
    return "Local (Ours)";
  });

  let theirsLabel = $derived.by(() => {
    const commit = operationState?.theirsCommit;
    const branch = operationState?.theirsBranch;
    if (commit && branch) return `Commit ${commit} on ${branch}`;
    if (commit) return `Commit ${commit}`;
    return "Origin (Theirs)";
  });

  function detectNewlineStyle(content: string): "\n" | "\r\n" {
    return content.includes("\r\n") ? "\r\n" : "\n";
  }

  function normalizeNewlines(content: string): string {
    return content.replace(/\r\n/g, "\n");
  }

  function denormalizeNewlines(content: string): string {
    if (newlineStyle === "\n") return content;
    return content.replace(/\n/g, newlineStyle);
  }

  function buildFallbackConflictContent(conflict: ConflictFile): string {
    const ours = normalizeNewlines(conflict.ours ?? "");
    const theirs = normalizeNewlines(conflict.theirs ?? "");
    return `<<<<<<< LOCAL\n${ours}\n=======\n${theirs}\n>>>>>>> ORIGIN\n`;
  }

  function parseResolutionSegments(content: string): {
    segments: ResolutionSegment[];
    conflictCount: number;
    hasMarkers: boolean;
  } {
    const normalized = normalizeNewlines(content);
    const parsedSegments: ResolutionSegment[] = [];
    let cursor = 0;
    let conflictCounter = 0;
    let match: RegExpExecArray | null;

    CONFLICT_BLOCK_REGEX.lastIndex = 0;
    while ((match = CONFLICT_BLOCK_REGEX.exec(normalized)) !== null) {
      const blockText = match[0] ?? "";
      const ours = match[1] ?? "";
      const theirs = match[2] ?? "";

      if (match.index > cursor) {
        parsedSegments.push({
          id: `plain-${cursor}`,
          kind: "plain",
          text: normalized.slice(cursor, match.index),
        });
      }

      conflictCounter += 1;
      parsedSegments.push({
        id: `conflict-${conflictCounter}`,
        kind: "conflict",
        ours,
        theirs,
        resolved: ours,
      });

      cursor = match.index + blockText.length;
    }

    if (cursor < normalized.length) {
      parsedSegments.push({
        id: `plain-tail-${cursor}`,
        kind: "plain",
        text: normalized.slice(cursor),
      });
    }

    if (parsedSegments.length === 0) {
      parsedSegments.push({
        id: "plain-empty",
        kind: "plain",
        text: normalized,
      });
    }

    const hasMarkers =
      normalized.includes("<<<<<<<") ||
      normalized.includes("=======") ||
      normalized.includes(">>>>>>>");

    return {
      segments: parsedSegments,
      conflictCount: conflictCounter,
      hasMarkers,
    };
  }

  function composeResolvedContent(currentSegments: ResolutionSegment[]): string {
    return currentSegments
      .map((segment) => (segment.kind === "plain" ? segment.text : segment.resolved))
      .join("");
  }

  function countConflicts(currentSegments: ResolutionSegment[]): number {
    return currentSegments.reduce(
      (total, segment) => total + (segment.kind === "conflict" ? 1 : 0),
      0,
    );
  }

  function toDisplayLines(text: string): string[] {
    const normalized = normalizeNewlines(text);
    if (!normalized.length) return [];
    const trimmedTrailing = normalized.endsWith("\n")
      ? normalized.slice(0, normalized.length - 1)
      : normalized;
    if (!trimmedTrailing.length) return [""];
    return trimmedTrailing.split("\n");
  }

  function getConflictLinesBySide(
    side: "ours" | "theirs",
    conflictIndex: number,
  ): string[] {
    const conflict = conflictSegments[conflictIndex];
    if (!conflict) return [];
    return toDisplayLines(side === "ours" ? conflict.ours : conflict.theirs);
  }

  function getStack(conflictIndex: number): StackEntry[] {
    return selectionStacks[conflictIndex] ?? [];
  }

  function isLineSelected(
    side: "ours" | "theirs",
    conflictIndex: number,
    lineIndex: number,
  ): boolean {
    return getStack(conflictIndex).some(
      (e) => e.side === side && e.lineIndex === lineIndex,
    );
  }

  function getSelectionOrder(
    side: "ours" | "theirs",
    conflictIndex: number,
    lineIndex: number,
  ): number | null {
    const idx = getStack(conflictIndex).findIndex(
      (e) => e.side === side && e.lineIndex === lineIndex,
    );
    return idx >= 0 ? idx + 1 : null;
  }

  function isBlockFullySelected(side: "ours" | "theirs", conflictIndex: number): boolean {
    const lines = getConflictLinesBySide(side, conflictIndex);
    if (lines.length === 0) return false;
    const stack = getStack(conflictIndex);
    return lines.every((_, i) => stack.some((e) => e.side === side && e.lineIndex === i));
  }

  function recomputeResolvedFromSelections(conflictIndex: number): void {
    const target = conflictSegments[conflictIndex];
    if (!target) return;

    const stack = getStack(conflictIndex);
    const resolvedLines: string[] = [];
    const sources: ("ours" | "theirs")[] = [];

    // Iterate in click-order (the stack preserves insertion order)
    for (const entry of stack) {
      const lines = getConflictLinesBySide(entry.side, conflictIndex);
      if (entry.lineIndex >= 0 && entry.lineIndex < lines.length) {
        resolvedLines.push(lines[entry.lineIndex]);
        sources.push(entry.side);
      }
    }

    const resolvedText = resolvedLines.length > 0 ? resolvedLines.join("\n") + "\n" : "";
    segments = segments.map((segment) => {
      if (segment.kind !== "conflict" || segment.id !== target.id) return segment;
      return { ...segment, resolved: resolvedText };
    });
    resolvedLineSources = { ...resolvedLineSources, [conflictIndex]: sources };
  }

  function toggleLineAndUpdateOutput(
    side: "ours" | "theirs",
    conflictIndex: number,
    lineIndex: number,
  ): void {
    const stack = getStack(conflictIndex);
    const existingIdx = stack.findIndex(
      (e) => e.side === side && e.lineIndex === lineIndex,
    );
    if (existingIdx >= 0) {
      // Remove (deselect)
      const next = [...stack];
      next.splice(existingIdx, 1);
      selectionStacks = { ...selectionStacks, [conflictIndex]: next };
    } else {
      // Push to end (select)
      selectionStacks = {
        ...selectionStacks,
        [conflictIndex]: [...stack, { side, lineIndex }],
      };
    }
    recomputeResolvedFromSelections(conflictIndex);
  }

  function toggleSideBlock(side: "ours" | "theirs", conflictIndex: number, selected: boolean): void {
    const lines = getConflictLinesBySide(side, conflictIndex);
    const stack = getStack(conflictIndex);
    if (selected) {
      // Append all non-selected lines of this side, in line order
      const toAdd = lines
        .map((_, i) => i)
        .filter((i) => !stack.some((e) => e.side === side && e.lineIndex === i))
        .map((i): StackEntry => ({ side, lineIndex: i }));
      selectionStacks = {
        ...selectionStacks,
        [conflictIndex]: [...stack, ...toAdd],
      };
    } else {
      // Remove all lines of this side from the stack
      selectionStacks = {
        ...selectionStacks,
        [conflictIndex]: stack.filter((e) => e.side !== side),
      };
    }
    recomputeResolvedFromSelections(conflictIndex);
  }

  function resetConflict(conflictIndex: number): void {
    const target = conflictSegments[conflictIndex];
    if (!target) return;
    selectionStacks = { ...selectionStacks, [conflictIndex]: [] };
    segments = segments.map((segment) => {
      if (segment.kind !== "conflict" || segment.id !== target.id) return segment;
      return { ...segment, resolved: segment.ours };
    });
    resolvedLineSources = { ...resolvedLineSources, [conflictIndex]: [] };
  }

  function undoLastSelection(conflictIndex: number): void {
    const stack = getStack(conflictIndex);
    if (stack.length === 0) return;
    selectionStacks = {
      ...selectionStacks,
      [conflictIndex]: stack.slice(0, -1),
    };
    recomputeResolvedFromSelections(conflictIndex);
  }

  function buildFullViewRows(view: "ours" | "theirs" | "output"): DiffLineRow[] {
    const rows: DiffLineRow[] = [];
    let lineNo = 1;
    let conflictIdx = 0;

    for (const segment of segments) {
      if (segment.kind === "plain") {
        const lines = toDisplayLines(segment.text);
        for (const line of lines) {
          rows.push({ lineNo, text: line, highlighted: false });
          lineNo += 1;
        }
        continue;
      }

      const lines = toDisplayLines(
        view === "ours" ? segment.ours : view === "theirs" ? segment.theirs : segment.resolved,
      );
      const highlighted = true; // all conflict lines are highlighted
      const sources = view === "output" ? (resolvedLineSources[conflictIdx] ?? []) : [];

      if (lines.length === 0) {
        rows.push({
          lineNo,
          text: "",
          highlighted,
          conflictIndex: conflictIdx,
          conflictLineIndex: 0,
          sourceSide: view === "output" ? sources[0] : undefined,
        });
        lineNo += 1;
      } else {
        for (let i = 0; i < lines.length; i += 1) {
          rows.push({
            lineNo,
            text: lines[i],
            highlighted,
            conflictIndex: conflictIdx,
            conflictLineIndex: i,
            sourceSide: view === "output" ? sources[i] : undefined,
          });
          lineNo += 1;
        }
      }

      conflictIdx += 1;
    }

    if (rows.length === 0) {
      rows.push({ lineNo: 1, text: "", highlighted: false });
    }

    return rows;
  }

  let conflictCount = $derived.by(() => countConflicts(segments));
  let conflictSegments = $derived.by<ConflictSegment[]>(() =>
    segments.filter((segment): segment is ConflictSegment => segment.kind === "conflict"),
  );
  let fullOursRows = $derived.by(() => buildFullViewRows("ours"));
  let fullTheirsRows = $derived.by(() => buildFullViewRows("theirs"));
  let fullOutputRows = $derived.by(() => buildFullViewRows("output"));

  function clampConflictIndex(index: number): number {
    if (conflictCount <= 0) return 0;
    if (index < 0) return 0;
    if (index >= conflictCount) return conflictCount - 1;
    return index;
  }

  function goToPrevConflict(): void {
    if (conflictCount <= 0) return;
    activeConflictIndex =
      activeConflictIndex <= 0 ? conflictCount - 1 : activeConflictIndex - 1;
  }

  function goToNextConflict(): void {
    if (conflictCount <= 0) return;
    activeConflictIndex =
      activeConflictIndex >= conflictCount - 1 ? 0 : activeConflictIndex + 1;
  }

  function handleEncodingChange(encoding: string): void {
    selectedEncoding = encoding;
    if (filePath) {
      void loadConflict(filePath);
    }
  }

  function closeModal(): void {
    if (saving) return;
    onClose();
  }

  function handleBackdropMouseDown(event: MouseEvent): void {
    if (event.target !== event.currentTarget) return;
    closeModal();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (!filePath || event.key !== "Escape") return;
    event.preventDefault();
    event.stopPropagation();
    closeModal();
  }

  async function loadConflict(targetPath: string) {
    const currentLoadToken = ++loadToken;
    loading = true;
    error = null;
    parseWarning = null;
    useManualEditor = false;
    conflictFile = null;
    segments = [];
    manualContent = "";
    activeConflictIndex = 0;
    selectionStacks = {};
    resolvedLineSources = {};

    try {
      const [conflictResult, modifiedContentResult] = await Promise.allSettled([
        GitService.getConflictFile(targetPath, repoPath, selectedEncoding),
        GitService.getFileModifiedContent(targetPath, false, repoPath, selectedEncoding),
      ]);
      if (currentLoadToken !== loadToken) return;

      if (conflictResult.status === "rejected") {
        throw conflictResult.reason;
      }

      const nextConflictFile = conflictResult.value;
      conflictFile = nextConflictFile;

      const modifiedContent =
        modifiedContentResult.status === "fulfilled" ? modifiedContentResult.value : null;

      // Fallback to construction from stages if modified content is missing or unexpectedly empty,
      // ensuring the user always sees the conflict structure even if the disk read is transiently empty.
      const sourceContent = (modifiedContent && modifiedContent.length > 0)
        ? modifiedContent
        : buildFallbackConflictContent(nextConflictFile);

      newlineStyle = detectNewlineStyle(sourceContent);

      const parsed = parseResolutionSegments(sourceContent);
      
      // Initialize stacks and sources to match the default 'ours' resolution.
      // This ensures that source indicators (A/B badges) and checkmarks appear immediately upon load.
      const initialStacks: Record<number, StackEntry[]> = {};
      const initialSources: Record<number, ("ours" | "theirs")[]> = {};
      let conflictCounter = 0;

      for (const segment of parsed.segments) {
        if (segment.kind === "conflict") {
          const lines = toDisplayLines(segment.ours);
          initialStacks[conflictCounter] = lines.map((_, i) => ({
            side: "ours" as const,
            lineIndex: i,
          }));
          initialSources[conflictCounter] = lines.map(() => "ours" as const);
          conflictCounter += 1;
        }
      }

      // Batch update all state to ensure atomic UI transition and prevent inconsistent intermediate renders.
      segments = parsed.segments;
      selectionStacks = initialStacks;
      resolvedLineSources = initialSources;

      if (modifiedContentResult.status === "rejected" || (modifiedContent !== null && modifiedContent.length === 0)) {
        parseWarning = "Working tree version is unavailable or empty. Showing content derived from conflict stages.";
      }

      if (parsed.conflictCount === 0) {
        useManualEditor = true;
        manualContent = normalizeNewlines(sourceContent);
        parseWarning = parseWarning
          ? parseWarning
          : parsed.hasMarkers
          ? "Could not parse conflict blocks automatically. Edit the resolved content manually."
          : "No conflict markers found in the file. You can still edit and mark it resolved.";
      }
    } catch (e) {
      if (currentLoadToken !== loadToken) return;
      const message = e instanceof Error ? e.message : String(e);
      error = message;
      conflictFile = null;
      segments = [];
      useManualEditor = true;
      manualContent = "";
    } finally {
      if (currentLoadToken !== loadToken) return;
      loading = false;
    }
  }

  async function handleSave(): Promise<void> {
    if (!filePath || saving) return;
    saving = true;
    error = null;

    try {
      const finalContent = useManualEditor
        ? denormalizeNewlines(manualContent)
        : denormalizeNewlines(composeResolvedContent(segments));

      await GitService.writeFile(filePath, finalContent, repoPath, selectedEncoding);
      await GitService.markResolved(filePath, repoPath);

      toast.success(`Resolved conflict: ${filePath}`);
      await onResolved?.(filePath);
      onClose();
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      error = message;
      toast.error(`Resolve failed: ${message}`);
    } finally {
      saving = false;
    }
  }

  $effect(() => {
    const targetPath = filePath;
    if (!targetPath) return;
    void loadConflict(targetPath);
  });

  $effect(() => {
    activeConflictIndex = clampConflictIndex(activeConflictIndex);
  });

  // Scroll sync: scroll to active conflict in all panels
  $effect(() => {
    const idx = activeConflictIndex;
    // tick delay to let DOM update
    requestAnimationFrame(() => {
      for (const panel of [panelOursEl, panelTheirsEl, panelOutputEl]) {
        if (!panel) continue;
        const el = panel.querySelector(`[data-conflict-start="${idx}"]`);
        el?.scrollIntoView({ behavior: "smooth", block: "center" });
      }
    });
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if filePath}
  <div
    class="fixed inset-0 z-[140] bg-black/70 backdrop-blur-sm flex items-center justify-center p-4"
    role="presentation"
    onmousedown={handleBackdropMouseDown}
  >
    <div
      class="w-full max-w-[1520px] h-[min(92vh,980px)] bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl overflow-hidden flex flex-col"
      role="dialog"
      aria-label="Resolve conflict"
    >
      <!-- Modal Header -->
      <div class="shrink-0 px-4 py-3 border-b border-[#30363d] bg-[#0d1117] flex items-center gap-3">
        <div class="min-w-0 flex-1">
          <h3 class="text-sm font-semibold text-white truncate">Resolve Conflict</h3>
          <p class="text-xs text-[#8b949e] truncate" title={filePath}>{filePath}</p>
        </div>
        <EncodingSelector
          {selectedEncoding}
          on:change={(e: CustomEvent<string>) => handleEncodingChange(e.detail)}
        />
        <span class="text-[11px] px-2 py-1 rounded border border-[#30363d] text-[#c9d1d9]">
          {conflictCount} block(s)
        </span>
        <button
          type="button"
          class="text-[#8b949e] hover:text-[#c9d1d9] p-1 rounded hover:bg-[#30363d] transition-colors"
          onclick={closeModal}
          aria-label="Close"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      {#if loading}
        <div class="flex-1 flex items-center justify-center text-sm text-[#8b949e]">
          Loading conflict content...
        </div>
      {:else if error && !conflictFile}
        <div class="flex-1 flex items-center justify-center p-6">
          <div class="max-w-[760px] w-full rounded border border-[#f85149]/45 bg-[#3b1f2c] p-3 text-sm text-[#ff7b72]">
            {error}
          </div>
        </div>
      {:else if useManualEditor}
        <!-- Manual editor fallback -->
        <div class="flex-1 min-h-0 flex flex-col bg-[#0d1117] p-3">
          {#if parseWarning}
            <div class="rounded border border-[#d29922]/45 bg-[#2d230f] p-2 text-xs text-[#e3b341] mb-3">
              {parseWarning}
            </div>
          {/if}
          <textarea
            class="w-full flex-1 bg-[#0f1620] border border-[#30363d] rounded p-3 font-mono text-xs text-[#c9d1d9] focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] resize-none"
            bind:value={manualContent}
          ></textarea>
        </div>
      {:else}
        <!-- GitKraken-style layout -->
        <div class="flex-1 min-h-0 flex flex-col bg-[#0d1117]">
          <!-- Two source panels (top half) -->
          <div class="min-h-0 h-[55%] grid grid-cols-2 divide-x divide-[#30363d]">
            <!-- Panel A (Ours) -->
            <section class="min-h-0 flex flex-col">
              <div class="shrink-0 px-3 h-8 border-b border-[#30363d] bg-[#0f2732] text-[#9ee7ff] text-xs font-semibold flex items-center gap-2">
                <input
                  type="checkbox"
                  class="h-3.5 w-3.5 accent-[#36a9da]"
                  checked={isBlockFullySelected("ours", activeConflictIndex)}
                  onchange={(event) =>
                    toggleSideBlock("ours", activeConflictIndex, (event.currentTarget as HTMLInputElement).checked)}
                  title="Select all local lines for active conflict"
                />
                <span class="inline-flex h-5 w-5 items-center justify-center rounded-full border border-[#1f6f8b] bg-[#123645] text-[10px] font-bold">A</span>
                <span class="truncate">{oursLabel}</span>
              </div>
              <div class="flex-1 min-h-0 overflow-auto font-mono text-[13px] leading-6" bind:this={panelOursEl}>
                {#each fullOursRows as row, rowIdx (`ours-${row.lineNo}`)}
                  {@const isConflict = row.conflictIndex !== undefined}
                  {@const isFirst = isConflict && row.conflictLineIndex === 0}
                  {@const selected = isConflict && row.conflictLineIndex !== undefined && isLineSelected("ours", row.conflictIndex!, row.conflictLineIndex!)}
                  <div
                    class="grid grid-cols-[24px_48px_1fr] {isConflict ? 'bg-[#14485c]/60 border-l-2 border-l-[#36a9da] cursor-pointer hover:bg-[#14485c]/90' : ''}"
                    data-conflict-start={isFirst ? row.conflictIndex : undefined}
                    onclick={() => {
                      if (isConflict && row.conflictIndex !== undefined && row.conflictLineIndex !== undefined) {
                        toggleLineAndUpdateOutput("ours", row.conflictIndex, row.conflictLineIndex);
                      }
                    }}
                    role={isConflict ? "button" : undefined}
                    tabindex={isConflict ? 0 : undefined}
                  >
                    <div class="flex items-center justify-center">
                      {#if selected}
                        <svg class="w-4 h-4 text-green-400" viewBox="0 0 20 20" fill="none">
                          <circle cx="10" cy="10" r="8" fill="currentColor" opacity="0.25"/>
                          <path d="M6.5 10.5l2.5 2.5 4.5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                      {/if}
                    </div>
                    <div class="px-1 text-right text-[#7d8590] select-none text-xs">{row.lineNo}</div>
                    <div class="pr-3 pl-2 text-[#d5f5ff] whitespace-pre overflow-hidden text-ellipsis">{row.text || " "}</div>
                  </div>
                {/each}
              </div>
            </section>

            <!-- Panel B (Theirs) -->
            <section class="min-h-0 flex flex-col">
              <div class="shrink-0 px-3 h-8 border-b border-[#30363d] bg-[#302a14] text-[#ffe38b] text-xs font-semibold flex items-center gap-2">
                <input
                  type="checkbox"
                  class="h-3.5 w-3.5 accent-[#f3cc47]"
                  checked={isBlockFullySelected("theirs", activeConflictIndex)}
                  onchange={(event) =>
                    toggleSideBlock("theirs", activeConflictIndex, (event.currentTarget as HTMLInputElement).checked)}
                  title="Select all origin lines for active conflict"
                />
                <span class="inline-flex h-5 w-5 items-center justify-center rounded-full border border-[#c8a833] bg-[#4b3f1b] text-[10px] font-bold">B</span>
                <span class="truncate">{theirsLabel}</span>
              </div>
              <div class="flex-1 min-h-0 overflow-auto font-mono text-[13px] leading-6" bind:this={panelTheirsEl}>
                {#each fullTheirsRows as row, rowIdx (`theirs-${row.lineNo}`)}
                  {@const isConflict = row.conflictIndex !== undefined}
                  {@const isFirst = isConflict && row.conflictLineIndex === 0}
                  {@const selected = isConflict && row.conflictLineIndex !== undefined && isLineSelected("theirs", row.conflictIndex!, row.conflictLineIndex!)}
                  <div
                    class="grid grid-cols-[24px_48px_1fr] {isConflict ? 'bg-[#584e22]/60 border-l-2 border-l-[#f3cc47] cursor-pointer hover:bg-[#584e22]/90' : ''}"
                    data-conflict-start={isFirst ? row.conflictIndex : undefined}
                    onclick={() => {
                      if (isConflict && row.conflictIndex !== undefined && row.conflictLineIndex !== undefined) {
                        toggleLineAndUpdateOutput("theirs", row.conflictIndex, row.conflictLineIndex);
                      }
                    }}
                    role={isConflict ? "button" : undefined}
                    tabindex={isConflict ? 0 : undefined}
                  >
                    <div class="flex items-center justify-center">
                      {#if selected}
                        <svg class="w-4 h-4 text-green-400" viewBox="0 0 20 20" fill="none">
                          <circle cx="10" cy="10" r="8" fill="currentColor" opacity="0.25"/>
                          <path d="M6.5 10.5l2.5 2.5 4.5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                      {/if}
                    </div>
                    <div class="px-1 text-right text-[#7d8590] select-none text-xs">{row.lineNo}</div>
                    <div class="pr-3 pl-2 text-[#fff3bf] whitespace-pre overflow-hidden text-ellipsis">{row.text || " "}</div>
                  </div>
                {/each}
              </div>
            </section>
          </div>

          <!-- Output Header Bar -->
          <div class="shrink-0 px-3 h-9 border-y border-[#30363d] bg-[#1d2230] flex items-center justify-between">
            <span class="text-sm font-semibold text-[#c9d1d9]">Output</span>
            <div class="flex items-center gap-2 text-xs">
              <span class="text-[#58a6ff]">conflict {conflictCount === 0 ? 0 : activeConflictIndex + 1} of {conflictCount}</span>
              <button
                type="button"
                class="h-6 w-6 inline-flex items-center justify-center rounded border border-[#3f4b63] bg-[#1f2b46] text-[#9ec1ff] hover:bg-[#263759] disabled:opacity-40 disabled:cursor-not-allowed"
                onclick={goToPrevConflict}
                disabled={conflictCount <= 1}
                title="Previous conflict"
              >
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M18 15l-6-6-6 6"></path>
                </svg>
              </button>
              <button
                type="button"
                class="h-6 w-6 inline-flex items-center justify-center rounded border border-[#3f4b63] bg-[#1f2b46] text-[#9ec1ff] hover:bg-[#263759] disabled:opacity-40 disabled:cursor-not-allowed"
                onclick={goToNextConflict}
                disabled={conflictCount <= 1}
                title="Next conflict"
              >
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M6 9l6 6 6-6"></path>
                </svg>
              </button>
            </div>
            <button
              type="button"
              class="px-2.5 py-1 text-[11px] font-medium rounded border border-[#30363d] text-[#c9d1d9] bg-[#21262d] hover:bg-[#30363d] hover:text-white transition-colors"
              onclick={() => resetConflict(activeConflictIndex)}
              title="Reset current conflict resolution"
            >
              Reset
            </button>
          </div>

          <!-- Output Content -->
          <div class="flex-1 min-h-0 overflow-auto bg-[#0d1117] font-mono text-[13px] leading-6" bind:this={panelOutputEl}>
            {#if parseWarning}
              <div class="mx-3 mt-2 rounded border border-[#d29922]/45 bg-[#2d230f] p-2 text-xs text-[#e3b341]">
                {parseWarning}
              </div>
            {/if}
            {#if error}
              <div class="mx-3 mt-2 rounded border border-[#f85149]/45 bg-[#3b1f2c] p-2 text-xs text-[#ff7b72]">
                {error}
              </div>
            {/if}

            {#each fullOutputRows as row (`output-${row.lineNo}`)}
              {@const isConflict = row.conflictIndex !== undefined}
              {@const isFirst = isConflict && row.conflictLineIndex === 0}
              {@const isOurs = row.sourceSide === "ours"}
              {@const isTheirs = row.sourceSide === "theirs"}
              <div
                class="grid grid-cols-[24px_48px_1fr] {isTheirs ? 'bg-[#584e22]/60 border-l-2 border-l-[#f3cc47]' : isOurs ? 'bg-[#14485c]/60 border-l-2 border-l-[#36a9da]' : isConflict ? 'bg-[#21262d]/40' : ''}"
                data-conflict-start={isFirst ? row.conflictIndex : undefined}
              >
                <div class="flex items-center justify-center">
                  {#if isTheirs}
                    <span class="text-[10px] font-bold text-[#ffe38b]">B</span>
                  {:else if isOurs}
                    <span class="text-[10px] font-bold text-[#9ee7ff]">A</span>
                  {/if}
                </div>
                <div class="px-1 text-right text-[#7d8590] select-none text-xs">{row.lineNo}</div>
                <div class="pr-3 pl-2 whitespace-pre overflow-hidden text-ellipsis {isTheirs ? 'text-[#fff3bf]' : isOurs ? 'text-[#d5f5ff]' : 'text-[#d2d9e7]'}">
                {#if isConflict && (isOurs || isTheirs)}
                  <span class="inline-flex items-center gap-1">
                    <svg class="w-3.5 h-3.5 text-green-400 shrink-0 inline" viewBox="0 0 20 20" fill="none">
                      <circle cx="10" cy="10" r="8" fill="currentColor" opacity="0.25"/>
                      <path d="M6.5 10.5l2.5 2.5 4.5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                    {row.text || " "}
                  </span>
                {:else}
                  {row.text || " "}
                {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Footer -->
      <div class="shrink-0 px-4 py-3 border-t border-[#30363d] bg-[#0d1117] flex items-center justify-end gap-2">
        <button
          type="button"
          class="px-3 py-1.5 text-xs font-medium text-[#c9d1d9] hover:text-white bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
          onclick={closeModal}
          disabled={saving}
        >
          Cancel
        </button>
        <button
          type="button"
          class="px-3 py-1.5 text-xs font-medium text-white bg-[#238636] hover:bg-[#2ea043] rounded border border-[rgba(240,246,252,0.1)] shadow-sm transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
          onclick={() => void handleSave()}
          disabled={loading || saving || !filePath}
        >
          {saving ? "Saving..." : "Save & Mark Resolved"}
        </button>
      </div>
    </div>
  </div>
{/if}
