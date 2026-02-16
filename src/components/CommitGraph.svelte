<script lang="ts">
  import { getAvatarUrl, type GraphNode, type LanePath, type ConnectionPath } from "../lib/graph-layout";
  import { onMount } from "svelte";
  import { GitService, type CommitChangedFile, type FileStatus } from "../lib/GitService";
  import { confirm } from "../lib/confirmation.svelte";
  import { prompt } from "../lib/prompt.svelte";
  import { toast } from "../lib/toast.svelte";
  import { executeWithFeedback, withToast } from "../lib/action-utils";
  import { GRAPH_CONFIG } from "../lib/graph-config";
  import {
      chooseBaseContent,
      buildCurvedConnectionPath,
      getTreePath,
      getBaseName,
      formatPathLabel
  } from "../lib/commit-graph-helpers";
  import ResizablePanel from "./resize/ResizablePanel.svelte";
  import { computeDiff, isLargeFile, extractHunks, type DiffResult, type DiffHunk } from "../lib/diff";
  import DiffView from "./diff/DiffView.svelte";
  import DiffToolbar from "./diff/DiffToolbar.svelte";
  import CommitContextMenu from "./common/CommitContextMenu.svelte";
  import StashCommitContextMenu from "./common/StashCommitContextMenu.svelte";
  import BranchContextMenu from "./common/BranchContextMenu.svelte";
  import type { BranchContextMenuState, BranchContextMenuAction } from "./common/branch-context-menu-types";
  import { rebaseStore } from "../lib/rebaseStore";
  import RebaseEditor from "./rebase/RebaseEditor.svelte";
  import RebaseProgress from "./rebase/RebaseProgress.svelte";
  import type {
      CommitContextMenuAction,
      CommitContextMenuState
  } from "./common/commit-context-menu-types";
  import type {
      StashCommitContextMenuAction,
      StashCommitContextMenuState
  } from "./common/stash-commit-context-menu-types";
  import FileChangeStatusBadge from "./common/FileChangeStatusBadge.svelte";
  import GraphWipPanel from "./commit/GraphWipPanel.svelte";

  interface Props {
    nodes?: GraphNode[];
    lanes?: LanePath[];
    connections?: ConnectionPath[];
    repoPath?: string;
    pendingPushCount?: number;
    onGraphReload?: () => Promise<void>;
    onLoadMoreCommits?: () => Promise<boolean>;
    hasMoreCommits?: boolean;
    isLoadingMoreCommits?: boolean;
    onShowHistory?: (filePath: string) => void;
    onShowBlame?: (filePath: string) => void;
  }

  let {
      nodes = [],
      lanes = [],
      connections = [],
      repoPath,
      pendingPushCount = 0,
      onGraphReload,
      onLoadMoreCommits,
      hasMoreCommits = true,
      isLoadingMoreCommits = false,
      onShowHistory,
      onShowBlame
  }: Props = $props();

  const {
      ROW_HEIGHT,
      COLUMN_WIDTH: COL_WIDTH,
      STROKE_WIDTH,
      PADDING_TOP,
      PADDING_LEFT,
      TOOLTIP_OFFSET_X,
      TOOLTIP_OFFSET_Y,
      TOOLTIP_MAX_WIDTH,
      TOOLTIP_MAX_HEIGHT,
      AVATAR_SIZE,
      STASH_AVATAR_CORNER_RADIUS,
      STASH_AVATAR_IMAGE_OPACITY,
      STASH_AVATAR_BASE_OPACITY,
      STASH_AVATAR_DASH,
      PATH_LABEL_MAX_LENGTH,
      PATH_COLLAPSE_TOKEN,
      CHANGED_FILE_CONTEXT_MENU_WIDTH,
      CHANGED_FILE_CONTEXT_MENU_ITEM_HEIGHT,
      CHANGED_FILE_CONTEXT_MENU_PADDING_Y
  } = GRAPH_CONFIG;
  const AVATAR_RADIUS = AVATAR_SIZE / 2;
  const SVG_INSTANCE_ID = `graph-${Math.random().toString(36).slice(2, 9)}`;
  const AVATAR_CLIP_ID = `${SVG_INSTANCE_ID}-avatar-clip`;
  const AVATAR_STASH_CLIP_ID = `${SVG_INSTANCE_ID}-avatar-stash-clip`;
  const AVATAR_SHADOW_ID = `${SVG_INSTANCE_ID}-avatar-shadow`;
  const CHANGED_FILES_VIEW_MODE_KEY = "commit_graph_changed_files_view_mode";
  const CHANGED_FILE_CONTEXT_MENU_SEPARATOR_HEIGHT = 9;
  const CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS = "context-menu-item";
  const HEADER_BASE = "panel-header";
  const LOAD_MORE_SCROLL_THRESHOLD_PX = 96;
  
  // -- State -- 
  interface Column {
      id: string;
      label: string;
      width: number;
      visible: boolean;
      minWidth: number;
  }

  let columns = $state<Column[]>([
      { id: "branch", label: "Branch / Tag", width: 170, visible: true, minWidth: 120 },
      { id: "graph", label: "Graph", width: 300, visible: true, minWidth: 100 },
      { id: "message", label: "Message", width: 400, visible: true, minWidth: 200 }, // Using numeric width to serve as flex basis concept if we were using flex, but for grid we can treat it as pixels or '1fr' logic if we get fancy. For now, pixel based resizing is robust.
      { id: "hash", label: "Hash", width: 80, visible: true, minWidth: 60 },
      { id: "author", label: "Author", width: 150, visible: true, minWidth: 80 },
      { id: "date", label: "Date", width: 140, visible: true, minWidth: 100 }
  ]);

  // Derived grid template
  let gridTemplate = $derived(columns.filter(c => c.visible).map(c => c.id === 'message' ? `minmax(${c.width}px, 1fr)` : `${c.width}px`).join(" "));
  let visibleColumns = $derived(columns.filter(c => c.visible));

  // Avatar cache: deterministic avatar URI per author, cached to avoid re-computation
  let avatarCache = $state<Map<string, string>>(new Map());
  let uniqueAuthors = $derived([...new Set(nodes.map(n => n.author))]);
  $effect(() => {
    let cacheChanged = false;
    for (const author of uniqueAuthors) {
      const nextUrl = getAvatarUrl(author);
      if (avatarCache.get(author) !== nextUrl) {
        avatarCache.set(author, nextUrl);
        cacheChanged = true;
      }
    }
    if (cacheChanged) {
      avatarCache = new Map(avatarCache);
    }
  });

  // Selection & Details
  let selectedCommit = $state<GraphNode | null>(null);
  let isWipRowSelected = $state(false);
  let changedFiles = $state<CommitChangedFile[]>([]);
  let isLoadingFiles = $state(false);
  let changedFilesViewMode = $state<"tree" | "path">("path");
  let changedFilesCollapsedDirs = $state<Set<string>>(new Set());

  type WipSummary = {
      files: CommitChangedFile[];
      stagedCount: number;
      unstagedCount: number;
      totalCount: number;
  };

  const EMPTY_WIP_SUMMARY: WipSummary = {
      files: [],
      stagedCount: 0,
      unstagedCount: 0,
      totalCount: 0
  };

  let wipSummary = $state<WipSummary>(EMPTY_WIP_SUMMARY);
  let hasWipRow = $state(true);

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

  function summarizeWorkingChanges(statusFiles: FileStatus[]): WipSummary {
      type AggregatedEntry = {
          path: string;
          stagedStatus: string | null;
          unstagedStatus: string | null;
      };

      const byPath = new Map<string, AggregatedEntry>();
      const stagedPaths = new Set<string>();
      const unstagedPaths = new Set<string>();

      for (const file of statusFiles) {
          const rawPath = file.path.trim();
          if (!rawPath) continue;
          const key = rawPath.toLowerCase();
          const existing = byPath.get(key) ?? {
              path: rawPath,
              stagedStatus: null,
              unstagedStatus: null
          };

          if (file.staged) {
              existing.stagedStatus = file.status;
              stagedPaths.add(key);
          } else {
              existing.unstagedStatus = file.status;
              unstagedPaths.add(key);
          }
          byPath.set(key, existing);
      }

      const files: CommitChangedFile[] = [...byPath.values()]
          .map((entry) => ({
              path: entry.path,
              status: entry.unstagedStatus ?? entry.stagedStatus ?? "M"
          }))
          .sort((a, b) => a.path.localeCompare(b.path));

      return {
          files,
          stagedCount: stagedPaths.size,
          unstagedCount: unstagedPaths.size,
          totalCount: files.length
      };
  }

  async function loadWipSummary(): Promise<void> {
      if (!repoPath) {
          wipSummary = EMPTY_WIP_SUMMARY;
          if (isWipRowSelected) {
              changedFiles = [];
          }
          return;
      }

      try {
          const statusFiles = await GitService.getStatusFiles(repoPath);
          const summary = summarizeWorkingChanges(statusFiles);
          wipSummary = summary;
          if (isWipRowSelected) {
              changedFiles = summary.files;
          }
      } catch (e) {
          console.error("Failed to load working changes for graph WIP row", e);
      }
  }

  function isSelectedCommit(node: GraphNode): boolean {
      return selectedCommit?.hash === node.hash && !isWipRowSelected;
  }

  function updateSelectedCommitState(node: GraphNode): void {
      selectedCommit = node;
      isWipRowSelected = false;
      changedFiles = [];
      changedFilesCollapsedDirs = new Set();
      closeChangedFileContextMenu();
      closeBranchContextMenu();
      closeDiff();
  }

  async function loadCommitDetails(node: GraphNode): Promise<void> {
      if (!repoPath) return;
      isLoadingFiles = true;
      const targetHash = node.hash;

      try {
          const files = await GitService.getCommitChangedFiles(node.hash, repoPath);
          if (selectedCommit?.hash === targetHash) changedFiles = files;
      } catch (e) {
          console.error("Failed to load commit files", e);
      } finally {
          if (selectedCommit?.hash === targetHash) isLoadingFiles = false;
      }
  }

  async function selectCommit(node: GraphNode) {
      if (isSelectedCommit(node)) return;
      updateSelectedCommitState(node);
      await loadCommitDetails(node);
  }

  export function selectWipRow() {
      isWipRowSelected = true;
      selectedCommit = null;
      changedFiles = wipSummary.files;
      changedFilesCollapsedDirs = new Set();
      closeChangedFileContextMenu();
      closeBranchContextMenu();
      closeCommitContextMenu();
      closeStashCommitContextMenu();
      closeDiff();
      void loadWipSummary();
  }

  export async function focusCommit(hash: string): Promise<boolean> {
      const normalized = hash.trim().toLowerCase();
      if (!normalized) return false;

      const target = nodes.find((node) => {
          const nodeHash = node.hash.toLowerCase();
          return nodeHash === normalized || nodeHash.startsWith(normalized);
      });
      if (!target) return false;

      await selectCommit(target);
      return true;
  }

  function closeDetails() {
      selectedCommit = null;
      isWipRowSelected = false;
      changedFiles = [];
      closeChangedFileContextMenu();
      closeDiff();
  }

  async function openDiff(file: string) {
      if (!selectedCommit || !repoPath) return;

      const targetCommitHash = selectedCommit.hash;
      const parentHashes = [...selectedCommit.parents];

      selectedDiffFile = file;
      showMenu = false;
      leftPanelMode = 'diff';
      isLoadingDiff = true;
      baseContent = "";
      modifiedContent = "";

      try {
          const [mod, parentContents] = await Promise.all([
              GitService.getFileAtCommit(targetCommitHash, file, repoPath, selectedEncoding)
                  .catch(() => ""), // Deleted file at selected commit
              Promise.all(
                  parentHashes.map((parentHash) =>
                      GitService.getFileAtCommit(parentHash, file, repoPath, selectedEncoding)
                          .catch(() => "") // Missing file at that parent
                  )
              )
          ]);

          if (selectedDiffFile !== file || selectedCommit?.hash !== targetCommitHash) return; // Race check

          modifiedContent = mod;
          // For merge commits, prefer the first parent that actually differs for this file.
          baseContent = chooseBaseContent(parentContents, mod);
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
          if (isWipRowSelected) {
              openWipDiff({ path: selectedDiffFile, status: "", staged: false });
          } else {
              openDiff(selectedDiffFile);
          }
      }
  }

  let wipPanelRef = $state<any>(null);

  async function openWipDiff(file: FileStatus) {
      if (!repoPath) return;
      selectedDiffFile = file.path;
      leftPanelMode = 'diff';
      isLoadingDiff = true;
      baseContent = "";
      modifiedContent = "";
      try {
          const [base, mod] = await Promise.all([
              GitService.getFileBaseContent(file.path, file.staged, repoPath, selectedEncoding),
              GitService.getFileModifiedContent(file.path, file.staged, repoPath, selectedEncoding),
          ]);
          if (selectedDiffFile !== file.path) return; // Race check
          modifiedContent = mod;
          baseContent = base;
      } catch (e) {
          console.error("Failed to load WIP diff", e);
      } finally {
          if (selectedDiffFile === file.path) {
              isLoadingDiff = false;
          }
      }
  }

  async function handleWipCommitSuccess() {
      closeDiff();
      closeDetails();
      await onGraphReload?.();
      await loadWipSummary();
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

      const changedFilesViewSaved = localStorage.getItem(CHANGED_FILES_VIEW_MODE_KEY);
      if (changedFilesViewSaved === "tree" || changedFilesViewSaved === "path") {
          changedFilesViewMode = changedFilesViewSaved;
      }
  });

  $effect(() => {
      localStorage.setItem("gh_table_columns", JSON.stringify(columns));
  });

  $effect(() => {
      localStorage.setItem(CHANGED_FILES_VIEW_MODE_KEY, changedFilesViewMode);
  });

  $effect(() => {
      repoPath;
      nodes;
      void loadWipSummary();
  });


  // -- Resizing Logic --
  let resizingColId = $state<string | null>(null);
  let startX = 0;
  let startWidth = 0;
  let resizeRaf: number | null = null;
  let pendingResizeX = 0;

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
      pendingResizeX = e.clientX;
      if (resizeRaf !== null) return;

      resizeRaf = requestAnimationFrame(() => {
          resizeRaf = null;
          if (!resizingColId) return;
          const idx = columns.findIndex(c => c.id === resizingColId);
          if (idx === -1) return;
          const diff = pendingResizeX - startX;
          columns[idx].width = Math.max(columns[idx].minWidth, startWidth + diff);
      });
  }

  function onMouseUp() {
      resizingColId = null;
      if (resizeRaf !== null) {
          cancelAnimationFrame(resizeRaf);
          resizeRaf = null;
      }
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
  let loadMoreRequestInFlight = $state(false);
  let branchContextMenu = $state<BranchContextMenuState | null>(null);
  let commitContextMenu = $state<CommitContextMenuState | null>(null);
  let stashCommitContextMenu = $state<StashCommitContextMenuState | null>(null);

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

  type PathGeometry = {
      key: string;
      color: string;
      path: string;
      lineStyle: "solid" | "dashed";
  };

  type ChangedFilesTreeDirectory = {
      name: string;
      path: string;
      children: Map<string, ChangedFilesTreeDirectory>;
      files: CommitChangedFile[];
  };

  type ChangedFilesDirectoryRow = {
      kind: "directory";
      key: string;
      depth: number;
      path: string;
      name: string;
      fileCount: number;
      collapsed: boolean;
  };

  type ChangedFilesFileRow = {
      kind: "file";
      key: string;
      depth: number;
      file: CommitChangedFile;
      label: string;
      title: string;
  };

  type ChangedFilesRow = ChangedFilesDirectoryRow | ChangedFilesFileRow;
  type ChangedFileContextMenuState = {
      visible: boolean;
      x: number;
      y: number;
      file: CommitChangedFile | null;
  };

  let changedFileContextMenu = $state<ChangedFileContextMenuState>({
      visible: false,
      x: 0,
      y: 0,
      file: null
  });

  let graphColumn = $derived(columns.find(c => c.id === "graph"));
  let graphColumnOffset = $derived.by(() => {
      let offset = 0;
      for (const column of visibleColumns) {
          if (column.id === "graph") break;
          offset += column.width;
      }
      return offset;
  });
  let graphRowOffset = $derived(hasWipRow ? 1 : 0);
  let totalRowCount = $derived(nodes.length + graphRowOffset);
  let rowIndexes = $derived(Array.from({ length: totalRowCount }, (_, index) => index));

  function columnToX(columnIndex: number) {
      return columnIndex * COL_WIDTH;
  }

  function rowToY(rowIndex: number) {
      return (rowIndex + graphRowOffset) * ROW_HEIGHT + ROW_HEIGHT / 2;
  }

  function nodeRenderX(node: GraphNode) {
      return columnToX(node.x);
  }

  function nodeRenderY(node: GraphNode) {
      return rowToY(node.y);
  }

  function isHeadCommit(node: GraphNode) {
      return node.refs.some((ref) => /^HEAD(\s*->|$)/.test(ref.trim()));
  }

  // Vertical lane paths — one continuous line per column span
  let laneGeometry = $derived.by<PathGeometry[]>(() =>
      lanes.map((lane, idx) => ({
          key: `lane-${lane.column}-${lane.rowStart}-${idx}`,
          color: lane.color,
          path: `M ${columnToX(lane.column)} ${rowToY(lane.rowStart)} V ${rowToY(lane.rowEnd)}`,
          lineStyle: "solid"
      }))
  );

  // Horizontal connection paths — L-shaped merge/fork lines with rounded corners
  let connectionGeometry = $derived.by<PathGeometry[]>(() =>
      connections.map((conn, idx) => {
          const x1 = columnToX(conn.fromColumn);
          const y1 = rowToY(conn.fromRow);
          const x2 = columnToX(conn.toColumn);
          const y2 = rowToY(conn.toRow);
          const turnAtStart = conn.parentIndex > 0;
          const path = buildCurvedConnectionPath(x1, y1, x2, y2, turnAtStart);

          return {
              key: `conn-${conn.fromColumn}-${conn.fromRow}-${conn.toColumn}-${conn.toRow}-${idx}`,
              color: conn.color,
              path,
              lineStyle: conn.lineStyle
          };
      })
  );

  let currentHeadNode = $derived(nodes.find((node) => isHeadCommit(node)) ?? null);
  let wipRowGraphX = $derived.by(() => {
      if (currentHeadNode) return columnToX(currentHeadNode.x);
      if (nodes.length > 0) return columnToX(nodes[0].x);
      return columnToX(0);
  });

  function getCurrentBranchFromRefs(refs: string[]): string | null {
      for (const rawRef of refs) {
          const ref = rawRef.trim();
          const match = ref.match(/^HEAD\s*->\s*(.+)$/);
          if (match?.[1]) return match[1].trim();
          if (ref === "HEAD") return "HEAD (detached)";
      }
      return null;
  }

  let currentBranchLabel = $derived.by(() => {
      if (!repoPath) return "No repository";
      if (currentHeadNode) {
          const branch = getCurrentBranchFromRefs(currentHeadNode.refs);
          if (branch) return branch;
      }
      if (nodes.length === 0) return "No commits";
      return "HEAD";
  });

  let currentBranchName = $derived.by(() => {
      if (!currentHeadNode) return "";
      const branch = getCurrentBranchFromRefs(currentHeadNode.refs);
      if (!branch || branch === "HEAD (detached)") return "";
      return branch;
  });

  let toolbarLocalBranches = $state<string[]>([]);
  let isToolbarBranchesLoading = $state(false);
  let isToolbarBranchSwitching = $state(false);

  async function loadToolbarBranches() {
      if (!repoPath) {
          toolbarLocalBranches = [];
          return;
      }
      isToolbarBranchesLoading = true;
      try {
          const localBranches = await GitService.getBranches(false, repoPath);
          toolbarLocalBranches = [...new Set(localBranches.map((b) => b.trim()).filter(Boolean))];
      } catch (e) {
          console.error("Failed to load local branches for toolbar", e);
          toolbarLocalBranches = [];
      } finally {
          isToolbarBranchesLoading = false;
      }
  }

  $effect(() => {
      if (!repoPath) {
          toolbarLocalBranches = [];
          return;
      }
      currentBranchLabel;
      void loadToolbarBranches();
  });

  async function handleToolbarBranchChange(event: Event) {
      const selectEl = event.currentTarget as HTMLSelectElement | null;
      const targetBranch = selectEl?.value?.trim() ?? "";
      if (!repoPath || !targetBranch || isToolbarBranchSwitching || targetBranch === currentBranchName) return;

      const confirmed = await confirm({
          title: "Confirm Checkout",
          message: `Switch to branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${targetBranch}</span>?`,
          isHtmlMessage: true,
          confirmLabel: "Checkout",
          cancelLabel: "Cancel"
      });
      if (!confirmed) {
          if (selectEl) {
              selectEl.value = currentBranchName || "";
          }
          return;
      }

      let switched = false;
      isToolbarBranchSwitching = true;
      try {
          const res = await GitService.switchBranch(targetBranch, repoPath);
          switched = res.success;
          if (res.success) {
              await onGraphReload?.();
              await loadToolbarBranches();
          }
      } catch (e) {
          console.error("Failed to switch branch from toolbar select", e);
      } finally {
          if (!switched && selectEl) {
              selectEl.value = currentBranchName || "";
          }
          isToolbarBranchSwitching = false;
      }
  }

  function handleCommitRowClick(node: GraphNode) {
      closeBranchContextMenu();
      closeCommitContextMenu();
      closeStashCommitContextMenu();
      closeChangedFileContextMenu();
      void selectCommit(node);
  }

  function handleCommitRowKeydown(event: KeyboardEvent, node: GraphNode) {
      if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          handleCommitRowClick(node);
      }
  }

  function handleWipRowKeydown(event: KeyboardEvent) {
      if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          selectWipRow();
      }
  }

  function navigateToCommitPanel() {
      selectWipRow();
  }

  function getRowCellHighlightClass(nodeHash: string, columnId: string): string {
      if (columnId === "graph") return "";
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

  async function maybeLoadMoreCommits(viewport: HTMLDivElement): Promise<void> {
      if (!onLoadMoreCommits || loadMoreRequestInFlight || isLoadingMoreCommits || !hasMoreCommits) {
          return;
      }

      const distanceToBottom = viewport.scrollHeight - (viewport.scrollTop + viewport.clientHeight);
      if (distanceToBottom > LOAD_MORE_SCROLL_THRESHOLD_PX) {
          return;
      }

      loadMoreRequestInFlight = true;
      try {
          await onLoadMoreCommits();
      } catch (e) {
          console.error("Failed to load older commits", e);
      } finally {
          loadMoreRequestInFlight = false;
      }
  }

  function handleGraphViewportScroll(event: Event) {
      hideCommitTooltip();
      const target = event.currentTarget;
      if (!(target instanceof HTMLDivElement)) return;
      void maybeLoadMoreCommits(target);
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

  function createEmptyDirectory(name: string, path: string): ChangedFilesTreeDirectory {
      return { name, path, children: new Map(), files: [] };
  }

  function ensureTreeChildDirectory(
      parent: ChangedFilesTreeDirectory,
      directoryName: string
  ): ChangedFilesTreeDirectory {
      const existing = parent.children.get(directoryName);
      if (existing) return existing;
      const path = parent.path ? `${parent.path}/${directoryName}` : directoryName;
      const created = createEmptyDirectory(directoryName, path);
      parent.children.set(directoryName, created);
      return created;
  }

  function insertFileIntoTree(root: ChangedFilesTreeDirectory, file: CommitChangedFile): void {
      const parts = getTreePath(file.path).split("/").filter(Boolean);
      if (parts.length <= 1) {
          root.files.push(file);
          return;
      }

      let current = root;
      for (const part of parts.slice(0, -1)) {
          current = ensureTreeChildDirectory(current, part);
      }
      current.files.push(file);
  }

  function buildChangedFilesTree(items: CommitChangedFile[]): ChangedFilesTreeDirectory {
      const root = createEmptyDirectory("", "");
      for (const file of items) insertFileIntoTree(root, file);
      return root;
  }

  function countChangedFiles(directory: ChangedFilesTreeDirectory): number {
      let count = directory.files.length;
      for (const child of directory.children.values()) {
          count += countChangedFiles(child);
      }
      return count;
  }

  function flattenChangedFilesTree(directory: ChangedFilesTreeDirectory, depth: number): ChangedFilesRow[] {
      const rows: ChangedFilesRow[] = [];

      const directories = [...directory.children.values()].sort((a, b) => a.name.localeCompare(b.name));
      for (const child of directories) {
          const collapsed = changedFilesCollapsedDirs.has(child.path);
          rows.push({
              kind: "directory",
              key: `dir:${child.path}`,
              depth,
              path: child.path,
              name: child.name,
              fileCount: countChangedFiles(child),
              collapsed
          });

          if (!collapsed) {
              rows.push(...flattenChangedFilesTree(child, depth + 1));
          }
      }

      const directoryFiles = [...directory.files].sort((a, b) => {
          const byName = getBaseName(a.path).localeCompare(getBaseName(b.path));
          if (byName !== 0) return byName;
          return getTreePath(a.path).localeCompare(getTreePath(b.path));
      });

      for (const file of directoryFiles) {
          rows.push({
              kind: "file",
              key: `file:${file.path}`,
              depth,
              file,
              label: getBaseName(file.path),
              title: file.path
          });
      }

      return rows;
  }

  let changedFileRows = $derived.by<ChangedFilesRow[]>(() => {
      if (changedFilesViewMode === "path") {
          return [...changedFiles]
              .sort((a, b) => a.path.localeCompare(b.path))
              .map((file) => ({
                  kind: "file" as const,
                  key: `file:${file.path}`,
                  depth: 0,
                  file,
                  label: formatPathLabel(file.path, PATH_LABEL_MAX_LENGTH, PATH_COLLAPSE_TOKEN),
                  title: file.path
              }));
      }

      const tree = buildChangedFilesTree(changedFiles);
      return flattenChangedFilesTree(tree, 0);
  });

  function toggleChangedFilesDirectory(path: string): void {
      closeChangedFileContextMenu();
      const next = new Set(changedFilesCollapsedDirs);
      if (next.has(path)) {
          next.delete(path);
      } else {
          next.add(path);
      }
      changedFilesCollapsedDirs = next;
  }

  function handleChangedFileKeydown(event: KeyboardEvent, filePath: string): void {
      if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          if (isWipRowSelected) return;
          closeChangedFileContextMenu();
          void openDiff(filePath);
      }
  }

  function resolveChangedFilePath(path: string): string {
      const normalized = path.replaceAll("\\", "/").trim();
      const renameParts = normalized.split(" -> ");
      return (renameParts[renameParts.length - 1] ?? normalized).trim();
  }

  function closeChangedFileContextMenu(): void {
      changedFileContextMenu = {
          visible: false,
          x: 0,
          y: 0,
          file: null
      };
  }

  function getChangedFileContextMenuPosition(clientX: number, clientY: number): { x: number; y: number } {
      const menuHeight = getChangedFileContextMenuHeight();
      const maxX = Math.max(8, window.innerWidth - CHANGED_FILE_CONTEXT_MENU_WIDTH - 8);
      const maxY = Math.max(8, window.innerHeight - menuHeight - 8);
      return {
          x: Math.min(Math.max(8, clientX), maxX),
          y: Math.min(Math.max(8, clientY), maxY)
      };
  }

  function getChangedFileContextMenuGroups(): number[] {
      const group1 = [!!onShowHistory, !!onShowBlame].filter(Boolean).length;
      const group2 = repoPath ? 4 : 0;
      const group3 = 1; // Copy file path
      const group4 = repoPath ? 2 : 0;
      return [group1, group2, group3, group4].filter((count) => count > 0);
  }

  function getChangedFileContextMenuHeight(): number {
      const groups = getChangedFileContextMenuGroups();
      const actionCount = groups.reduce((total, count) => total + count, 0);
      const separatorCount = Math.max(0, groups.length - 1);
      return (
          actionCount * CHANGED_FILE_CONTEXT_MENU_ITEM_HEIGHT +
          separatorCount * CHANGED_FILE_CONTEXT_MENU_SEPARATOR_HEIGHT +
          CHANGED_FILE_CONTEXT_MENU_PADDING_Y * 2
      );
  }

  function handleChangedFileContextMenu(event: MouseEvent, file: CommitChangedFile): void {
      event.preventDefault();
      event.stopPropagation();
      const pos = getChangedFileContextMenuPosition(event.clientX, event.clientY);
      changedFileContextMenu = {
          visible: true,
          x: pos.x,
          y: pos.y,
          file
      };
      branchContextMenu = null;
      commitContextMenu = null;
      stashCommitContextMenu = null;
  }

  function handleChangedFileShowHistory(): void {
      if (!changedFileContextMenu.file || !onShowHistory) return;
      const targetPath = resolveChangedFilePath(changedFileContextMenu.file.path);
      closeChangedFileContextMenu();
      onShowHistory(targetPath);
  }

  function handleChangedFileShowBlame(): void {
      if (!changedFileContextMenu.file || !onShowBlame) return;
      const targetPath = resolveChangedFilePath(changedFileContextMenu.file.path);
      closeChangedFileContextMenu();
      onShowBlame(targetPath);
  }

  async function handleOpenChangedFileInDiffTool(): Promise<void> {
      if (!repoPath || !changedFileContextMenu.file) return;
      const targetPath = changedFileContextMenu.file.path;
      closeChangedFileContextMenu();
      try {
          await GitService.openInDiffTool(targetPath, false, repoPath);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleOpenChangedFileInEditor(): Promise<void> {
      if (!repoPath || !changedFileContextMenu.file) return;
      const targetPath = changedFileContextMenu.file.path;
      closeChangedFileContextMenu();
      try {
          await GitService.openInEditor(targetPath, repoPath);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleOpenChangedFileInDefaultProgram(): Promise<void> {
      if (!repoPath || !changedFileContextMenu.file) return;
      const targetPath = changedFileContextMenu.file.path;
      closeChangedFileContextMenu();
      try {
          await GitService.openRepoFile(targetPath, repoPath);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleShowChangedFileInFolder(): Promise<void> {
      if (!repoPath || !changedFileContextMenu.file) return;
      const targetPath = changedFileContextMenu.file.path;
      closeChangedFileContextMenu();
      try {
          await GitService.showInFolder(targetPath, repoPath);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleCopyChangedFilePath(): Promise<void> {
      if (!changedFileContextMenu.file) return;
      const targetPath = resolveChangedFilePath(changedFileContextMenu.file.path);
      closeChangedFileContextMenu();

      try {
          await navigator.clipboard.writeText(targetPath);
          toast.success(`Copied path: ${targetPath}`);
      } catch (e) {
          console.error("Copy file path failed", e);
          toast.error("Copy file path failed");
      }
  }

  async function handleEditChangedFile(): Promise<void> {
      await handleOpenChangedFileInEditor();
  }

  async function handleDeleteChangedFile(): Promise<void> {
      if (!repoPath || !changedFileContextMenu.file) return;
      const targetPath = changedFileContextMenu.file.path;
      const displayPath = resolveChangedFilePath(targetPath);
      closeChangedFileContextMenu();

      const confirmed = await confirm({
          title: "Delete File",
          message: `Delete "${displayPath}" permanently?\nThis action cannot be undone.`,
          confirmLabel: "Delete",
          cancelLabel: "Cancel"
      });
      if (!confirmed) return;

      try {
          await GitService.deleteFile(targetPath, repoPath);
          if (isWipRowSelected) {
              await loadWipSummary();
          }
      } catch (e) {
          // toast handled in service
      }
  }

  function handleWindowMouseDown(event: MouseEvent): void {
      if (!changedFileContextMenu.visible) return;
      const target = event.target as Element | null;
      if (target?.closest(".changed-file-context-menu")) return;
      closeChangedFileContextMenu();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
      if (!changedFileContextMenu.visible) return;
      if (event.key === "Escape") {
          event.preventDefault();
          closeChangedFileContextMenu();
      }
  }

  type RefBadgeType = "branch" | "remote" | "tag";
  type RefBadge = {
      text: string;
      type: RefBadgeType;
      isCurrent: boolean;
      originalIndex: number;
  };

  function isStashRefBadge(ref: string): boolean {
      const normalized = ref.trim();
      return normalized === "refs/stash" || /^stash@\{\d+\}$/.test(normalized);
  }

  type RefBadgeCollector = { items: RefBadge[]; seen: Set<string> };

  function pushUniqueRefBadge(
      collector: RefBadgeCollector,
      text: string,
      type: RefBadgeType,
      isCurrent: boolean,
      originalIndex: number
  ): void {
      const normalized = text.trim();
      if (!normalized) return;
      const dedupeKey = `${type}:${normalized.toLowerCase()}`;
      if (collector.seen.has(dedupeKey)) return;
      collector.seen.add(dedupeKey);
      collector.items.push({ text: normalized, type, isCurrent, originalIndex });
  }

  function parseRefBadge(rawRef: string, index: number, collector: RefBadgeCollector): void {
      const ref = rawRef.trim();
      if (!ref || isStashRefBadge(ref)) return;
      if (ref.includes("HEAD ->")) {
          const current = ref.split("HEAD ->")[1]?.trim() ?? "";
          pushUniqueRefBadge(collector, current, "branch", true, index);
          return;
      }
      if (ref.startsWith("tag:")) {
          pushUniqueRefBadge(collector, ref.replace("tag:", "").trim(), "tag", false, index);
          return;
      }
      if (ref.includes("/")) {
          pushUniqueRefBadge(collector, ref, "remote", false, index);
          return;
      }
      pushUniqueRefBadge(collector, ref, "branch", false, index);
  }

  function filterDuplicatedRemoteBadges(badges: RefBadge[]): RefBadge[] {
      const localBranchNames = new Set(
          badges.filter((badge) => badge.type === "branch").map((badge) => badge.text.toLowerCase())
      );
      return badges.filter((badge) => {
          if (badge.type !== "remote") return true;
          const trackingName = badge.text.split("/").slice(1).join("/").trim().toLowerCase();
          return !trackingName || !localBranchNames.has(trackingName);
      });
  }

  function getRefBadgePriority(badge: RefBadge): number {
      if (badge.isCurrent) return 300;
      if (badge.type === "branch") return 200;
      if (badge.type === "remote") return 100;
      return 50;
  }

  function sortRefBadges(badges: RefBadge[]): RefBadge[] {
      return badges.sort((a, b) => {
          const byPriority = getRefBadgePriority(b) - getRefBadgePriority(a);
          if (byPriority !== 0) return byPriority;
          return a.originalIndex - b.originalIndex;
      });
  }

  // Parse refs and rank: current branch first, then local branches, remotes, tags.
  function getRankedRefBadges(refs: string[]): RefBadge[] {
      const collector: RefBadgeCollector = { items: [], seen: new Set() };
      refs.forEach((ref, index) => parseRefBadge(ref, index, collector));
      return sortRefBadges(filterDuplicatedRemoteBadges(collector.items));
  }

  function getRefBadgeClass(badge: RefBadge) {
      if (badge.isCurrent) {
          return "bg-sky-900/45 text-sky-300 border-sky-700/60";
      }
      if (badge.type === "branch") {
          return "bg-emerald-900/40 text-emerald-300 border-emerald-700/50";
      }
      if (badge.type === "remote") {
          return "bg-purple-900/40 text-purple-300 border-purple-700/50";
      }
      return "bg-yellow-900/40 text-yellow-300 border-yellow-700/50";
  }

  type ParsedCommitRefs = {
      isHead: boolean;
      localBranches: string[];
      remoteBranches: string[];
      tags: string[];
  };

  function parseCommitRefsForContextMenu(refs: string[]): ParsedCommitRefs {
      const localBranches: string[] = [];
      const remoteBranches: string[] = [];
      const tags: string[] = [];
      const localSeen = new Set<string>();
      const remoteSeen = new Set<string>();
      const tagSeen = new Set<string>();
      let isHead = false;

      function pushUnique(target: string[], seen: Set<string>, value: string) {
          const normalized = value.trim();
          if (!normalized) return;
          const key = normalized.toLowerCase();
          if (seen.has(key)) return;
          seen.add(key);
          target.push(normalized);
      }

      for (const rawRef of refs) {
          const ref = rawRef.trim();
          if (!ref || isStashRefBadge(ref)) continue;

          const headMatch = ref.match(/^HEAD\s*->\s*(.+)$/);
          if (headMatch?.[1]) {
              isHead = true;
              pushUnique(localBranches, localSeen, headMatch[1]);
              continue;
          }

          if (ref === "HEAD") {
              isHead = true;
              continue;
          }

          if (ref.startsWith("tag:")) {
              pushUnique(tags, tagSeen, ref.replace("tag:", ""));
              continue;
          }

          if (ref.includes("/")) {
              pushUnique(remoteBranches, remoteSeen, ref);
              continue;
          }

          pushUnique(localBranches, localSeen, ref);
      }

      return { isHead, localBranches, remoteBranches, tags };
  }

  function closeCommitContextMenu() {
      commitContextMenu = null;
  }

  function closeStashCommitContextMenu() {
      stashCommitContextMenu = null;
  }

  function handleCommitRowContextMenu(event: MouseEvent, node: GraphNode) {
      event.preventDefault();
      event.stopPropagation();

      closeChangedFileContextMenu();
      closeBranchContextMenu();
      closeCommitContextMenu();
      closeStashCommitContextMenu();

      if (node.isStash) {
          stashCommitContextMenu = {
              x: event.clientX,
              y: event.clientY,
              node
          };
          return;
      }

      const parsedRefs = parseCommitRefsForContextMenu(node.refs);

      commitContextMenu = {
          x: event.clientX,
          y: event.clientY,
          node,
          isHead: parsedRefs.isHead,
          currentBranch: currentBranchName,
          localBranches: parsedRefs.localBranches,
          remoteBranches: parsedRefs.remoteBranches,
          tags: parsedRefs.tags
      };
  }

  function parseRemoteRef(remoteRef: string): { remote: string; branch: string } | null {
      const normalized = remoteRef.trim();
      if (!normalized) return null;
      const slashIndex = normalized.indexOf("/");
      if (slashIndex <= 0 || slashIndex >= normalized.length - 1) return null;
      return {
          remote: normalized.slice(0, slashIndex),
          branch: normalized.slice(slashIndex + 1)
      };
  }

  function getPreferredUpstreamRef(menu: CommitContextMenuState): string {
      const current = menu.currentBranch.trim();
      if (current) {
          const match = menu.remoteBranches.find((remoteRef) => {
              const parsed = parseRemoteRef(remoteRef);
              return parsed?.branch === current;
          });
          if (match) return match;
          return `origin/${current}`;
      }
      return menu.remoteBranches[0] ?? "origin/main";
  }

  function getShortHash(hash: string): string {
      return hash.slice(0, 8);
  }

  async function reloadGraph(includeToolbar: boolean): Promise<void> {
      await onGraphReload?.();
      if (includeToolbar) {
          await loadToolbarBranches();
      }
  }

  async function runCommandWithReload(
      command: () => Promise<{ success: boolean }>,
      includeToolbar = true
  ): Promise<void> {
      const result = await command();
      if (result.success) {
          await reloadGraph(includeToolbar);
      }
  }

  async function runConfirmedCommand(
      confirmation: Parameters<typeof confirm>[0],
      command: () => Promise<{ success: boolean }>,
      includeToolbar = true
  ): Promise<void> {
      const result = await executeWithFeedback({
          confirmation,
          action: command,
          errorMessage: ""
      });
      if (result?.success) {
          await reloadGraph(includeToolbar);
      }
  }

  async function copyToClipboard(text: string, successMessage: string): Promise<void> {
      await withToast(() => navigator.clipboard.writeText(text), successMessage, "Copy failed");
  }

  async function handleSetUpstreamAction(menu: CommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const branchName = menu.currentBranch.trim();
      if (!branchName) {
          toast.error("Current branch is required to set upstream");
          return;
      }

      const upstreamInput = await prompt({
          title: "Set Upstream",
          message: `Set upstream for branch <code>${branchName}</code>:`,
          isHtmlMessage: true,
          placeholder: "origin/main",
          defaultValue: getPreferredUpstreamRef(menu),
          confirmLabel: "Set Upstream",
          cancelLabel: "Cancel"
      });
      const upstream = upstreamInput?.trim() ?? "";
      if (!upstream) return;
      await runCommandWithReload(() => GitService.setUpstream(branchName, upstream, repoPath));
  }

  async function handleCheckoutLocalAction(
      action: Extract<CommitContextMenuAction, { type: "checkout-local" }>
  ): Promise<void> {
      await checkoutFromBadge({
          text: action.branch,
          type: "branch",
          isCurrent: action.branch === currentBranchName,
          originalIndex: 0
      });
  }

  async function handleCheckoutRemoteAction(
      action: Extract<CommitContextMenuAction, { type: "checkout-remote" }>
  ): Promise<void> {
      await checkoutFromBadge({
          text: action.remoteRef,
          type: "remote",
          isCurrent: false,
          originalIndex: 0
      });
  }

  async function handleCheckoutDetachedAction(menu: CommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      await runConfirmedCommand({
          title: "Detached HEAD Checkout",
          message: `Checkout commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span> in detached HEAD state?`,
          isHtmlMessage: true,
          confirmLabel: "Checkout",
          cancelLabel: "Cancel"
      }, () => GitService.checkout(menu.node.hash, repoPath));
  }

  async function handleCreateBranchHereAction(menu: CommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      const branchInput = await prompt({
          title: "Create Branch Here",
          message: `Enter a branch name for commit <code>${shortHash}</code>:`,
          isHtmlMessage: true,
          placeholder: "feature/my-branch",
          confirmLabel: "Create",
          cancelLabel: "Cancel"
      });
      const branchName = branchInput?.trim() ?? "";
      if (!branchName) return;
      await runCommandWithReload(() => GitService.createBranch(branchName, menu.node.hash, repoPath));
  }

  function getResetConfirmation(
      mode: Extract<CommitContextMenuAction, { type: "reset" }>["mode"],
      shortHash: string
  ): Parameters<typeof confirm>[0] {
      if (mode === "hard") {
          return {
              title: "Hard Reset",
              message: `Hard reset to <code>${shortHash}</code>?<br/><br/><strong class="text-[#f85149]">Warning: all uncommitted changes will be permanently lost.</strong>`,
              isHtmlMessage: true,
              confirmLabel: "Hard Reset",
              cancelLabel: "Cancel"
          };
      }

      const resetLabel = mode.toUpperCase();
      return {
          title: `${resetLabel} Reset`,
          message: `${resetLabel} reset to commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?`,
          isHtmlMessage: true,
          confirmLabel: "Reset",
          cancelLabel: "Cancel"
      };
  }

  async function handleResetAction(
      action: Extract<CommitContextMenuAction, { type: "reset" }>,
      menu: CommitContextMenuState
  ): Promise<void> {
      if (!repoPath) return;
      const confirmation = getResetConfirmation(action.mode, getShortHash(menu.node.hash));
      await runConfirmedCommand(confirmation, () => GitService.resetToCommit(menu.node.hash, action.mode, repoPath));
  }

  async function handleRevertAction(menu: CommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      await runConfirmedCommand({
          title: "Revert Commit",
          message: `Revert commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?<br/><em>"${menu.node.subject}"</em><br/><br/>This creates a new commit that undoes this commit.`,
          isHtmlMessage: true,
          confirmLabel: "Revert",
          cancelLabel: "Cancel"
      }, () => GitService.revertCommit(menu.node.hash, repoPath));
  }

  async function handleRenameBranchAction(
      action: Extract<CommitContextMenuAction, { type: "rename-branch" }>
  ): Promise<void> {
      if (!repoPath) return;
      const oldName = action.branch.trim();
      if (!oldName) return;

      const newNameInput = await prompt({
          title: "Rename Branch",
          message: `Rename branch <code>${oldName}</code> to:`,
          isHtmlMessage: true,
          defaultValue: oldName,
          placeholder: "new-branch-name",
          confirmLabel: "Rename",
          cancelLabel: "Cancel"
      });
      const newName = newNameInput?.trim() ?? "";
      if (!newName || newName === oldName) return;
      await runCommandWithReload(() => GitService.renameBranch(oldName, newName, repoPath));
  }

  async function handleDeleteLocalBranchAction(
      action: Extract<CommitContextMenuAction, { type: "delete-local-branch" }>
  ): Promise<void> {
      if (!repoPath) return;
      const targetBranch = action.branch.trim();
      if (!targetBranch) return;

      await runConfirmedCommand({
          title: "Delete Local Branch",
          message: `Delete local branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${targetBranch}</span>?`,
          isHtmlMessage: true,
          confirmLabel: "Delete",
          cancelLabel: "Cancel"
      }, () => GitService.deleteBranch(targetBranch, false, repoPath));
  }

  async function handleDeleteRemoteBranchAction(
      action: Extract<CommitContextMenuAction, { type: "delete-remote-branch" }>
  ): Promise<void> {
      if (!repoPath) return;
      const parsed = parseRemoteRef(action.remoteRef);
      if (!parsed) {
          toast.error("Invalid remote branch reference");
          return;
      }

      await runConfirmedCommand({
          title: "Delete Remote Branch",
          message: `Delete remote branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${action.remoteRef}</span>?`,
          isHtmlMessage: true,
          confirmLabel: "Delete",
          cancelLabel: "Cancel"
      }, () => GitService.deleteRemoteBranch(parsed.remote, parsed.branch, repoPath));
  }

  async function handleDeleteLocalAndRemoteAction(
      action: Extract<CommitContextMenuAction, { type: "delete-local-and-remote" }>
  ): Promise<void> {
      if (!repoPath) return;
      const parsed = parseRemoteRef(action.remoteRef);
      if (!parsed) {
          toast.error("Invalid remote branch reference");
          return;
      }

      const confirmation = {
          title: "Delete Local and Remote Branch",
          message: `Delete local branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${action.branch}</span> and remote branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${action.remoteRef}</span>?`,
          isHtmlMessage: true,
          confirmLabel: "Delete Both",
          cancelLabel: "Cancel"
      };
      const result = await executeWithFeedback({
          confirmation,
          action: async () => {
              const remoteRes = await GitService.deleteRemoteBranch(parsed.remote, parsed.branch, repoPath);
              if (!remoteRes.success) return remoteRes;
              return GitService.deleteBranch(action.branch, false, repoPath);
          },
          errorMessage: ""
      });
      if (result?.success) await reloadGraph(true);
  }

  async function handleCopyCommitShaAction(menu: CommitContextMenuState): Promise<void> {
      await copyToClipboard(menu.node.hash, `Copied: ${getShortHash(menu.node.hash)}`);
  }

  async function handleCopyBranchNameAction(
      action: Extract<CommitContextMenuAction, { type: "copy-branch-name" }>
  ): Promise<void> {
      await copyToClipboard(action.branch, `Copied: ${action.branch}`);
  }

  async function handleCreatePatchFromCommitAction(menu: CommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const patch = await GitService.createPatchFromCommit(menu.node.hash, repoPath);
      if (!patch.trim()) {
          toast.error("No patch content available for this commit");
          return;
      }
      await copyToClipboard(patch, `Copied patch for ${getShortHash(menu.node.hash)}`);
  }

  async function handleCreateTagAction(
      action: Extract<CommitContextMenuAction, { type: "create-tag" }>,
      menu: CommitContextMenuState
  ): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      const tagNameInput = await prompt({
          title: action.annotated ? "Create Annotated Tag" : "Create Tag",
          message: `Tag name for commit <code>${shortHash}</code>:`,
          isHtmlMessage: true,
          placeholder: "v1.0.0",
          confirmLabel: "Create",
          cancelLabel: "Cancel"
      });
      const tagName = tagNameInput?.trim() ?? "";
      if (!tagName) return;

      let tagMessage: string | undefined = undefined;
      if (action.annotated) {
          const messageInput = await prompt({
              title: "Annotated Tag Message",
              message: "Enter annotation message:",
              placeholder: "Release notes",
              confirmLabel: "Continue",
              cancelLabel: "Cancel"
          });
          const trimmedMessage = messageInput?.trim() ?? "";
          if (!trimmedMessage) return;
          tagMessage = trimmedMessage;
      }
      await runCommandWithReload(() => GitService.createTag(tagName, menu.node.hash, tagMessage, repoPath));
  }

  type CommitActionRunner = (action: CommitContextMenuAction, menu: CommitContextMenuState) => Promise<void>;

  const commitContextActionHandlers: Record<CommitContextMenuAction["type"], CommitActionRunner> = {
      "pull": async () => handlePull(),
      "push": async () => handlePush(),
      "fetch": async () => handleFetch(),
      "set-upstream": async (_, menu) => handleSetUpstreamAction(menu),
      "checkout-local": async (action) => handleCheckoutLocalAction(action as Extract<CommitContextMenuAction, { type: "checkout-local" }>),
      "checkout-remote": async (action) => handleCheckoutRemoteAction(action as Extract<CommitContextMenuAction, { type: "checkout-remote" }>),
      "checkout-detached": async (_, menu) => handleCheckoutDetachedAction(menu),
      "create-branch-here": async (_, menu) => handleCreateBranchHereAction(menu),
      "reset": async (action, menu) => handleResetAction(action as Extract<CommitContextMenuAction, { type: "reset" }>, menu),
      "rebase": async (_, menu) => {
          if (!repoPath) return;
          const confirmed = await confirm({
              title: "Rebase",
              message: `Rebase <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${menu.currentBranch || "current"}</span> onto commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${getShortHash(menu.node.hash)}</span>?`,
              isHtmlMessage: true,
              confirmLabel: "Rebase",
              cancelLabel: "Cancel"
          });
          if (!confirmed) return;
          await rebaseStore.startRebase(menu.node.hash, repoPath);
          await onGraphReload?.();
      },
      "interactive-rebase": async (_, menu) => {
          if (!repoPath) return;
          await rebaseStore.prepareInteractive(menu.node.hash, repoPath);
      },
      "revert": async (_, menu) => handleRevertAction(menu),
      "rename-branch": async (action) => handleRenameBranchAction(action as Extract<CommitContextMenuAction, { type: "rename-branch" }>),
      "delete-local-branch": async (action) => handleDeleteLocalBranchAction(action as Extract<CommitContextMenuAction, { type: "delete-local-branch" }>),
      "delete-remote-branch": async (action) => handleDeleteRemoteBranchAction(action as Extract<CommitContextMenuAction, { type: "delete-remote-branch" }>),
      "delete-local-and-remote": async (action) => handleDeleteLocalAndRemoteAction(action as Extract<CommitContextMenuAction, { type: "delete-local-and-remote" }>),
      "copy-commit-sha": async (_, menu) => handleCopyCommitShaAction(menu),
      "copy-branch-name": async (action) => handleCopyBranchNameAction(action as Extract<CommitContextMenuAction, { type: "copy-branch-name" }>),
      "create-patch-from-commit": async (_, menu) => handleCreatePatchFromCommitAction(menu),
      "create-tag": async (action, menu) => handleCreateTagAction(action as Extract<CommitContextMenuAction, { type: "create-tag" }>, menu)
  };

  async function handleCommitContextAction(action: CommitContextMenuAction, menu: CommitContextMenuState) {
      const handler = commitContextActionHandlers[action.type];
      if (!handler) return;
      try {
          await handler(action, menu);
      } catch (e) {
          console.error("Commit context menu action failed", e);
      }
  }

  async function showStashConflictMessage(message: string): Promise<void> {
      if (!repoPath) return;
      const hasConflicts = await GitService.checkConflictState(repoPath).catch(() => false);
      if (hasConflicts) {
          toast.error(message);
      }
  }

  async function handleApplyStashAction(menu: StashCommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const res = await GitService.applyStash(menu.node.hash, repoPath);
      if (!res.success) return;
      await showStashConflictMessage("Stash applied with conflicts. Open the Conflicts tab to resolve.");
  }

  async function handlePopStashAction(menu: StashCommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      const confirmation = {
          title: "Pop Stash",
          message: `Pop stash <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?<br/><br/>This applies changes and removes the stash entry.`,
          isHtmlMessage: true,
          confirmLabel: "Pop Stash",
          cancelLabel: "Cancel"
      };
      const res = await executeWithFeedback({
          confirmation,
          action: () => GitService.popStash(menu.node.hash, repoPath),
          errorMessage: ""
      });
      if (!res?.success) return;
      await reloadGraph(false);
      await showStashConflictMessage("Stash popped with conflicts. Open the Conflicts tab to resolve.");
  }

  async function handleDeleteStashAction(menu: StashCommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      await runConfirmedCommand({
          title: "Delete Stash",
          message: `Delete stash <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?`,
          isHtmlMessage: true,
          confirmLabel: "Delete Stash",
          cancelLabel: "Cancel"
      }, () => GitService.deleteStash(menu.node.hash, repoPath), false);
  }

  async function handleEditStashMessageAction(menu: StashCommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      const messageInput = await prompt({
          title: "Edit stash message",
          message: `Update message for stash <code>${shortHash}</code>:`,
          isHtmlMessage: true,
          defaultValue: menu.node.subject,
          placeholder: "Stash message",
          confirmLabel: "Save",
          cancelLabel: "Cancel"
      });
      const newMessage = messageInput?.trim() ?? "";
      if (!newMessage) return;
      await runCommandWithReload(() => GitService.editStashMessage(menu.node.hash, newMessage, repoPath), false);
  }

  async function handleShareStashCloudPatchAction(menu: StashCommitContextMenuState): Promise<void> {
      if (!repoPath) return;
      const shortHash = getShortHash(menu.node.hash);
      const patch = await GitService.createPatchFromStash(menu.node.hash, repoPath);
      if (!patch.trim()) {
          toast.error("No patch content available for this stash");
          return;
      }
      await copyToClipboard(patch, `Cloud Patch not configured, copied stash patch ${shortHash} instead`);
  }

  type StashActionRunner = (
      action: StashCommitContextMenuAction,
      menu: StashCommitContextMenuState
  ) => Promise<void>;

  const stashContextActionHandlers: Record<StashCommitContextMenuAction["type"], StashActionRunner> = {
      "apply-stash": async (_, menu) => handleApplyStashAction(menu),
      "pop-stash": async (_, menu) => handlePopStashAction(menu),
      "delete-stash": async (_, menu) => handleDeleteStashAction(menu),
      "edit-stash-message": async (_, menu) => handleEditStashMessageAction(menu),
      "share-stash-cloud-patch": async (_, menu) => handleShareStashCloudPatchAction(menu),
      "hide": async () => {}
  };

  async function handleStashCommitContextAction(
      action: StashCommitContextMenuAction,
      menu: StashCommitContextMenuState
  ) {
      const handler = stashContextActionHandlers[action.type];
      if (!handler) return;
      try {
          await handler(action, menu);
      } catch (e) {
          console.error("Stash context menu action failed", e);
      }
  }

  let isBranchCheckoutLoading = $state(false);

  function canCheckoutFromBadge(badge: RefBadge): boolean {
      return !badge.isCurrent && (badge.type === "branch" || badge.type === "remote");
  }

  function getTrackingBranchName(remoteRef: string): string {
      return remoteRef.split("/").slice(1).join("/").trim();
  }

  function isCurrentBranchBadge(badge: RefBadge): boolean {
      if (badge.isCurrent) return true;
      if (badge.type === "branch") return badge.text === currentBranchName;
      if (badge.type === "remote") {
          const localBranch = getTrackingBranchName(badge.text);
          return !!localBranch && localBranch === currentBranchName;
      }
      return false;
  }

  function closeBranchContextMenu() {
      branchContextMenu = null;
  }

  function handleBranchBadgeContextMenu(event: MouseEvent, badge: RefBadge, node: GraphNode) {
      event.preventDefault();
      event.stopPropagation();
      if (!canCheckoutFromBadge(badge)) return;
      closeChangedFileContextMenu();
      closeCommitContextMenu();
      closeStashCommitContextMenu();
      branchContextMenu = {
          x: event.clientX,
          y: event.clientY,
          branchName: badge.text,
          branchType: badge.type === "remote" ? "remote" : "branch",
          commitHash: node.hash,
          currentBranch: currentBranchName || "",
          node
      };
  }

  async function checkoutFromBadge(badge: RefBadge) {
      if (!repoPath || isBranchCheckoutLoading || !canCheckoutFromBadge(badge)) return;

      isBranchCheckoutLoading = true;
      try {
          if (badge.type === "branch") {
              const confirmed = await confirm({
                  title: "Confirm Checkout",
                  message: `Switch to branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${badge.text}</span>?`,
                  isHtmlMessage: true,
                  confirmLabel: "Checkout",
                  cancelLabel: "Cancel"
              });
              if (!confirmed) return;

              const res = await GitService.switchBranch(badge.text, repoPath);
              if (res.success) {
                  await onGraphReload?.();
                  await loadToolbarBranches();
              }
              return;
          }

          const localBranch = getTrackingBranchName(badge.text);
          if (!localBranch) return;

          const localBranches = await GitService.getBranches(false, repoPath);
          if (localBranches.includes(localBranch)) {
              const confirmed = await confirm({
                  title: "Confirm Checkout",
                  message: `A local branch named <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${localBranch}</span> exists.<br/><br/>Switch to this local branch?`,
                  isHtmlMessage: true,
                  confirmLabel: "Checkout",
                  cancelLabel: "Cancel"
              });
              if (!confirmed) return;

              const res = await GitService.switchBranch(localBranch, repoPath);
              if (res.success) {
                  await onGraphReload?.();
                  await loadToolbarBranches();
              }
              return;
          }

          const confirmed = await confirm({
              title: "Confirm Checkout",
              message: `Checkout remote branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${badge.text}</span>?<br/><span class="text-xs text-gray-500">A new local tracking branch <span class="font-mono">${localBranch}</span> will be created.</span>`,
              isHtmlMessage: true,
              confirmLabel: "Checkout",
              cancelLabel: "Cancel"
          });
          if (!confirmed) return;

          const res = await GitService.checkoutNew(localBranch, badge.text, repoPath);
          if (res.success) {
              await onGraphReload?.();
              await loadToolbarBranches();
          }
      } catch (e) {
          console.error("Failed to checkout from graph branch badge", e);
      } finally {
          isBranchCheckoutLoading = false;
      }
  }

  async function handleBranchBadgeClick(event: MouseEvent, badge: RefBadge) {
      event.preventDefault();
      event.stopPropagation();
      await checkoutFromBadge(badge);
  }

  async function mergeFromBadge(badge: RefBadge) {
      if (!repoPath || isBranchCheckoutLoading || !canCheckoutFromBadge(badge)) return;

      const branchRef = badge.text.trim();
      if (!branchRef) return;

      if (isCurrentBranchBadge(badge)) return;

      const confirmed = await confirm({
          title: "Merge Branch",
          message: `Merge branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${branchRef}</span> into <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${currentBranchName || currentBranchLabel}</span>?`,
          isHtmlMessage: true,
          confirmLabel: "Merge",
          cancelLabel: "Cancel"
      });
      if (!confirmed) return;

      function toErrorText(error: unknown): string {
          if (error instanceof Error) {
              return error.message;
          }
          return String(error ?? "");
      }

      function isMergeOrRebaseInProgressMessage(message: string): boolean {
          const normalized = message.toLowerCase();
          return (
              normalized.includes("merge_head exists") ||
              normalized.includes("not concluded your merge") ||
              normalized.includes("you have unmerged paths") ||
              normalized.includes("conflict") ||
              normalized.includes("rebase") ||
              normalized.includes("cherry-pick")
          );
      }

      async function navigateToCommitWhenMergeBlocked(message: string): Promise<boolean> {
          const hasConflicts = await GitService.checkConflictState(repoPath).catch(() => false);
          if (hasConflicts || isMergeOrRebaseInProgressMessage(message)) {
              selectWipRow();
              await loadWipSummary();
              return true;
          }
          return false;
      }

      isBranchCheckoutLoading = true;
      try {
          const res = await GitService.merge(branchRef, repoPath);
          if (res.success) {
              await onGraphReload?.();
              await loadToolbarBranches();
              return;
          }

          const mergedErrorText = res.stderr || res.stdout || "";
          await navigateToCommitWhenMergeBlocked(mergedErrorText);
      } catch (e) {
          console.error("Failed to merge from graph branch badge", e);
          const errorText = toErrorText(e);
          await navigateToCommitWhenMergeBlocked(errorText);
      } finally {
          isBranchCheckoutLoading = false;
      }
  }

  async function handleBranchContextAction(action: BranchContextMenuAction, menu: BranchContextMenuState) {
      if (!repoPath) return;
      try {
          switch (action.type) {
              case "checkout": {
                  const badge: RefBadge = { text: menu.branchName, type: menu.branchType, isCurrent: false, originalIndex: 0 };
                  await checkoutFromBadge(badge);
                  break;
              }
              case "merge": {
                  const badge: RefBadge = { text: menu.branchName, type: menu.branchType, isCurrent: false, originalIndex: 0 };
                  await mergeFromBadge(badge);
                  break;
              }
              case "rebase": {
                  const confirmed = await confirm({
                      title: "Rebase",
                      message: `Rebase <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${menu.currentBranch || "current"}</span> onto <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${menu.branchName}</span>?`,
                      isHtmlMessage: true,
                      confirmLabel: "Rebase",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;
                  await rebaseStore.startRebase(menu.branchName, repoPath);
                  await onGraphReload?.();
                  break;
              }
              case "interactive-rebase": {
                   await rebaseStore.prepareInteractive(menu.commitHash, repoPath);
                   break;
              }
              case "cherry-pick": {
                  const shortHash = getShortHash(menu.commitHash);
                  const confirmed = await confirm({
                      title: "Cherry Pick",
                      message: `Cherry-pick commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?`,
                      isHtmlMessage: true,
                      confirmLabel: "Cherry Pick",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;
                  await runCommandWithReload(() => GitService.cherryPick(menu.commitHash, repoPath));
                  break;
              }
              case "create-branch-here": {
                  const shortHash = getShortHash(menu.commitHash);
                  const branchInput = await prompt({
                      title: "Create Branch Here",
                      message: `Enter a branch name for commit <code>${shortHash}</code>:`,
                      isHtmlMessage: true,
                      placeholder: "feature/my-branch",
                      confirmLabel: "Create",
                      cancelLabel: "Cancel"
                  });
                  const branchName = branchInput?.trim() ?? "";
                  if (!branchName) return;
                  await runCommandWithReload(() => GitService.createBranch(branchName, menu.commitHash, repoPath));
                  break;
              }
              case "reset": {
                  const shortHash = getShortHash(menu.commitHash);
                  const confirmation = getResetConfirmation(action.mode, shortHash);
                  await runConfirmedCommand(confirmation, () => GitService.resetToCommit(menu.commitHash, action.mode, repoPath));
                  break;
              }
              case "revert": {
                  const shortHash = getShortHash(menu.commitHash);
                  await runConfirmedCommand({
                      title: "Revert Commit",
                      message: `Revert commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?<br/><em>"${menu.node.subject}"</em><br/><br/>This creates a new commit that undoes this commit.`,
                      isHtmlMessage: true,
                      confirmLabel: "Revert",
                      cancelLabel: "Cancel"
                  }, () => GitService.revertCommit(menu.commitHash, repoPath));
                  break;
              }
              case "delete": {
                  if (menu.branchType === "remote") {
                      const parsed = parseRemoteRef(menu.branchName);
                      if (!parsed) {
                          toast.error("Invalid remote branch reference");
                          return;
                      }
                      await runConfirmedCommand({
                          title: "Delete Remote Branch",
                          message: `Delete remote branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${menu.branchName}</span>?`,
                          isHtmlMessage: true,
                          confirmLabel: "Delete",
                          cancelLabel: "Cancel"
                      }, () => GitService.deleteRemoteBranch(parsed.remote, parsed.branch, repoPath));
                  } else {
                      await runConfirmedCommand({
                          title: "Delete Local Branch",
                          message: `Delete local branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${menu.branchName}</span>?`,
                          isHtmlMessage: true,
                          confirmLabel: "Delete",
                          cancelLabel: "Cancel"
                      }, () => GitService.deleteBranch(menu.branchName, false, repoPath));
                  }
                  break;
              }
              case "copy-branch-name":
                  await copyToClipboard(menu.branchName, `Copied: ${menu.branchName}`);
                  break;
              case "copy-commit-sha":
                  await copyToClipboard(menu.commitHash, `Copied: ${getShortHash(menu.commitHash)}`);
                  break;
              case "create-tag": {
                  const shortHash = getShortHash(menu.commitHash);
                  const tagNameInput = await prompt({
                      title: action.annotated ? "Create Annotated Tag" : "Create Tag",
                      message: `Tag name for commit <code>${shortHash}</code>:`,
                      isHtmlMessage: true,
                      placeholder: "v1.0.0",
                      confirmLabel: "Create",
                      cancelLabel: "Cancel"
                  });
                  const tagName = tagNameInput?.trim() ?? "";
                  if (!tagName) return;

                  let tagMessage: string | undefined = undefined;
                  if (action.annotated) {
                      const messageInput = await prompt({
                          title: "Annotated Tag Message",
                          message: "Enter annotation message:",
                          placeholder: "Release notes",
                          confirmLabel: "Continue",
                          cancelLabel: "Cancel"
                      });
                      const trimmedMessage = messageInput?.trim() ?? "";
                      if (!trimmedMessage) return;
                      tagMessage = trimmedMessage;
                  }
                  await runCommandWithReload(() => GitService.createTag(tagName, menu.commitHash, tagMessage, repoPath));
                  break;
              }
          }
      } catch (e) {
          console.error("Branch context menu action failed", e);
      }
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

<svelte:window onmousedown={handleWindowMouseDown} onkeydown={handleWindowKeydown} />

<div class="w-full h-full overflow-hidden flex bg-[#0f172a] font-sans">
  
  <!-- Main Graph Area -->
  <div class="flex-1 flex flex-col min-w-0 overflow-hidden relative">
        {#if leftPanelMode === 'diff'}
            <!-- Diff View Overlay -->
             <div class="absolute inset-0 z-20 flex flex-col bg-[#0f172a]">
             <DiffView
                 {diffResult}
                 {hunks}
                 loading={isLoadingDiff}
                 {isTooLarge}
                 {selectedEncoding}
                 onEncodingChange={handleEncodingChange}
             >
                {#snippet header(toolbarProps)}
                    <div class="{HEADER_BASE} px-2 justify-between gap-2 flex-wrap">
                        <div class="flex items-center gap-2 overflow-hidden flex-1 mr-4 max-[900px]:mr-0 max-[900px]:w-full">
                            <button 
                                class="text-xs text-[#8b949e] hover:text-[#c9d1d9] flex items-center gap-1 hover:bg-[#1e293b] px-2 py-0.5 rounded transition-colors shrink-0"
                                onclick={closeDiff}
                            >
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="12" x2="5" y2="12"></line><polyline points="12 19 5 12 12 5"></polyline></svg>
                                Back to Graph
                            </button>
                            <div class="w-px h-3 bg-[#1e293b] shrink-0"></div>
                            <span class="text-xs font-mono text-[#c9d1d9] truncate min-w-0" title={selectedDiffFile}>{selectedDiffFile}</span>
                        </div>

                        <!-- Diff Toolbar -->
                        <div class="shrink-0 max-[900px]:w-full">
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
        <div class="{HEADER_BASE} px-2 relative justify-center flex-wrap gap-2 max-[1100px]:justify-start">
            <div class="absolute left-2 top-1/2 -translate-y-1/2 min-w-0 max-w-[44%] max-[1100px]:static max-[1100px]:translate-y-0 max-[1100px]:max-w-full max-[1100px]:order-1">
                <div class="inline-flex flex-wrap items-center gap-1.5 max-w-full px-2 py-1 rounded border border-[#1e293b] bg-[#0f172a]/80">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#8b949e" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <line x1="6" y1="3" x2="6" y2="15"></line>
                        <circle cx="18" cy="6" r="3"></circle>
                        <circle cx="6" cy="18" r="3"></circle>
                        <path d="M18 9a9 9 0 0 1-9 9"></path>
                    </svg>
                    <span class="text-[10px] uppercase tracking-wider text-[#8b949e] shrink-0">Current</span>
                    <div class="relative min-w-0 sm:min-w-[126px] max-w-[240px] flex-1">
                        <select
                            class="branch-switch-select w-full rounded bg-[#0d1526] border border-[#1e293b] text-xs font-mono text-[#c9d1d9] pl-2 pr-6 py-0.5 focus:outline-none focus:ring-1 focus:ring-[#4a90d9] disabled:opacity-60"
                            value={currentBranchName}
                            onchange={handleToolbarBranchChange}
                            disabled={!repoPath || isToolbarBranchesLoading || isToolbarBranchSwitching || (toolbarLocalBranches.length === 0 && !currentBranchName)}
                            title={currentBranchLabel}
                        >
                            {#if !currentBranchName}
                                <option value="" disabled>{currentBranchLabel}</option>
                            {/if}
                            {#if toolbarLocalBranches.length === 0}
                                <option value="" disabled>{isToolbarBranchesLoading ? "Loading branches..." : "No branches"}</option>
                            {:else}
                                {#each toolbarLocalBranches as branch (branch)}
                                    <option value={branch}>{branch}</option>
                                {/each}
                            {/if}
                        </select>
                        <svg class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[#8b949e]" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                            <path d="m6 9 6 6 6-6"></path>
                        </svg>
                    </div>
                </div>
            </div>
            <!-- Actions -->
            <div class="flex flex-wrap items-center gap-1 max-[1100px]:order-2">
                <button 
                    class="text-xs text-[#8b949e] hover:text-white px-2 py-1 rounded hover:bg-[#1e293b] flex items-center gap-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
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
                    class="text-xs text-[#8b949e] hover:text-white px-2 py-1 rounded hover:bg-[#1e293b] flex items-center gap-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
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
                    class="text-xs text-[#8b949e] hover:text-white px-2 py-1 rounded hover:bg-[#1e293b] flex items-center gap-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
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

            {#if leftPanelMode === 'graph'}
                <div class="absolute right-2 top-1/2 -translate-y-1/2 z-50 max-[1100px]:static max-[1100px]:translate-y-0 max-[1100px]:order-3">
                    <button  
                        onclick={() => showMenu = !showMenu}
                        class="text-xs text-[#8b949e] hover:text-[#c9d1d9] px-2 py-1 rounded hover:bg-[#1e293b] flex items-center gap-1 transition-colors"
                    >
                        <span>Columns</span>
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
                    </button>

                    {#if showMenu}
                        <div class="absolute top-8 right-0 bg-[#111827] border border-[#1e293b] rounded-md shadow-xl z-[70] p-2 w-40 animate-in fade-in zoom-in-95 duration-100">
                            {#each columns as col (col.id)}
                                <label class="flex items-center gap-2 p-1.5 hover:bg-[#1e293b] rounded cursor-pointer text-xs text-[#c9d1d9]">
                                    <input type="checkbox" bind:checked={col.visible} class="rounded border-[#1e293b] bg-[#0f172a] text-[#238636] focus:ring-0">
                                    {col.label}
                                </label>
                            {/each}
                        </div>
                    {/if}
                </div>
            {/if}

            {#if leftPanelMode === 'graph' && showMenu}
                <!-- Backdrop to close -->
                <div class="fixed inset-0 z-40" onclick={() => showMenu = false} role="none"></div>
            {/if}
        </div>

        <!-- Header Row -->
        <div 
            class="border-b border-[#1e293b] bg-[#111827] select-none text-xs font-semibold text-[#8b949e] uppercase tracking-wider relative shrink-0"
            style="display: grid; grid-template-columns: {gridTemplate}; min-width: 100%;"
        >
            {#each visibleColumns as col (col.id)}
            <div 
                class="relative flex items-center px-4 h-8 group border-r border-[#1e293b]/30"
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

        <div class="flex-1 overflow-auto custom-scrollbar relative" bind:this={graphViewportEl} onscroll={handleGraphViewportScroll}>
            <div class="relative min-w-full" style="height: {totalRowCount * ROW_HEIGHT + PADDING_TOP}px;">
                <!-- Background stripe rows -->
                <div class="absolute top-0 left-0 w-full pt-[8px] z-[1] pointer-events-none">
                    {#each rowIndexes as rowIndex (rowIndex)}
                        <div
                            class="border-b border-[#1e293b]/15 {rowIndex % 2 === 0 ? 'bg-[#0f172a]' : 'bg-[#111b2f]/55'}"
                            style="height: {ROW_HEIGHT}px;"
                        ></div>
                    {/each}
                </div>
                
                <!-- Graph SVG Layer -->
                <!-- Locked to the width of the 'graph' column if visible -->
                {#if graphColumn?.visible}
                    <div class="absolute top-0 h-full pointer-events-none z-[5] overflow-hidden" style="left: {graphColumnOffset}px; width: {graphColumn?.width}px">
                        <svg class="w-full h-full">
                        <defs>
                            <clipPath id={AVATAR_CLIP_ID} clipPathUnits="userSpaceOnUse">
                                <circle cx="0" cy="0" r={AVATAR_RADIUS} />
                            </clipPath>
                            <clipPath id={AVATAR_STASH_CLIP_ID} clipPathUnits="userSpaceOnUse">
                                <rect
                                    x={-AVATAR_RADIUS}
                                    y={-AVATAR_RADIUS}
                                    width={AVATAR_SIZE}
                                    height={AVATAR_SIZE}
                                    rx={STASH_AVATAR_CORNER_RADIUS}
                                />
                            </clipPath>
                            <filter id={AVATAR_SHADOW_ID} x="-20%" y="-20%" width="140%" height="140%">
                                <feDropShadow dx="0" dy="0" stdDeviation="1.5" flood-color="#000" flood-opacity="0.45" />
                            </filter>
                        </defs>
                        <g transform="translate({PADDING_LEFT}, {PADDING_TOP})">
                            <!-- Continuous vertical lane lines -->
                            <g class="lanes">
                                {#each laneGeometry as lane (lane.key)}
                                    <!-- Glow layer for hovered branch -->
                                    {#if hoveredBranchColor && lane.color === hoveredBranchColor}
                                        <path
                                            d={lane.path}
                                            fill="none"
                                            stroke={lane.color}
                                            stroke-width={STROKE_WIDTH + 4}
                                            stroke-linecap="round"
                                            opacity="0.15"
                                        />
                                    {/if}
                                    <path
                                        class="graph-edge"
                                        d={lane.path}
                                        fill="none"
                                        stroke={lane.color}
                                        stroke-width={STROKE_WIDTH}
                                        stroke-linecap="round"
                                        opacity={hoveredBranchColor && lane.color !== hoveredBranchColor ? 0.26 : 0.92}
                                    />
                                {/each}
                            </g>
                            <!-- L-shaped merge/fork connection lines -->
                            <g class="connections">
                                {#each connectionGeometry as conn (conn.key)}
                                    <!-- Glow layer for hovered branch -->
                                    {#if hoveredBranchColor && conn.color === hoveredBranchColor}
                                        <path
                                            d={conn.path}
                                            fill="none"
                                            stroke={conn.color}
                                            stroke-width={STROKE_WIDTH + 4}
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            stroke-dasharray={conn.lineStyle === "dashed" ? "6 4" : undefined}
                                            opacity="0.15"
                                        />
                                    {/if}
                                    <path
                                        class="graph-edge"
                                        d={conn.path}
                                        fill="none"
                                        stroke={conn.color}
                                        stroke-width={STROKE_WIDTH}
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-dasharray={conn.lineStyle === "dashed" ? "6 4" : undefined}
                                        opacity={hoveredBranchColor && conn.color !== hoveredBranchColor ? 0.26 : 0.92}
                                    />
                                {/each}
                            </g>
                            <!-- WIP connection line from HEAD commit to WIP row -->
                            {#if hasWipRow && currentHeadNode}
                                {@const headX = nodeRenderX(currentHeadNode)}
                                {@const headY = nodeRenderY(currentHeadNode)}
                                {@const wipY = ROW_HEIGHT / 2}
                                <path
                                    class="graph-edge"
                                    d={`M ${wipRowGraphX} ${wipY} V ${headY}`}
                                    fill="none"
                                    stroke={currentHeadNode.color}
                                    stroke-width={STROKE_WIDTH}
                                    stroke-linecap="round"
                                    stroke-dasharray="4 3"
                                    opacity={hoveredBranchColor && currentHeadNode.color !== hoveredBranchColor ? 0.26 : 0.65}
                                />
                            {/if}
                            <g class="nodes">
                                {#if hasWipRow}
                                    <g class="graph-node" transform={`translate(${wipRowGraphX}, ${ROW_HEIGHT / 2})`}>
                                        <g filter={`url(#${AVATAR_SHADOW_ID})`}>
                                            <circle
                                                cx="0"
                                                cy="0"
                                                r={AVATAR_RADIUS + 1}
                                                fill="#0f172a"
                                                stroke="#58a6ff"
                                                stroke-width="2"
                                                stroke-dasharray="2 2"
                                            />
                                            <circle
                                                cx="0"
                                                cy="0"
                                                r={2}
                                                fill="#58a6ff"
                                            />
                                        </g>
                                    </g>
                                {/if}
                                {#each nodes as node (node.hash)}
                                    {@const cx = nodeRenderX(node)}
                                    {@const cy = nodeRenderY(node)}
                                    {@const isSelected = selectedCommit?.hash === node.hash}
                                    {@const isHovered = hoveredCommitHash === node.hash}
                                    {@const isStashNode = node.isStash}
                                    {@const avatarUrl = avatarCache.get(node.author) ?? getAvatarUrl(node.author)}
                                    <g
                                        class="graph-node"
                                        transform={`translate(${cx}, ${cy})`}
                                        aria-label={`Commit ${node.hash} by ${node.author}`}
                                    >
                                        <g transform={`scale(${isHovered ? 1.1 : 1})`} filter={`url(#${AVATAR_SHADOW_ID})`}>
                                            <title>{`${node.hash} - ${node.subject}`}</title>
                                            {#if isStashNode}
                                                <!-- Color fallback square (shows if avatar fails to load) -->
                                                <rect
                                                    x={-AVATAR_RADIUS}
                                                    y={-AVATAR_RADIUS}
                                                    width={AVATAR_SIZE}
                                                    height={AVATAR_SIZE}
                                                    rx={STASH_AVATAR_CORNER_RADIUS}
                                                    fill={node.color}
                                                    opacity={STASH_AVATAR_BASE_OPACITY}
                                                />
                                                <rect
                                                    x={-AVATAR_RADIUS}
                                                    y={-AVATAR_RADIUS}
                                                    width={AVATAR_SIZE}
                                                    height={AVATAR_SIZE}
                                                    rx={STASH_AVATAR_CORNER_RADIUS}
                                                    fill="rgba(255,255,255,0.22)"
                                                />
                                                <rect
                                                    x={-AVATAR_RADIUS - 2}
                                                    y={-AVATAR_RADIUS - 2}
                                                    width={AVATAR_SIZE + 4}
                                                    height={AVATAR_SIZE + 4}
                                                    rx={STASH_AVATAR_CORNER_RADIUS + 2}
                                                    fill="none"
                                                    stroke={isSelected ? "#f0f6fc" : "rgba(255,255,255,0.9)"}
                                                    stroke-width="2"
                                                    stroke-dasharray={STASH_AVATAR_DASH}
                                                    opacity="0.85"
                                                />
                                            {:else}
                                                <!-- Color fallback circle (shows if avatar fails to load) -->
                                                <circle
                                                    cx="0"
                                                    cy="0"
                                                    r={AVATAR_RADIUS}
                                                    fill={node.color}
                                                />
                                                <!-- White border circle -->
                                                <circle
                                                    cx="0"
                                                    cy="0"
                                                    r={AVATAR_RADIUS + 2}
                                                    fill="none"
                                                    stroke={isSelected ? "#f0f6fc" : "rgba(255,255,255,0.9)"}
                                                    stroke-width="2"
                                                />
                                            {/if}
                                            <!-- Avatar image -->
                                            <image
                                                href={avatarUrl}
                                                x={-AVATAR_RADIUS}
                                                y={-AVATAR_RADIUS}
                                                width={AVATAR_SIZE}
                                                height={AVATAR_SIZE}
                                                clip-path={`url(#${isStashNode ? AVATAR_STASH_CLIP_ID : AVATAR_CLIP_ID})`}
                                                preserveAspectRatio="xMidYMid slice"
                                                opacity={isStashNode ? STASH_AVATAR_IMAGE_OPACITY : 1}
                                            />
                                            <!-- Selection ring + glow -->
                                            {#if isSelected}
                                                {#if isStashNode}
                                                    <rect
                                                        x={-AVATAR_RADIUS - 4}
                                                        y={-AVATAR_RADIUS - 4}
                                                        width={AVATAR_SIZE + 8}
                                                        height={AVATAR_SIZE + 8}
                                                        rx={STASH_AVATAR_CORNER_RADIUS + 4}
                                                        fill="none"
                                                        stroke={node.color}
                                                        stroke-width="1"
                                                        stroke-dasharray={STASH_AVATAR_DASH}
                                                        opacity="0.3"
                                                    />
                                                    <rect
                                                        x={-AVATAR_RADIUS - 3}
                                                        y={-AVATAR_RADIUS - 3}
                                                        width={AVATAR_SIZE + 6}
                                                        height={AVATAR_SIZE + 6}
                                                        rx={STASH_AVATAR_CORNER_RADIUS + 3}
                                                        fill="none"
                                                        stroke={node.color}
                                                        stroke-width="2"
                                                        stroke-dasharray={STASH_AVATAR_DASH}
                                                        opacity="0.8"
                                                    />
                                                {:else}
                                                    <circle
                                                        cx="0"
                                                        cy="0"
                                                        r={AVATAR_RADIUS + 4}
                                                        fill="none"
                                                        stroke={node.color}
                                                        stroke-width="1"
                                                        opacity="0.3"
                                                    />
                                                    <circle
                                                        cx="0"
                                                        cy="0"
                                                        r={AVATAR_RADIUS + 2.5}
                                                        fill="none"
                                                        stroke={node.color}
                                                        stroke-width="2"
                                                        opacity="0.8"
                                                    />
                                                {/if}
                                            {/if}
                                        </g>
                                    </g>
                                {/each}
                            </g>
                        </g>
                        </svg>
                    </div>
                {/if}

                <!-- Rows Container -->
                <div class="absolute top-0 left-0 w-full pt-[8px] z-10">
                {#if hasWipRow}
                    {@const isWipHoveredRow = hoveredCommitHash === "__wip__"}
                    <div
                        class="border-b border-[#1e293b]/20 transition-colors text-xs items-center group cursor-pointer
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#4a90d9]/70 focus-visible:ring-inset
                               {isWipHoveredRow ? 'row-hover' : ''} {isWipRowSelected ? 'row-selected' : ''}"
                        style="display: grid; grid-template-columns: {gridTemplate}; height: {ROW_HEIGHT}px;"
                        onclick={selectWipRow}
                        onmouseenter={() => {
                            hoveredCommitHash = "__wip__";
                            hoveredBranchColor = currentHeadNode?.color ?? null;
                            hideCommitTooltip();
                        }}
                        onmouseleave={handleRowMouseLeave}
                        onfocus={() => {
                            hoveredCommitHash = "__wip__";
                            hoveredBranchColor = currentHeadNode?.color ?? null;
                            hideCommitTooltip();
                        }}
                        onblur={handleRowBlur}
                        role="button"
                        tabindex="0"
                        aria-label="Select working tree changes"
                        onkeydown={handleWipRowKeydown}
                    >
                        {#each visibleColumns as col (col.id)}
                        {#if col.id === "graph"}
                            <div class="pointer-events-none"></div>
                        {:else if col.id === "branch"}
                            <div class="pl-3 pr-2 flex items-center gap-1.5 graph-row-info-cell">
                                <span class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0 bg-cyan-900/40 text-cyan-300 border-cyan-700/50">
                                    WIP
                                </span>
                            </div>
                        {:else if col.id === "message"}
                            <div class="pl-4 pr-4 flex items-center min-w-0 graph-row-info-cell">
                                <div class="w-full h-6 rounded border border-[#245d84]/60 bg-[#0b2942]/65 flex items-center gap-3 px-2 overflow-hidden">
                                    <span class="text-[11px] font-mono text-[#79c0ff] shrink-0">// WIP</span>
                                    <span class="text-[10px] text-[#3fb950] shrink-0">+{wipSummary.stagedCount} staged</span>
                                    <span class="text-[10px] text-[#f2cc60] shrink-0">+{wipSummary.unstagedCount} unstaged</span>
                                    <span class="text-[10px] text-[#8b949e] truncate">{wipSummary.totalCount} changed file(s)</span>
                                </div>
                            </div>
                        {:else if col.id === "hash"}
                            <div class="pl-4 font-mono text-[#8b949e] opacity-70 truncate graph-row-info-cell">WIP</div>
                        {:else if col.id === "author"}
                            <div class="pl-4 truncate text-[#8b949e] opacity-70 graph-row-info-cell">Working Tree</div>
                        {:else if col.id === "date"}
                            <div class="pl-4 text-[#8b949e] opacity-70 font-mono truncate graph-row-info-cell">Now</div>
                        {/if}
                        {/each}
                    </div>
                {/if}
                {#each nodes as node (node.hash)}
                    {@const isCurrentHead = currentHeadNode?.hash === node.hash}
                    {@const isHoveredRow = hoveredCommitHash === node.hash}
                    {@const isSelectedRow = selectedCommit?.hash === node.hash}
                    <div 
                        class="border-b border-[#1e293b]/20 transition-colors text-xs items-center group cursor-pointer
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#4a90d9]/70 focus-visible:ring-inset
                               {isHoveredRow ? 'row-hover' : ''} {isCurrentHead ? 'row-head' : ''} {isSelectedRow ? 'row-selected' : ''}"
                        style="display: grid; grid-template-columns: {gridTemplate}; height: {ROW_HEIGHT}px;"
                        onclick={() => handleCommitRowClick(node)}
                        oncontextmenu={(event) => handleCommitRowContextMenu(event, node)}
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
                        {#each visibleColumns as col (col.id)}
                        {#if col.id === 'graph'}
                            <div class="pointer-events-none"><!-- Placeholder for SVG overlay --></div>
                        {:else if col.id === 'branch'}
                            {@const branchBadges = getRankedRefBadges(node.refs)}
                            {@const primaryBadge = branchBadges[0]}
                            {@const secondaryBadges = branchBadges.slice(1)}
                            <div class="pl-3 pr-2 flex items-center gap-1.5 relative branch-cell graph-row-info-cell {getRowCellHighlightClass(node.hash, col.id)}">
                                {#if primaryBadge}
                                    {#if canCheckoutFromBadge(primaryBadge)}
                                        <button
                                            type="button"
                                            class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0 truncate max-w-[118px] bg-transparent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-sky-500/80 cursor-pointer hover:brightness-110 {getRefBadgeClass(primaryBadge)}"
                                            title={`${primaryBadge.text} (click to checkout)`}
                                            onclick={(e) => handleBranchBadgeClick(e, primaryBadge)}
                                            oncontextmenu={(e) => handleBranchBadgeContextMenu(e, primaryBadge, node)}
                                        >
                                            {primaryBadge.text}
                                        </button>
                                    {:else}
                                        <span
                                            class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0 truncate max-w-[118px] {getRefBadgeClass(primaryBadge)}"
                                            title={primaryBadge.text}
                                        >
                                            {primaryBadge.text}
                                        </span>
                                    {/if}
                                {/if}
                                {#if secondaryBadges.length > 0}
                                    <span class="px-1 py-0.5 rounded text-[10px] font-medium border shrink-0 bg-slate-800/70 text-slate-300 border-slate-600/50" title={`${secondaryBadges.length} refs hidden`}>
                                        +{secondaryBadges.length}
                                    </span>
                                    <div class="branch-dropdown">
                                        {#each secondaryBadges as badge (`${badge.type}:${badge.text}:${badge.originalIndex}`)}
                                            <div class="branch-dropdown-item">
                                                {#if canCheckoutFromBadge(badge)}
                                                    <button
                                                        type="button"
                                                        class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0 bg-transparent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-sky-500/80 cursor-pointer hover:brightness-110 {getRefBadgeClass(badge)}"
                                                        title={`${badge.text} (double-click to checkout)`}
                                                        onclick={(e) => e.stopPropagation()}
                                                        ondblclick={(e) => handleBranchBadgeClick(e, badge)}
                                                        oncontextmenu={(e) => handleBranchBadgeContextMenu(e, badge, node)}
                                                    >
                                                        {badge.text}
                                                    </button>
                                                {:else}
                                                    <span
                                                        class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0 {getRefBadgeClass(badge)}"
                                                        title={badge.text}
                                                    >
                                                        {badge.text}
                                                    </span>
                                                {/if}
                                            </div>
                                        {/each}
                                    </div>
                                {/if}
                            </div>
                        {:else if col.id === 'hash'}
                            <div class="pl-4 font-mono text-[#8b949e] opacity-70 truncate graph-row-info-cell {getRowCellHighlightClass(node.hash, col.id)}">{node.hash}</div>
                        {:else if col.id === 'message'}
                            <div class="pl-4 flex items-center min-w-0 pr-4 overflow-hidden relative graph-row-info-cell {getRowCellHighlightClass(node.hash, col.id)}">
                                <span class="truncate text-[#c9d1d9] group-hover:text-white font-medium">{node.subject}</span>
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
            {#if isLoadingMoreCommits}
                <div class="pointer-events-none absolute bottom-3 left-1/2 -translate-x-1/2 rounded border border-[#30363d] bg-[#0f172a] px-3 py-1 text-[11px] text-[#8b949e]">
                    Loading older commits...
                </div>
            {/if}
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
  {#if selectedCommit || isWipRowSelected}
      <ResizablePanel side="left" initialSize={isWipRowSelected ? 380 : 320} minSize={240} maxSize={600}>
          {#if isWipRowSelected}
              <div class="h-full border-l border-[#1e293b]">
                  <GraphWipPanel
                      bind:this={wipPanelRef}
                      repoPath={repoPath ?? ""}
                      onFileSelect={openWipDiff}
                      onClose={closeDetails}
                      onCommitSuccess={handleWipCommitSuccess}
                      {onShowHistory}
                      {onShowBlame}
                  />
              </div>
          {:else}
          <div class="h-full flex flex-col bg-[#0f172a] border-l border-[#1e293b]">
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
                       {#if selectedCommit}
                           <div class="select-text font-mono text-[10px] text-[#8b949e] bg-[#111827] p-1.5 rounded border border-[#1e293b]">
                               {selectedCommit.hash}
                           </div>
                           
                           <div class="text-sm font-medium text-[#c9d1d9] leading-tight select-text py-1">
                               {selectedCommit.subject}
                           </div>
                           
                           <div class="flex items-center gap-2 text-xs text-[#8b949e]">
                               <div class="flex items-center gap-1 overflow-hidden" title={selectedCommit.author}>
                                   <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="4"/><path d="M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-3.92 7.94"/></svg>
                                   <span class="truncate">{selectedCommit.author}</span>
                               </div>
                               <span>|</span>
                               <span title={selectedCommit.date}>
                                   {new Date(selectedCommit.date).toLocaleString()}
                               </span>
                           </div>
                       {/if}
                  </div>

                  <!-- Changes -->
                  <div class="mt-4">
                      <div class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider mb-2 flex justify-between items-center">
                          <div class="flex items-center gap-2">
                              <span>Changed Files</span>
                              {#if changedFiles.length > 0}
                                  <span class="text-[10px] font-normal bg-[#1e293b] text-[#c9d1d9] px-1.5 rounded-full">{changedFiles.length}</span>
                              {/if}
                          </div>
                          <div class="inline-flex rounded border border-[#1e293b] overflow-hidden normal-case tracking-normal">
                              <button
                                  type="button"
                                  class="px-2 py-0.5 text-[10px] font-medium transition-colors {changedFilesViewMode === 'tree' ? 'bg-[#1e293b] text-white' : 'bg-[#0f172a] text-[#8b949e] hover:text-[#c9d1d9]'}"
                                  onclick={() => changedFilesViewMode = "tree"}
                                  title="View changed files as directory tree"
                              >
                                  Tree
                              </button>
                              <button
                                  type="button"
                                  class="px-2 py-0.5 text-[10px] font-medium border-l border-[#1e293b] transition-colors {changedFilesViewMode === 'path' ? 'bg-[#1e293b] text-white' : 'bg-[#0f172a] text-[#8b949e] hover:text-[#c9d1d9]'}"
                                  onclick={() => changedFilesViewMode = "path"}
                                  title="View changed files by path"
                              >
                                  Path
                              </button>
                          </div>
                      </div>
                      
                      {#if isLoadingFiles}
                          <div class="flex items-center justify-center py-8 text-[#8b949e] gap-2">
                              <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
                              <span class="text-xs">Loading changes...</span>
                          </div>
                      {:else if changedFiles.length > 0}
                          <div class="space-y-0.5">
                              {#each changedFileRows as row (row.key)}
                                  {#if row.kind === "directory"}
                                      <button
                                          type="button"
                                          class="w-full flex items-center gap-1.5 px-1 py-1 text-xs rounded text-[#8b949e] hover:bg-[#111827] transition-colors"
                                          style={`padding-left: ${4 + row.depth * 12}px;`}
                                          onclick={() => toggleChangedFilesDirectory(row.path)}
                                          title={row.path}
                                      >
                                          <svg class={`w-3 h-3 shrink-0 transition-transform ${row.collapsed ? '' : 'rotate-90'}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                              <polyline points="9 6 15 12 9 18"></polyline>
                                          </svg>
                                          <svg class="w-3.5 h-3.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                              <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z"></path>
                                          </svg>
                                          <span class="truncate text-left">{row.name}</span>
                                          <span class="ml-auto text-[10px] text-[#6e7681]">{row.fileCount}</span>
                                      </button>
                                  {:else}
                                      <div 
                                          class="flex items-start gap-2 py-1 px-1 hover:bg-[#111827] rounded text-xs group cursor-pointer {selectedDiffFile === row.file.path ? 'bg-[#1e293b] text-white' : ''}" 
                                          style={`padding-left: ${4 + row.depth * 12}px;`}
                                          title={row.title}
                                          onclick={() => {
                                              closeChangedFileContextMenu();
                                              if (isWipRowSelected) return;
                                              void openDiff(row.file.path);
                                          }}
                                          oncontextmenu={(e) => handleChangedFileContextMenu(e, row.file)}
                                          role="button"
                                          tabindex="0"
                                          onkeydown={(e) => handleChangedFileKeydown(e, row.file.path)}
                                      >
                                          <FileChangeStatusBadge status={row.file.status} compact={true} showCode={true} className="shrink-0 mt-[1px]" />
                                          <span class="truncate text-[#c9d1d9] leading-tight">
                                              {row.label}
                                          </span>
                                      </div>
                                  {/if}
                              {/each}
                          </div>
                      {:else}
                          <div class="text-xs text-[#8b949e] italic text-center py-4">
                              No changes found
                          </div>
                      {/if}
                  </div>
              </div>
          </div>
          {/if}
      </ResizablePanel>
  {/if}
</div>

{#if changedFileContextMenu.visible}
  <div
    class="fixed z-[70] rounded-md border border-[#30363d] bg-[#161b22] shadow-2xl py-1 changed-file-context-menu"
    style="top: {changedFileContextMenu.y}px; left: {changedFileContextMenu.x}px; width: {CHANGED_FILE_CONTEXT_MENU_WIDTH}px;"
    role="menu"
    tabindex="-1"
    oncontextmenu={(e) => {
      e.preventDefault();
      e.stopPropagation();
    }}
  >
    {#if onShowHistory}
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={handleChangedFileShowHistory}
        role="menuitem"
      >
        File History
      </button>
    {/if}
    {#if onShowBlame}
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={handleChangedFileShowBlame}
        role="menuitem"
      >
        File Blame
      </button>
    {/if}

    {#if onShowHistory || onShowBlame}
      <div class="border-t border-[#30363d] my-1"></div>
    {/if}

    {#if repoPath}
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={() => void handleOpenChangedFileInDiffTool()}
        role="menuitem"
      >
        Open in external diff tool
      </button>
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={() => void handleOpenChangedFileInEditor()}
        role="menuitem"
      >
        Open in external editor
      </button>
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={() => void handleOpenChangedFileInDefaultProgram()}
        role="menuitem"
      >
        Open file in default program
      </button>
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={() => void handleShowChangedFileInFolder()}
        role="menuitem"
      >
        Show in folder
      </button>

      <div class="border-t border-[#30363d] my-1"></div>
    {/if}

    <button
      type="button"
      class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
      onclick={() => void handleCopyChangedFilePath()}
      role="menuitem"
    >
      Copy file path
    </button>

    {#if repoPath}
      <div class="border-t border-[#30363d] my-1"></div>
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={() => void handleEditChangedFile()}
        role="menuitem"
      >
        Edit file
      </button>
      <button
        type="button"
        class={CHANGED_FILE_CONTEXT_MENU_ITEM_CLASS}
        onclick={() => void handleDeleteChangedFile()}
        role="menuitem"
      >
        Delete file
      </button>
    {/if}
  </div>
{/if}

<BranchContextMenu
  menu={branchContextMenu}
  onClose={closeBranchContextMenu}
  onAction={handleBranchContextAction}
/>

<RebaseEditor />
<RebaseProgress />

<CommitContextMenu
  menu={commitContextMenu}
  onClose={closeCommitContextMenu}
  onAction={handleCommitContextAction}
/>

<StashCommitContextMenu
  menu={stashCommitContextMenu}
  onClose={closeStashCommitContextMenu}
  onAction={handleStashCommitContextAction}
/>

<style>
  .graph-edge {
    transition: opacity 120ms ease;
    animation: graph-edge-fade-in 180ms ease-out both;
  }

  .graph-node {
    animation: node-enter 250ms ease-out both;
  }

  .graph-row-info-cell {
    transition: background-color 120ms ease;
  }

  .row-hover {
    background: rgba(56, 139, 253, 0.1);
  }

  .row-head {
    background: rgba(255, 143, 74, 0.2);
  }

  .row-head.row-hover {
    background: rgba(255, 143, 74, 0.36);
  }

  .row-selected {
    background: rgba(31, 111, 235, 0.2);
  }

  .row-selected.row-hover {
    background: rgba(31, 111, 235, 0.26);
  }

  .row-head.row-selected {
    background: rgba(31, 111, 235, 0.22);
  }

  .row-head.row-selected.row-hover {
    background: rgba(31, 111, 235, 0.28);
  }

  .branch-switch-select {
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
  }

  .branch-cell {
    overflow: visible;
  }

  .branch-dropdown {
    position: absolute;
    left: 0;
    top: 100%;
    z-index: 40;
    min-width: 160px;
    max-width: 260px;
    padding: 6px;
    border-radius: 8px;
    border: 1px solid rgba(30, 41, 59, 0.95);
    background: rgba(15, 23, 42, 0.96);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.45);
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .branch-cell:hover .branch-dropdown {
    opacity: 1;
    pointer-events: auto;
  }

  .branch-dropdown-item + .branch-dropdown-item {
    margin-top: 4px;
  }

  .graph-tooltip {
    max-width: 340px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid rgba(74, 144, 217, 0.25);
    background: rgba(15, 23, 42, 0.95);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
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

  @keyframes node-enter {
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
    background: #0f172a;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #1e293b;
    border: 2px solid #0f172a;
    border-radius: 99px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #334155;
  }
</style>

