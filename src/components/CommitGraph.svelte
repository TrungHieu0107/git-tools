<script lang="ts">
  import { buildStraightGraphPath, type GraphNode, type GraphEdge } from "../lib/graph-layout";
  import { onMount } from "svelte";
  import { GitService } from "../lib/GitService";
  import { confirm } from "../lib/confirmation.svelte";
  import ResizablePanel from "./resize/ResizablePanel.svelte";
  import { computeDiff, isLargeFile, extractHunks, type DiffResult, type DiffHunk } from "../lib/diff";
  import DiffView from "./diff/DiffView.svelte";
  import DiffToolbar from "./diff/DiffToolbar.svelte";

  interface Props {
    nodes?: GraphNode[];
    edges?: GraphEdge[];
    repoPath?: string;
    pendingPushCount?: number;
    onGraphReload?: () => Promise<void>;
  }

  let { nodes = [], edges = [], repoPath, pendingPushCount = 0, onGraphReload }: Props = $props();

  const ROW_HEIGHT = 40;
  const COL_WIDTH = 28;
  const DOT_RADIUS = 5;
  const STROKE_WIDTH = 2;
  const MERGE_TURN_GAP = 10;
  const PADDING_TOP = 8;
  const PADDING_LEFT = 24;
  const TOOLTIP_OFFSET_X = 14;
  const TOOLTIP_OFFSET_Y = 12;
  const TOOLTIP_MAX_WIDTH = 360;
  const TOOLTIP_MAX_HEIGHT = 88;

  const HEADER_BASE = "h-8 flex items-center bg-[#161b22] border-b border-[#30363d] shrink-0";
  
  // -- State -- 
  interface Column {
      id: string;
      label: string;
      width: number;
      visible: boolean;
      minWidth: number;
  }

  let columns = $state<Column[]>([
      { id: "graph", label: "Graph", width: 300, visible: true, minWidth: 100 },
      { id: "hash", label: "Hash", width: 80, visible: true, minWidth: 60 },
      { id: "message", label: "Message", width: 400, visible: true, minWidth: 200 }, // Using numeric width to serve as flex basis concept if we were using flex, but for grid we can treat it as pixels or '1fr' logic if we get fancy. For now, pixel based resizing is robust.
      { id: "author", label: "Author", width: 150, visible: true, minWidth: 80 },
      { id: "date", label: "Date", width: 140, visible: true, minWidth: 100 }
  ]);

  // Derived grid template
  let gridTemplate = $derived(columns.filter(c => c.visible).map(c => c.id === 'message' ? `minmax(${c.width}px, 1fr)` : `${c.width}px`).join(" "));
  let visibleColumns = $derived(columns.filter(c => c.visible));

  // Selection & Details
  let selectedCommit = $state<GraphNode | null>(null);
  let changedFiles = $state<string[]>([]);
  let isLoadingFiles = $state(false);

  // Diff View State
  let leftPanelMode = $state<'graph' | 'diff'>('graph');
  let selectedDiffFile = $state<string | null>(null);
  let isLoadingDiff = $state(false);
  let baseContent = $state("");
  let modifiedContent = $state("");
  let selectedEncoding = $state<string | undefined>(undefined);

  // Derived: full-file diff for side-by-side view (same pattern as CommitPanel/FileHistoryPanel)
  let diffResult = $derived.by<DiffResult | null>(() => {
      if (!baseContent && !modifiedContent) return null;
      if (isLargeFile(baseContent) || isLargeFile(modifiedContent)) return null;
      return computeDiff(baseContent, modifiedContent);
  });

  let isTooLarge = $derived(
      isLargeFile(baseContent) || isLargeFile(modifiedContent)
  );

  // Extract change hunks with context for hunk view mode
  let hunks = $derived.by<DiffHunk[]>(() => {
      if (!diffResult) return [];
      return extractHunks(diffResult, 3);
  });

  async function selectCommit(node: GraphNode) {
      if (selectedCommit?.hash === node.hash) return;
      
      selectedCommit = node;
      changedFiles = [];
      // Reset diff view when switching commits (optional, or keep if same file exists?)
      closeDiff(); 
      
      if (!repoPath) return;

      isLoadingFiles = true;
      const targetHash = node.hash;
      try {
          const files = await GitService.getCommitChangedFiles(node.hash, repoPath);
          if (selectedCommit?.hash === targetHash) {
              changedFiles = files;
          }
      } catch (e) {
          console.error("Failed to load commit files", e);
      } finally {
          if (selectedCommit?.hash === targetHash) {
              isLoadingFiles = false;
          }
      }
  }

  function closeDetails() {
      selectedCommit = null;
      changedFiles = [];
      closeDiff();
  }

  async function openDiff(file: string) {
      if (!selectedCommit || !repoPath) return;

      selectedDiffFile = file;
      leftPanelMode = 'diff';
      isLoadingDiff = true;
      baseContent = "";
      modifiedContent = "";

      try {
          // Step 1: Get commit diff to find parent hash
          const diff = await GitService.getCommitDiff(selectedCommit.hash, repoPath, file, selectedEncoding);

          if (selectedDiffFile !== file) return; // Race check

          // Step 2: Fetch full file contents in parallel for side-by-side view
          const promises: Promise<string>[] = [];
          // Modified content (file at selected commit)
          promises.push(
              GitService.getFileAtCommit(selectedCommit.hash, file, repoPath, selectedEncoding)
                  .catch(() => "") // File might not exist (deleted)
          );
          // Base content (file at parent commit)
          if (diff.parentHash) {
              promises.push(
                  GitService.getFileAtCommit(diff.parentHash, file, repoPath, selectedEncoding)
                      .catch(() => "") // File might not exist at parent (newly added)
              );
          } else {
              promises.push(Promise.resolve("")); // Root commit — no parent
          }

          const [mod, base] = await Promise.all(promises);

          if (selectedDiffFile !== file) return; // Race check

          modifiedContent = mod;
          baseContent = base;
      } catch (e) {
          console.error("Failed to load diff", e);
      } finally {
          if (selectedDiffFile === file || selectedDiffFile === null) {
              isLoadingDiff = false;
          }
      }
  }

  function closeDiff() {
      leftPanelMode = 'graph';
      selectedDiffFile = null;
      baseContent = "";
      modifiedContent = "";
      isLoadingDiff = false;
      selectedEncoding = undefined;
  }

  function handleEncodingChange(encoding: string) {
      selectedEncoding = encoding;
      if (selectedDiffFile) {
          openDiff(selectedDiffFile);
      }
  }


  // Persistence
  onMount(() => {
      const saved = localStorage.getItem("gh_table_columns");
      if (saved) {
          try {
              const parsed = JSON.parse(saved);
              // Merge saved widths/visibility into default config (to handle potential schema changes or missing cols)
              columns = columns.map(def => {
                  const s = parsed.find((p: Column) => p.id === def.id);
                  return s ? { ...def, width: s.width, visible: s.visible } : def;
              });
          } catch(e) { console.error("Failed to load column settings", e); }
      }
  });

  $effect(() => {
      localStorage.setItem("gh_table_columns", JSON.stringify(columns));
  });


  // -- Resizing Logic --
  let resizingColId = $state<string | null>(null);
  let startX = 0;
  let startWidth = 0;

  function onMouseDown(e: MouseEvent, colId: string) {
      const col = columns.find(c => c.id === colId);
      if (!col) return;
      
      resizingColId = colId;
      startX = e.clientX;
      startWidth = col.width;
      
      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
  }

  function onMouseMove(e: MouseEvent) {
      if (!resizingColId) return;
      
      const idx = columns.findIndex(c => c.id === resizingColId);
      if (idx === -1) return;
      
      const diff = e.clientX - startX;
      let newWidth = Math.max(columns[idx].minWidth, startWidth + diff);
      
      // Mutate grid
      columns[idx].width = newWidth;
  }

  function onMouseUp() {
      resizingColId = null;
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
  }
  
  // -- Visibility Toggle --
  let showMenu = $state(false);
  let hoveredBranchColor = $state<string | null>(null);
  let hoveredCommitHash = $state<string | null>(null);
  let graphViewportEl = $state<HTMLDivElement | null>(null);

  type GraphTooltip = {
      visible: boolean;
      x: number;
      y: number;
      subject: string;
      hash: string;
  };

  let graphTooltip = $state<GraphTooltip>({
      visible: false,
      x: 0,
      y: 0,
      subject: "",
      hash: ""
  });

  type EdgeGeometry = {
      key: string;
      color: string;
      path: string;
  };

  let graphColumn = $derived(columns.find(c => c.id === "graph"));

  function columnToX(columnIndex: number) {
      return columnIndex * COL_WIDTH;
  }

  function rowToY(rowIndex: number) {
      return rowIndex * ROW_HEIGHT + ROW_HEIGHT / 2;
  }

  function nodeRenderX(node: GraphNode) {
      return columnToX(node.x);
  }

  function nodeRenderY(node: GraphNode) {
      return rowToY(node.y);
  }

  function getNodeRadius(node: GraphNode) {
      if (selectedCommit?.hash === node.hash) return DOT_RADIUS + 0.6;
      if (hoveredBranchColor === node.color) return DOT_RADIUS + 0.9;
      return DOT_RADIUS;
  }

  // Geometry is memoized against edge inputs so minor UI state updates avoid recomputing routes.
  let edgeGeometry = $derived.by<EdgeGeometry[]>(() =>
      edges.map((edge, idx) => {
          const x1 = columnToX(edge.x1);
          const y1 = rowToY(edge.y1);
          const x2 = columnToX(edge.x2);
          const y2 = rowToY(edge.y2);
          // Secondary parent edges (forks) turn near the source (child/merge commit).
          // First parent edges (continuity) turn near the target (parent).
          const turnAtStart = edge.parentIndex > 0;

          return {
              key: `${edge.x1}:${edge.y1}:${edge.x2}:${edge.y2}:${edge.color}:${idx}`,
              color: edge.color,
              path: buildStraightGraphPath(x1, y1, x2, y2, MERGE_TURN_GAP, turnAtStart)
          };
      })
  );

  function handleCommitRowKeydown(event: KeyboardEvent, node: GraphNode) {
      if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          void selectCommit(node);
      }
  }

  function getRowCellHighlightClass(nodeHash: string, columnId: string): string {
      if (columnId === "graph") return "";
      if (selectedCommit?.hash === nodeHash) return "row-info-cell-selected";
      if (hoveredCommitHash === nodeHash) return "row-info-cell-hovered";
      return "";
  }

  function getTooltipPosition(event: MouseEvent) {
      if (!graphViewportEl) {
          return { x: event.clientX + TOOLTIP_OFFSET_X, y: event.clientY + TOOLTIP_OFFSET_Y };
      }

      const rect = graphViewportEl.getBoundingClientRect();
      const rawX = event.clientX - rect.left + TOOLTIP_OFFSET_X;
      const rawY = event.clientY - rect.top + TOOLTIP_OFFSET_Y;
      const maxX = Math.max(8, rect.width - TOOLTIP_MAX_WIDTH);
      const maxY = Math.max(8, rect.height - TOOLTIP_MAX_HEIGHT);

      return {
          x: Math.min(Math.max(8, rawX), maxX),
          y: Math.min(Math.max(8, rawY), maxY)
      };
  }

  function showCommitTooltip(event: MouseEvent, node: GraphNode) {
      const pos = getTooltipPosition(event);
      graphTooltip.visible = true;
      graphTooltip.x = pos.x;
      graphTooltip.y = pos.y;
      graphTooltip.subject = node.subject;
      graphTooltip.hash = node.hash;
  }

  function moveCommitTooltip(event: MouseEvent) {
      if (!graphTooltip.visible) return;
      const pos = getTooltipPosition(event);
      graphTooltip.x = pos.x;
      graphTooltip.y = pos.y;
  }

  function hideCommitTooltip() {
      graphTooltip.visible = false;
  }

  function handleRowMouseEnter(event: MouseEvent, node: GraphNode) {
      hoveredCommitHash = node.hash;
      hoveredBranchColor = node.color;
      showCommitTooltip(event, node);
  }

  function handleRowMouseMove(event: MouseEvent) {
      moveCommitTooltip(event);
  }

  function handleRowMouseLeave() {
      hoveredCommitHash = null;
      hoveredBranchColor = null;
      hideCommitTooltip();
  }

  function handleRowFocus(node: GraphNode) {
      hoveredCommitHash = node.hash;
      hoveredBranchColor = node.color;
  }

  function handleRowBlur() {
      hoveredCommitHash = null;
      hoveredBranchColor = null;
      hideCommitTooltip();
  }

  // Helper to parse refs for badges
  function getBadges(refs: string[]) {
      const badges = [];
      for (const ref of refs) {
          if (ref.includes("HEAD ->")) {
              badges.push({ text: "HEAD", type: "head" });
              const branch = ref.split("HEAD ->")[1].trim();
              if (branch) badges.push({ text: branch, type: "branch" });
          } else if (ref.startsWith("tag:")) {
               badges.push({ text: ref.replace("tag:", "").trim(), type: "tag" });
          } else if (ref.includes("/")) {
               badges.push({ text: ref.trim(), type: "remote" });
          } else {
               badges.push({ text: ref.trim(), type: "branch" });
          }
      }
      return badges;
  }
  // -- Git Actions --
  let isFetching = $state(false);
  let isPulling = $state(false);
  let isPushing = $state(false);

  async function handleFetch() {
      if (!repoPath || isFetching) return;
      isFetching = true;
      try {
          await GitService.fetch(repoPath);
          await onGraphReload?.();
      } catch (e: any) {
          console.error("Fetch failed", e);
          await confirm({ title: "Fetch Failed", message: e.toString(), confirmLabel: "OK", cancelLabel: "Close" });
      } finally {
          isFetching = false;
      }
  }

  async function handlePull() {
      if (!repoPath || isPulling) return;
      isPulling = true;
      try {
          await GitService.pull(repoPath);
          await onGraphReload?.();
      } catch (e: any) {
          console.error("Pull failed", e);
          await confirm({ title: "Pull Failed", message: e.toString(), confirmLabel: "OK", cancelLabel: "Close" });
      } finally {
          isPulling = false;
      }
  }

  async function handlePush() {
      if (!repoPath || isPushing) return;
      isPushing = true;
      try {
          await GitService.push(repoPath);
          await onGraphReload?.();
      } catch (e: any) {
          console.error("Push failed", e);
          await confirm({ title: "Push Failed", message: e.toString(), confirmLabel: "OK", cancelLabel: "Close" });
      } finally {
          isPushing = false;
      }
  }
</script>

<div class="w-full h-full overflow-hidden flex bg-[#0d1117] font-sans">
  
  <!-- Main Graph Area -->
  <div class="flex-1 flex flex-col min-w-0 overflow-hidden relative">
        {#if leftPanelMode === 'diff'}
            <!-- Diff View Overlay -->
             <div class="absolute inset-0 z-20 flex flex-col bg-[#0d1117]">
             <DiffView
                 {diffResult}
                 {hunks}
                 loading={isLoadingDiff}
                 {isTooLarge}
                 {selectedEncoding}
                 onEncodingChange={handleEncodingChange}
             >
                {#snippet header(toolbarProps)}
                    <div class="{HEADER_BASE} px-2 justify-between">
                        <div class="flex items-center gap-2 overflow-hidden flex-1 mr-4">
                            <button 
                                class="text-xs text-[#8b949e] hover:text-[#c9d1d9] flex items-center gap-1 hover:bg-[#30363d] px-2 py-0.5 rounded transition-colors shrink-0"
                                onclick={closeDiff}
                            >
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="12" x2="5" y2="12"></line><polyline points="12 19 5 12 12 5"></polyline></svg>
                                Back to Graph
                            </button>
                            <div class="w-px h-3 bg-[#30363d] shrink-0"></div>
                            <span class="text-xs font-mono text-[#c9d1d9] truncate min-w-0" title={selectedDiffFile}>{selectedDiffFile}</span>
                        </div>

                        <!-- Diff Toolbar -->
                        <div class="shrink-0">
                                <DiffToolbar 
                                viewMode={toolbarProps.viewMode}
                                onViewModeChange={toolbarProps.onViewModeChange}
                                currentHunkIndex={toolbarProps.currentHunkIndex}
                                totalHunks={toolbarProps.totalHunks}
                                onPrevHunk={toolbarProps.onPrevHunk}
                                onNextHunk={toolbarProps.onNextHunk}
                                selectedEncoding={toolbarProps.selectedEncoding}
                                onEncodingChange={toolbarProps.onEncodingChange}
                                />
                        </div>
                    </div>
                {/snippet}
             </DiffView>
             </div>
        {/if}

        <!-- Toolbar / Menu -->
        <div class="{HEADER_BASE} px-2 relative">
            <button  
                onclick={() => showMenu = !showMenu}
                class="text-xs text-[#8b949e] hover:text-[#c9d1d9] px-2 py-1 rounded hover:bg-[#21262d] flex items-center gap-1 transition-colors"
            >
                <span>Columns ▾</span> 
            </button>

            <!-- Divider -->
            <div class="w-px h-4 bg-[#30363d] mx-2"></div>

            <!-- Actions -->
            <div class="flex items-center gap-1">
                <button 
                    class="text-xs text-[#8b949e] hover:text-white px-2 py-1 rounded hover:bg-[#21262d] flex items-center gap-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    onclick={handlePull}
                    disabled={!repoPath || isPulling}
                    title="Pull"
                >
                    {#if isPulling}
                        <svg class="animate-spin h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
                    {:else}
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M19 12l-7 7-7-7"/></svg>
                    {/if}
                    <span>Pull</span>
                </button>
                <button 
                    class="text-xs text-[#8b949e] hover:text-white px-2 py-1 rounded hover:bg-[#21262d] flex items-center gap-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    onclick={handlePush}
                    disabled={!repoPath || isPushing || pendingPushCount === 0}
                    title={pendingPushCount > 0 ? `Push ${pendingPushCount} commit(s)` : "Nothing to push"}
                >
                    {#if isPushing}
                        <svg class="animate-spin h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
                    {:else}
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19V5M5 12l7-7 7 7"/></svg>
                    {/if}
                    <span>Push</span>
                    {#if pendingPushCount > 0}
                        <span class="bg-[#1f6feb] text-white text-[10px] px-1.5 py-0.5 rounded-full font-bold ml-0.5" style="line-height: normal;">{pendingPushCount}</span>
                    {/if}
                </button>
                <button 
                    class="text-xs text-[#8b949e] hover:text-white px-2 py-1 rounded hover:bg-[#21262d] flex items-center gap-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    onclick={handleFetch}
                    disabled={!repoPath || isFetching}
                    title="Fetch"
                >
                    {#if isFetching}
                        <svg class="animate-spin h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
                    {:else}
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"></path><path d="M3 3v5h5"></path><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"></path><path d="M16 21h5v-5"></path></svg>
                    {/if}
                    <span>Fetch</span>
                </button>
            </div>

            {#if showMenu}
                <div class="absolute top-8 left-2 bg-[#1f2428] border border-[#30363d] rounded-md shadow-xl z-50 p-2 w-40 animate-in fade-in zoom-in-95 duration-100">
                    {#each columns as col}
                        <label class="flex items-center gap-2 p-1.5 hover:bg-[#282e33] rounded cursor-pointer text-xs text-[#c9d1d9]">
                            <input type="checkbox" bind:checked={col.visible} class="rounded border-[#30363d] bg-[#0d1117] text-[#238636] focus:ring-0">
                            {col.label}
                        </label>
                    {/each}
                </div>
                <!-- Backdrop to close -->
                <div class="fixed inset-0 z-40" onclick={() => showMenu = false} role="none"></div>
            {/if}
        </div>

        <!-- Header Row -->
        <div 
            class="border-b border-[#30363d] bg-[#161b22] select-none text-xs font-semibold text-[#8b949e] uppercase tracking-wider relative shrink-0"
            style="display: grid; grid-template-columns: {gridTemplate}; min-width: 100%;"
        >
            {#each visibleColumns as col (col.id)}
            <div 
                class="relative flex items-center px-4 h-8 group border-r border-[#30363d]/30"
            >
                {col.label}
                <!-- Resize Handle -->
                <div 
                    role="none"
                    class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-[#58a6ff] active:bg-[#58a6ff] z-10 opacity-0 group-hover:opacity-100 transition-opacity"
                    onmousedown={(e) => onMouseDown(e, col.id)}
                    ondblclick={() => /* reset? */ col.width = col.minWidth + 100 }
                ></div>
            </div>
            {/each}
        </div>

        <div class="flex-1 overflow-auto custom-scrollbar relative" bind:this={graphViewportEl} onscroll={hideCommitTooltip}>
            <div class="relative min-w-full" style="height: {nodes.length * ROW_HEIGHT + PADDING_TOP}px;">
                
                <!-- Graph SVG Layer -->
                <!-- Locked to the width of the 'graph' column if visible -->
                {#if graphColumn?.visible}
                    <div class="absolute top-0 left-0 h-full pointer-events-none z-[5] overflow-hidden" style="width: {graphColumn?.width}px">
                        <svg class="w-full h-full"> 
                        <g transform="translate({PADDING_LEFT}, {PADDING_TOP})">
                            <g class="edges">
                                {#each edgeGeometry as edge (edge.key)}
                                    <path
                                        class="graph-edge"
                                        d={edge.path}
                                        fill="none"
                                        stroke={edge.color}
                                        stroke-width={STROKE_WIDTH}
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        opacity={hoveredBranchColor && edge.color !== hoveredBranchColor ? 0.26 : 0.92}
                                    />
                                {/each}
                            </g>
                            <g class="nodes">
                                {#each nodes as node (node.hash)}
                                <circle 
                                    cx={nodeRenderX(node)} 
                                    cy={nodeRenderY(node)} 
                                    r={getNodeRadius(node)} 
                                    fill={node.color} 
                                    stroke={selectedCommit?.hash === node.hash ? "#f0f6fc" : "#0d1117"}
                                    stroke-width={selectedCommit?.hash === node.hash ? 4.5 : 4}
                                    aria-label={`Commit ${node.hash}`}
                                >
                                    <title>{`${node.hash} - ${node.subject}`}</title>
                                </circle>
                                {/each}
                            </g>
                        </g>
                        </svg>
                    </div>
                {/if}

                <!-- Rows Container -->
                <div class="absolute top-0 left-0 w-full pt-[8px] z-10">
                {#each nodes as node (node.hash)}
                    <div 
                        class="border-b border-[#30363d]/20 transition-colors text-xs items-center group cursor-pointer
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#58a6ff]/70 focus-visible:ring-inset"
                        style="display: grid; grid-template-columns: {gridTemplate}; height: {ROW_HEIGHT}px;"
                        onclick={() => selectCommit(node)}
                        onmouseenter={(e) => handleRowMouseEnter(e, node)}
                        onmousemove={handleRowMouseMove}
                        onmouseleave={handleRowMouseLeave}
                        onfocus={() => handleRowFocus(node)}
                        onblur={handleRowBlur}
                        role="button"
                        tabindex="0"
                        aria-label={`Select commit ${node.hash}: ${node.subject}`}
                        onkeydown={(e) => handleCommitRowKeydown(e, node)}
                    >
                        {#each visibleColumns as col}
                        {#if col.id === 'graph'}
                            <div class="pointer-events-none"><!-- Placeholder for SVG overlay --></div>
                        {:else if col.id === 'hash'}
                            <div class="pl-4 font-mono text-[#8b949e] opacity-70 truncate graph-row-info-cell {getRowCellHighlightClass(node.hash, col.id)}">{node.hash}</div>
                        {:else if col.id === 'message'}
                            <div class="pl-4 flex items-center min-w-0 pr-4 overflow-hidden graph-row-info-cell {getRowCellHighlightClass(node.hash, col.id)}">
                                <!-- Badges inside Message column -->
                                {#each getBadges(node.refs) as badge}
                                    <span 
                                    class="px-1.5 py-0.5 rounded text-[10px] font-medium mr-2 border shrink-0
                                        {badge.type === 'head' ? 'bg-sky-900/40 text-sky-300 border-sky-700/50' : 
                                        badge.type === 'tag' ? 'bg-yellow-900/40 text-yellow-300 border-yellow-700/50' : 
                                        badge.type === 'remote' ? 'bg-purple-900/40 text-purple-300 border-purple-700/50' :
                                        'bg-emerald-900/40 text-emerald-300 border-emerald-700/50'}"
                                    >
                                    {badge.text}
                                    </span>
                                {/each}
                                <span class="truncate text-[#c9d1d9] group-hover:text-white">{node.subject}</span>
                            </div>
                        {:else if col.id === 'author'}
                            <div class="pl-4 truncate text-[#c9d1d9] opacity-80 graph-row-info-cell {getRowCellHighlightClass(node.hash, col.id)}">{node.author}</div>
                        {:else if col.id === 'date'}
                            <div class="pl-4 text-[#8b949e] opacity-70 font-mono truncate graph-row-info-cell {getRowCellHighlightClass(node.hash, col.id)}">
                                {new Date(node.date).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute:'2-digit' })}
                            </div>
                        {/if}
                        {/each}
                    </div>
                {/each}
                </div>

            </div>
            <div class="absolute inset-0 z-50 pointer-events-none">
                {#if graphTooltip.visible}
                    <div
                        class="graph-tooltip absolute"
                        style="transform: translate({graphTooltip.x}px, {graphTooltip.y}px);"
                    >
                        <div class="graph-tooltip-subject">{graphTooltip.subject}</div>
                        <div class="graph-tooltip-hash">{graphTooltip.hash}</div>
                    </div>
                {/if}
            </div>
        </div>
  </div>

  <!-- Commit Details Panel -->
  {#if selectedCommit}
      <ResizablePanel side="left" initialSize={320} minSize={200} maxSize={600}>
          <div class="h-full flex flex-col bg-[#0d1117] border-l border-[#30363d]">
              <!-- Header -->
              <div class="{HEADER_BASE} justify-between px-2">
                  <span class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider">Commit Details</span>
                  <button class="text-[#8b949e] hover:text-white p-1 rounded" onclick={closeDetails} title="Close">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                  </button>
              </div>
              
              <div class="flex-1 overflow-auto p-3 custom-scrollbar">
                  <!-- Metadata -->
                  <div class="mb-4 space-y-2">
                       <div class="select-text font-mono text-[10px] text-[#8b949e] bg-[#161b22] p-1.5 rounded border border-[#30363d]">
                           {selectedCommit.hash}
                       </div>
                       
                       <div class="text-sm font-semibold text-[#c9d1d9] leading-tight select-text py-1">
                           {selectedCommit.subject}
                       </div>
                       
                       <div class="flex items-center gap-2 text-xs text-[#8b949e]">
                           <div class="flex items-center gap-1 overflow-hidden" title={selectedCommit.author}>
                               <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="4"/><path d="M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-3.92 7.94"/></svg>
                               <span class="truncate">{selectedCommit.author}</span>
                           </div>
                           <span>•</span>
                           <span title={selectedCommit.date}>
                               {new Date(selectedCommit.date).toLocaleString()}
                           </span>
                       </div>
                  </div>
                  
                  <!-- Changes -->
                  <div class="mt-4">
                      <div class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider mb-2 flex justify-between items-center">
                          <span>Changed Files</span>
                          {#if changedFiles.length > 0}
                              <span class="text-[10px] font-normal bg-[#30363d] text-[#c9d1d9] px-1.5 rounded-full">{changedFiles.length}</span>
                          {/if}
                      </div>
                      
                      {#if isLoadingFiles}
                          <div class="flex items-center justify-center py-8 text-[#8b949e] gap-2">
                              <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
                              <span class="text-xs">Loading changes...</span>
                          </div>
                      {:else if changedFiles.length > 0}
                          <div class="space-y-0.5">
                              {#each changedFiles as file}
                                  <div 
                                      class="flex items-start gap-2 py-1 px-1 hover:bg-[#161b22] rounded text-xs group cursor-pointer {selectedDiffFile === file ? 'bg-[#30363d] text-white' : ''}" 
                                      title={file}
                                      onclick={() => openDiff(file)}
                                      role="button"
                                      tabindex="0"
                                      onkeydown={(e) => e.key === 'Enter' && openDiff(file)}
                                  >
                                      <svg class="shrink-0 text-[#8b949e] w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>
                                      <span class="truncate text-[#c9d1d9] leading-tight break-all">
                                          {file}
                                      </span>
                                  </div>
                              {/each}
                          </div>
                      {:else}
                          <div class="text-xs text-[#8b949e] italic text-center py-4">No changes found</div>
                      {/if}
                  </div>
              </div>
          </div>
      </ResizablePanel>
  {/if}
</div>

<style>
  .graph-edge {
    transition: opacity 120ms ease;
    animation: graph-edge-fade-in 180ms ease-out both;
  }

  .graph-row-info-cell {
    transition: background-color 120ms ease;
  }

  .row-info-cell-hovered {
    background: rgba(26, 35, 52, 0.42);
  }

  .row-info-cell-selected {
    background: rgba(31, 111, 235, 0.18);
  }

  .graph-tooltip {
    max-width: 340px;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid rgba(88, 166, 255, 0.35);
    background: rgba(13, 17, 23, 0.95);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(3px);
  }

  .graph-tooltip-subject {
    color: #c9d1d9;
    font-size: 12px;
    line-height: 1.25;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .graph-tooltip-hash {
    margin-top: 4px;
    color: #8b949e;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
    font-size: 11px;
  }

  @keyframes graph-edge-fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

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
