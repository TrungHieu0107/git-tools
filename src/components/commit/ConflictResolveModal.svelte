<script lang="ts">
  import { GitService, type ConflictFile } from "../../lib/GitService";
  import { toast } from "../../lib/toast.svelte";

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
  };

  interface Props {
    repoPath?: string;
    filePath: string | null;
    onClose: () => void;
    onResolved?: (filePath: string) => void | Promise<void>;
  }

  let { repoPath, filePath, onClose, onResolved }: Props = $props();

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
  let selectedLinesByKey = $state<Record<string, number[]>>({});
  let loadToken = 0;

  const CONFLICT_BLOCK_REGEX =
    /^<<<<<<<[^\n]*\n([\s\S]*?)(?:^\|\|\|\|\|\|\|[^\n]*\n[\s\S]*?)?^=======\n([\s\S]*?)^>>>>>>>[^\n]*(?:\n|$)/gm;

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

  function countLogicalLines(text: string): number {
    const lines = toDisplayLines(text);
    return lines.length === 0 ? 0 : lines.length;
  }

  function selectionKey(side: "ours" | "theirs", conflictIndex: number): string {
    return `${side}:${conflictIndex}`;
  }

  function getConflictLinesBySide(
    side: "ours" | "theirs",
    conflictIndex: number,
  ): string[] {
    const conflict = conflictSegments[conflictIndex];
    if (!conflict) return [];
    return toDisplayLines(side === "ours" ? conflict.ours : conflict.theirs);
  }

  function getSelectedLineIndexes(
    side: "ours" | "theirs",
    conflictIndex: number,
  ): number[] {
    const key = selectionKey(side, conflictIndex);
    return selectedLinesByKey[key] ?? [];
  }

  function isLineSelected(
    side: "ours" | "theirs",
    conflictIndex: number,
    lineIndex: number,
  ): boolean {
    return getSelectedLineIndexes(side, conflictIndex).includes(lineIndex);
  }

  function setSelectedLineIndexes(
    side: "ours" | "theirs",
    conflictIndex: number,
    indexes: number[],
  ): void {
    const key = selectionKey(side, conflictIndex);
    selectedLinesByKey = {
      ...selectedLinesByKey,
      [key]: [...new Set(indexes)].sort((a, b) => a - b),
    };
  }

  function toggleSelectedLine(
    side: "ours" | "theirs",
    conflictIndex: number,
    lineIndex: number,
  ): void {
    const current = getSelectedLineIndexes(side, conflictIndex);
    if (current.includes(lineIndex)) {
      setSelectedLineIndexes(
        side,
        conflictIndex,
        current.filter((index) => index !== lineIndex),
      );
      return;
    }

    setSelectedLineIndexes(side, conflictIndex, [...current, lineIndex]);
  }

  function selectAllLinesFromSide(side: "ours" | "theirs", conflictIndex: number): void {
    const lines = getConflictLinesBySide(side, conflictIndex);
    setSelectedLineIndexes(
      side,
      conflictIndex,
      lines.map((_, index) => index),
    );
  }

  function clearSelectedLinesFromSide(side: "ours" | "theirs", conflictIndex: number): void {
    setSelectedLineIndexes(side, conflictIndex, []);
  }

  function isBlockFullySelected(side: "ours" | "theirs", conflictIndex: number): boolean {
    const lines = getConflictLinesBySide(side, conflictIndex);
    if (lines.length === 0) return false;
    return getSelectedLineIndexes(side, conflictIndex).length === lines.length;
  }

  function setBlockSelected(
    side: "ours" | "theirs",
    conflictIndex: number,
    selected: boolean,
  ): void {
    if (selected) {
      selectAllLinesFromSide(side, conflictIndex);
    } else {
      clearSelectedLinesFromSide(side, conflictIndex);
    }
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
      const highlighted = conflictIdx === activeConflictIndex;

      if (lines.length === 0) {
        rows.push({
          lineNo,
          text: "",
          highlighted,
          conflictIndex: conflictIdx,
          conflictLineIndex: 0,
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
  let isSingleLineConflictLayout = $derived.by(() => {
    if (useManualEditor || conflictCount !== 1 || conflictSegments.length !== 1) return false;
    const onlyConflict = conflictSegments[0];
    return countLogicalLines(onlyConflict.ours) <= 1 && countLogicalLines(onlyConflict.theirs) <= 1;
  });
  let fullOursRows = $derived.by(() => buildFullViewRows("ours"));
  let fullTheirsRows = $derived.by(() => buildFullViewRows("theirs"));
  let fullOutputRows = $derived.by(() => buildFullViewRows("output"));
  let activeResolvedLineValue = $derived.by(() => {
    const activeConflict = conflictSegments[activeConflictIndex];
    if (!activeConflict) return "";
    const lines = toDisplayLines(activeConflict.resolved);
    return lines[0] ?? "";
  });

  function clampConflictIndex(index: number): number {
    if (conflictCount <= 0) return 0;
    if (index < 0) return 0;
    if (index >= conflictCount) return conflictCount - 1;
    return index;
  }

  function getConflictIndexBySegmentId(segmentId: string): number {
    let index = 0;
    for (const segment of segments) {
      if (segment.kind !== "conflict") continue;
      if (segment.id === segmentId) return index;
      index += 1;
    }
    return -1;
  }

  function selectConflict(index: number): void {
    activeConflictIndex = clampConflictIndex(index);
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

  function isActiveConflictSegment(segmentId: string): boolean {
    return getConflictIndexBySegmentId(segmentId) === activeConflictIndex;
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
    selectedLinesByKey = {};

    try {
      const [conflictResult, modifiedContentResult] = await Promise.allSettled([
        GitService.getConflictFile(targetPath, repoPath),
        GitService.getFileModifiedContent(targetPath, false, repoPath),
      ]);
      if (currentLoadToken !== loadToken) return;

      if (conflictResult.status === "rejected") {
        throw conflictResult.reason;
      }

      const nextConflictFile = conflictResult.value;
      conflictFile = nextConflictFile;

      const modifiedContent =
        modifiedContentResult.status === "fulfilled" ? modifiedContentResult.value : null;
      const sourceContent = modifiedContent ?? buildFallbackConflictContent(nextConflictFile);
      newlineStyle = detectNewlineStyle(sourceContent);

      const parsed = parseResolutionSegments(sourceContent);
      segments = parsed.segments;

      if (modifiedContentResult.status === "rejected") {
        parseWarning = "Working tree version is unavailable. Showing content from conflict stages.";
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

  function applyConflictFromOurs(segmentId: string): void {
    const conflictIndex = getConflictIndexBySegmentId(segmentId);
    if (conflictIndex >= 0) {
      activeConflictIndex = conflictIndex;
    }
    segments = segments.map((segment) => {
      if (segment.kind !== "conflict" || segment.id !== segmentId) return segment;
      return { ...segment, resolved: segment.ours };
    });
  }

  function applyConflictFromTheirs(segmentId: string): void {
    const conflictIndex = getConflictIndexBySegmentId(segmentId);
    if (conflictIndex >= 0) {
      activeConflictIndex = conflictIndex;
    }
    segments = segments.map((segment) => {
      if (segment.kind !== "conflict" || segment.id !== segmentId) return segment;
      return { ...segment, resolved: segment.theirs };
    });
  }

  function updateResolvedSegment(segmentId: string, value: string): void {
    const conflictIndex = getConflictIndexBySegmentId(segmentId);
    if (conflictIndex >= 0) {
      activeConflictIndex = conflictIndex;
    }
    const normalizedValue = normalizeNewlines(value);
    segments = segments.map((segment) => {
      if (segment.kind !== "conflict" || segment.id !== segmentId) return segment;
      return { ...segment, resolved: normalizedValue };
    });
  }

  function applyAllFrom(side: "ours" | "theirs"): void {
    if (useManualEditor) {
      const source =
        side === "ours"
          ? normalizeNewlines(conflictFile?.ours ?? "")
          : normalizeNewlines(conflictFile?.theirs ?? "");
      manualContent = source;
      return;
    }

    segments = segments.map((segment) => {
      if (segment.kind !== "conflict") return segment;
      return { ...segment, resolved: side === "ours" ? segment.ours : segment.theirs };
    });
  }

  function applyConflictAtIndexFrom(side: "ours" | "theirs", index: number): void {
    const target = conflictSegments[index];
    if (!target) return;
    selectConflict(index);
    if (side === "ours") {
      applyConflictFromOurs(target.id);
    } else {
      applyConflictFromTheirs(target.id);
    }
  }

  function applySelectedLinesAtIndexFrom(
    side: "ours" | "theirs",
    conflictIndex: number,
  ): void {
    const target = conflictSegments[conflictIndex];
    if (!target) return;

    const selectedIndexes = getSelectedLineIndexes(side, conflictIndex);
    if (selectedIndexes.length === 0) {
      toast.error("Please select at least one line first.");
      return;
    }

    const lines = getConflictLinesBySide(side, conflictIndex);
    const selectedText = selectedIndexes
      .filter((index) => index >= 0 && index < lines.length)
      .map((index) => lines[index])
      .join("\n");

    selectConflict(conflictIndex);
    updateResolvedSegment(target.id, selectedText);
  }

  function applyFromSideUsingSelection(side: "ours" | "theirs", conflictIndex: number): void {
    if (getSelectedLineIndexes(side, conflictIndex).length > 0) {
      applySelectedLinesAtIndexFrom(side, conflictIndex);
      return;
    }
    applyConflictAtIndexFrom(side, conflictIndex);
  }

  function updateActiveResolvedSingleLine(value: string): void {
    const activeConflict = conflictSegments[activeConflictIndex];
    if (!activeConflict) return;
    updateResolvedSegment(activeConflict.id, value);
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

  async function handleSave(): Promise<void> {
    if (!filePath || saving) return;
    saving = true;
    error = null;

    try {
      const finalContent = useManualEditor
        ? denormalizeNewlines(manualContent)
        : denormalizeNewlines(composeResolvedContent(segments));

      await GitService.writeFile(filePath, finalContent, repoPath);
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
      <div class="shrink-0 px-4 py-3 border-b border-[#30363d] bg-[#0d1117] flex items-center gap-3">
        <div class="min-w-0 flex-1">
          <h3 class="text-sm font-semibold text-white truncate">Resolve Conflict</h3>
          <p class="text-xs text-[#8b949e] truncate" title={filePath}>{filePath}</p>
        </div>
        <span class="text-[11px] px-2 py-1 rounded border border-[#30363d] text-[#c9d1d9]">
          {conflictCount} block(s)
        </span>
        <button
          type="button"
          class="text-[#8b949e] hover:text-[#c9d1d9] p-1 rounded hover:bg-[#30363d] transition-colors"
          onclick={closeModal}
          aria-label="Close"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
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
      {:else}
        {#if isSingleLineConflictLayout}
          <div class="flex-1 min-h-0 flex flex-col bg-[#0d1117]">
            <div class="shrink-0 px-3 py-2 border-b border-[#30363d] bg-[#111827] flex items-center justify-between gap-2 text-xs">
              <div class="text-[#8b949e] font-semibold uppercase tracking-wider">Single-line Conflict View</div>
              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  class="px-2 py-1 rounded border border-[#2ea8d6]/60 bg-[#0d2f3c] text-[#9ee7ff] hover:bg-[#124152] transition-colors"
                  onclick={() => applyFromSideUsingSelection("ours", activeConflictIndex)}
                >
                  Use Local
                </button>
                <button
                  type="button"
                  class="px-2 py-1 rounded border border-[#d7bb4f]/60 bg-[#433913] text-[#ffe38b] hover:bg-[#5c4d1b] transition-colors"
                  onclick={() => applyFromSideUsingSelection("theirs", activeConflictIndex)}
                >
                  Use Origin
                </button>
              </div>
            </div>

            <div class="min-h-0 h-[62%] grid grid-cols-2 divide-x divide-[#30363d]">
              <section class="min-h-0 flex flex-col">
                <div class="shrink-0 px-3 h-8 border-b border-[#30363d] bg-[#0f2732] text-[#9ee7ff] text-xs font-semibold flex items-center gap-2">
                  <input
                    type="checkbox"
                    class="h-3.5 w-3.5 accent-[#36a9da]"
                    checked={isBlockFullySelected("ours", activeConflictIndex)}
                    onchange={(event) =>
                      setBlockSelected(
                        "ours",
                        activeConflictIndex,
                        (event.currentTarget as HTMLInputElement).checked,
                      )}
                    title="Select entire local block"
                  />
                  <span class="inline-flex h-5 w-5 items-center justify-center rounded-sm border border-[#1f6f8b] bg-[#123645]">A</span>
                  Local (Ours)
                </div>
                <div class="flex-1 min-h-0 overflow-auto font-mono text-[13px] leading-6">
                  {#each fullOursRows as row (`ours-${row.lineNo}`)}
                    <div class="grid grid-cols-[22px_56px_1fr] {row.highlighted ? 'bg-[#14485c]/75 border-l-2 border-l-[#36a9da]' : ''}">
                      <div class="flex items-center justify-center">
                        {#if row.conflictIndex !== undefined && row.conflictLineIndex !== undefined}
                          <input
                            type="checkbox"
                            class="h-3.5 w-3.5 accent-[#36a9da]"
                            checked={isLineSelected("ours", row.conflictIndex, row.conflictLineIndex)}
                            onchange={() => toggleSelectedLine("ours", row.conflictIndex!, row.conflictLineIndex!)}
                            title={`Select line ${row.lineNo}`}
                          />
                        {/if}
                      </div>
                      <div class="px-2 text-right text-[#7d8590] select-none">{row.lineNo}</div>
                      <div class="pr-3 pl-2 text-[#d5f5ff] whitespace-pre">{row.text || " "}</div>
                    </div>
                  {/each}
                </div>
              </section>

              <section class="min-h-0 flex flex-col">
                <div class="shrink-0 px-3 h-8 border-b border-[#30363d] bg-[#302a14] text-[#ffe38b] text-xs font-semibold flex items-center gap-2">
                  <input
                    type="checkbox"
                    class="h-3.5 w-3.5 accent-[#f3cc47]"
                    checked={isBlockFullySelected("theirs", activeConflictIndex)}
                    onchange={(event) =>
                      setBlockSelected(
                        "theirs",
                        activeConflictIndex,
                        (event.currentTarget as HTMLInputElement).checked,
                      )}
                    title="Select entire origin block"
                  />
                  <span class="inline-flex h-5 w-5 items-center justify-center rounded-sm border border-[#c8a833] bg-[#4b3f1b]">B</span>
                  Origin (Theirs)
                </div>
                <div class="flex-1 min-h-0 overflow-auto font-mono text-[13px] leading-6">
                  {#each fullTheirsRows as row (`theirs-${row.lineNo}`)}
                    <div class="grid grid-cols-[22px_56px_1fr] {row.highlighted ? 'bg-[#584e22]/75 border-l-2 border-l-[#f3cc47]' : ''}">
                      <div class="flex items-center justify-center">
                        {#if row.conflictIndex !== undefined && row.conflictLineIndex !== undefined}
                          <input
                            type="checkbox"
                            class="h-3.5 w-3.5 accent-[#f3cc47]"
                            checked={isLineSelected("theirs", row.conflictIndex, row.conflictLineIndex)}
                            onchange={() => toggleSelectedLine("theirs", row.conflictIndex!, row.conflictLineIndex!)}
                            title={`Select line ${row.lineNo}`}
                          />
                        {/if}
                      </div>
                      <div class="px-2 text-right text-[#7d8590] select-none">{row.lineNo}</div>
                      <div class="pr-3 pl-2 text-[#fff3bf] whitespace-pre">{row.text || " "}</div>
                    </div>
                  {/each}
                </div>
              </section>
            </div>

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
            </div>

            <section class="flex-1 min-h-0 overflow-auto bg-[#0d1117] font-mono text-[13px] leading-6">
              {#if parseWarning}
                <div class="mx-3 mt-2 rounded border border-[#d29922]/45 bg-[#2d230f] p-2 text-xs text-[#e3b341]">
                  {parseWarning}
                </div>
              {/if}

              <div class="min-h-0 overflow-auto mt-2">
                {#each fullOutputRows as row (`output-${row.lineNo}`)}
                  <div class="grid grid-cols-[56px_1fr] {row.highlighted ? 'bg-[#5a1f66]/80 border-l-2 border-l-[#d02ef0]' : ''}">
                    <div class="px-2 text-right text-[#7d8590] select-none">{row.lineNo}</div>
                    <div class="pr-3 pl-2 text-[#d2d9e7] whitespace-pre">
                      {#if row.highlighted}
                        <input
                          type="text"
                          class="w-full bg-transparent text-[#f0dcff] outline-none border-none p-0 m-0"
                          value={activeResolvedLineValue}
                          oninput={(event) => updateActiveResolvedSingleLine((event.currentTarget as HTMLInputElement).value)}
                        />
                      {:else}
                        {row.text || " "}
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </section>
          </div>
        {:else}
        <div class="flex-1 min-h-0 flex flex-col bg-[#0d1117]">
          <div class="shrink-0 px-3 py-2 border-b border-[#30363d] bg-[#111827] flex flex-wrap items-center justify-between gap-2 text-[11px]">
            <div class="font-semibold uppercase tracking-wider text-[#8b949e]">
              Conflict Source Panels
            </div>
            <div class="flex flex-wrap gap-1.5">
              <button
                type="button"
                class="px-2 py-1 rounded border border-[#238636]/55 text-[#7ee787] bg-[#0f2418] hover:bg-[#153220] transition-colors"
                onclick={() => applyAllFrom("ours")}
              >
                Apply All Local
              </button>
              <button
                type="button"
                class="px-2 py-1 rounded border border-[#1f6feb]/55 text-[#79c0ff] bg-[#11213a] hover:bg-[#17305a] transition-colors"
                onclick={() => applyAllFrom("theirs")}
              >
                Apply All Origin
              </button>
            </div>
          </div>

          <div class="min-h-0 h-[42%] grid grid-cols-2 divide-x divide-[#30363d] max-[1100px]:h-[48%]">
            <section class="min-h-0 flex flex-col bg-[#0d1117]">
              <div class="shrink-0 px-3 py-2 border-b border-[#30363d] text-xs font-semibold text-[#9ee7ff] bg-[#0f2732] flex items-center gap-2">
                <span class="inline-flex h-5 w-5 items-center justify-center rounded-sm border border-[#1f6f8b] bg-[#123645] text-[#7cd8ff]">A</span>
                Local (Ours)
              </div>
              <div class="flex-1 min-h-0 overflow-auto p-2 space-y-2">
                {#if conflictSegments.length === 0}
                  <pre class="h-full rounded border border-[#30363d] bg-[#0f1620] p-2 text-xs leading-relaxed whitespace-pre-wrap font-mono text-[#c9d1d9]">{conflictFile?.ours ?? ""}</pre>
                {:else}
                  {#each conflictSegments as conflict, index (conflict.id)}
                    <div
                      class="rounded border overflow-hidden transition-colors {index === activeConflictIndex
                        ? 'border-[#36a9da] bg-[#14485c]/70'
                        : 'border-[#1f6f8b]/40 bg-[#123341]/45'}"
                    >
                      <div class="px-2 py-1 border-b border-[#1f6f8b]/45 flex items-center justify-between gap-2 text-[10px] uppercase tracking-wider text-[#9ee7ff]">
                        <span>Conflict {index + 1}</span>
                        <div class="flex items-center gap-1">
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#2ea8d6]/45 bg-[#0d2f3c] text-[#9ee7ff] hover:bg-[#124152] transition-colors normal-case"
                            onclick={() => selectAllLinesFromSide("ours", index)}
                            title="Select all lines in this block"
                          >
                            All
                          </button>
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#2ea8d6]/35 bg-[#0d2f3c] text-[#9ee7ff] hover:bg-[#124152] transition-colors normal-case disabled:opacity-45 disabled:cursor-not-allowed"
                            onclick={() => clearSelectedLinesFromSide("ours", index)}
                            disabled={getSelectedLineIndexes("ours", index).length === 0}
                            title="Clear selected lines"
                          >
                            Clear
                          </button>
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#2ea8d6]/60 bg-[#0d2f3c] text-[#9ee7ff] hover:bg-[#124152] transition-colors normal-case disabled:opacity-45 disabled:cursor-not-allowed"
                            onclick={() => applySelectedLinesAtIndexFrom("ours", index)}
                            disabled={getSelectedLineIndexes("ours", index).length === 0}
                            title="Apply selected lines to output"
                          >
                            Selected &gt;&gt;
                          </button>
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#2ea8d6]/60 bg-[#0d2f3c] text-[#9ee7ff] hover:bg-[#124152] transition-colors normal-case"
                            onclick={() => applyConflictAtIndexFrom("ours", index)}
                            title="Apply local block to output"
                          >
                            Block &gt;&gt;
                          </button>
                        </div>
                      </div>
                      <div class="max-h-[180px] overflow-auto p-1">
                        {#if getConflictLinesBySide("ours", index).length === 0}
                          <div class="px-2 py-1 text-xs text-[#9ee7ff]/70 font-mono">(empty)</div>
                        {:else}
                          {#each getConflictLinesBySide("ours", index) as line, lineIndex (`ours-${conflict.id}-${lineIndex}`)}
                            <button
                              type="button"
                              class="w-full px-2 py-1 text-left text-xs font-mono leading-relaxed rounded flex items-center gap-2 transition-colors {isLineSelected('ours', index, lineIndex) ? 'bg-[#1b5c74] text-[#ecfbff]' : 'text-[#d5f5ff] hover:bg-[#18495b]/70'}"
                              onclick={() => toggleSelectedLine("ours", index, lineIndex)}
                              title={`Select line ${lineIndex + 1}`}
                            >
                              <input
                                type="checkbox"
                                checked={isLineSelected("ours", index, lineIndex)}
                                class="h-3.5 w-3.5 pointer-events-none accent-[#36a9da]"
                                tabindex="-1"
                              />
                              <span class="w-6 shrink-0 text-right text-[#84cfe8]">{lineIndex + 1}</span>
                              <span class="whitespace-pre-wrap break-all">{line || " "}</span>
                            </button>
                          {/each}
                        {/if}
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
            </section>

            <section class="min-h-0 flex flex-col bg-[#0d1117]">
              <div class="shrink-0 px-3 py-2 border-b border-[#30363d] text-xs font-semibold text-[#ffe38b] bg-[#302a14] flex items-center gap-2">
                <span class="inline-flex h-5 w-5 items-center justify-center rounded-sm border border-[#c8a833] bg-[#4b3f1b] text-[#ffe38b]">B</span>
                Origin (Theirs)
              </div>
              <div class="flex-1 min-h-0 overflow-auto p-2 space-y-2">
                {#if conflictSegments.length === 0}
                  <pre class="h-full rounded border border-[#30363d] bg-[#0f1620] p-2 text-xs leading-relaxed whitespace-pre-wrap font-mono text-[#c9d1d9]">{conflictFile?.theirs ?? ""}</pre>
                {:else}
                  {#each conflictSegments as conflict, index (conflict.id)}
                    <div
                      class="rounded border overflow-hidden transition-colors {index === activeConflictIndex
                        ? 'border-[#f3cc47] bg-[#584e22]/70'
                        : 'border-[#8a6d1d]/45 bg-[#3f3618]/45'}"
                    >
                      <div class="px-2 py-1 border-b border-[#8a6d1d]/45 flex items-center justify-between gap-2 text-[10px] uppercase tracking-wider text-[#ffe38b]">
                        <div class="flex items-center gap-1">
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#d7bb4f]/45 bg-[#433913] text-[#ffe38b] hover:bg-[#5c4d1b] transition-colors normal-case"
                            onclick={() => selectAllLinesFromSide("theirs", index)}
                            title="Select all lines in this block"
                          >
                            All
                          </button>
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#d7bb4f]/35 bg-[#433913] text-[#ffe38b] hover:bg-[#5c4d1b] transition-colors normal-case disabled:opacity-45 disabled:cursor-not-allowed"
                            onclick={() => clearSelectedLinesFromSide("theirs", index)}
                            disabled={getSelectedLineIndexes("theirs", index).length === 0}
                            title="Clear selected lines"
                          >
                            Clear
                          </button>
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#d7bb4f]/60 bg-[#433913] text-[#ffe38b] hover:bg-[#5c4d1b] transition-colors normal-case disabled:opacity-45 disabled:cursor-not-allowed"
                            onclick={() => applySelectedLinesAtIndexFrom("theirs", index)}
                            disabled={getSelectedLineIndexes("theirs", index).length === 0}
                            title="Apply selected lines to output"
                          >
                            &lt;&lt; Selected
                          </button>
                          <button
                            type="button"
                            class="px-1.5 py-0.5 rounded border border-[#d7bb4f]/60 bg-[#433913] text-[#ffe38b] hover:bg-[#5c4d1b] transition-colors normal-case"
                            onclick={() => applyConflictAtIndexFrom("theirs", index)}
                            title="Apply origin block to output"
                          >
                            &lt;&lt; Block
                          </button>
                        </div>
                        <span>Conflict {index + 1}</span>
                      </div>
                      <div class="max-h-[180px] overflow-auto p-1">
                        {#if getConflictLinesBySide("theirs", index).length === 0}
                          <div class="px-2 py-1 text-xs text-[#ffe38b]/70 font-mono">(empty)</div>
                        {:else}
                          {#each getConflictLinesBySide("theirs", index) as line, lineIndex (`theirs-${conflict.id}-${lineIndex}`)}
                            <button
                              type="button"
                              class="w-full px-2 py-1 text-left text-xs font-mono leading-relaxed rounded flex items-center gap-2 transition-colors {isLineSelected('theirs', index, lineIndex) ? 'bg-[#6c5f2a] text-[#fff9dd]' : 'text-[#fff3bf] hover:bg-[#5b4d20]/60'}"
                              onclick={() => toggleSelectedLine("theirs", index, lineIndex)}
                              title={`Select line ${lineIndex + 1}`}
                            >
                              <input
                                type="checkbox"
                                checked={isLineSelected("theirs", index, lineIndex)}
                                class="h-3.5 w-3.5 pointer-events-none accent-[#f3cc47]"
                                tabindex="-1"
                              />
                              <span class="w-6 shrink-0 text-right text-[#f4d982]">{lineIndex + 1}</span>
                              <span class="whitespace-pre-wrap break-all">{line || " "}</span>
                            </button>
                          {/each}
                        {/if}
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
            </section>
          </div>

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
          </div>

          <section class="flex-1 min-h-0 overflow-auto p-3 space-y-3 bg-[#0d1117]">
            {#if error}
              <div class="rounded border border-[#f85149]/45 bg-[#3b1f2c] p-2 text-xs text-[#ff7b72]">
                {error}
              </div>
            {/if}

            {#if parseWarning}
              <div class="rounded border border-[#d29922]/45 bg-[#2d230f] p-2 text-xs text-[#e3b341]">
                {parseWarning}
              </div>
            {/if}

            {#if useManualEditor}
              <textarea
                class="w-full min-h-[280px] h-[calc(100%-2px)] bg-[#0f1620] border border-[#30363d] rounded p-3 font-mono text-xs text-[#c9d1d9] focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] resize-y"
                bind:value={manualContent}
              ></textarea>
            {:else}
              {#each segments as segment (segment.id)}
                {#if segment.kind === "plain"}
                  {#if segment.text.trim().length > 0}
                    <div class="rounded border border-[#30363d] bg-[#0f1620] overflow-hidden">
                      <div class="px-2 py-1 border-b border-[#30363d] text-[10px] uppercase tracking-wider text-[#8b949e]">
                        Context
                      </div>
                      <pre class="p-2 text-[11px] leading-relaxed whitespace-pre-wrap font-mono text-[#8b949e] max-h-[150px] overflow-auto">{segment.text}</pre>
                    </div>
                  {/if}
                {:else}
                  {@const currentConflictIndex = getConflictIndexBySegmentId(segment.id)}
                  <div
                    class="rounded border overflow-hidden transition-colors {isActiveConflictSegment(segment.id)
                      ? 'border-[#58a6ff]/80 bg-[#14253d]/55'
                      : 'border-[#30363d] bg-[#0f1620]'}"
                  >
                    <div class="px-2 py-1 border-b border-[#30363d] flex items-center justify-between gap-2">
                      <span class="text-[10px] uppercase tracking-wider text-[#8b949e]">
                        Output Conflict {currentConflictIndex + 1}
                      </span>
                      <div class="flex gap-1">
                        <button
                          type="button"
                          class="px-2 py-0.5 text-[10px] rounded border border-[#238636]/55 text-[#7ee787] bg-[#0f2418] hover:bg-[#153220] transition-colors"
                          onclick={() => applyConflictFromOurs(segment.id)}
                          title="Use local block"
                        >
                          &lt;&lt; Local
                        </button>
                        <button
                          type="button"
                          class="px-2 py-0.5 text-[10px] rounded border border-[#1f6feb]/55 text-[#79c0ff] bg-[#11213a] hover:bg-[#17305a] transition-colors"
                          onclick={() => applyConflictFromTheirs(segment.id)}
                          title="Use origin block"
                        >
                          Origin &gt;&gt;
                        </button>
                      </div>
                    </div>
                    <textarea
                      class="w-full min-h-[110px] bg-transparent p-2 font-mono text-xs text-[#c9d1d9] focus:outline-none focus:bg-[#101b2b] resize-y"
                      value={segment.resolved}
                      onfocus={() => selectConflict(currentConflictIndex)}
                      oninput={(event) => updateResolvedSegment(segment.id, (event.currentTarget as HTMLTextAreaElement).value)}
                    ></textarea>
                  </div>
                {/if}
              {/each}
            {/if}
          </section>
        </div>
        {/if}
      {/if}

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
