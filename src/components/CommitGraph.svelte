<script lang="ts">
  import { getAvatarUrl, type GraphNode, type LanePath, type ConnectionPath } from "../lib/graph-layout";
  import { onMount } from "svelte";
  import { GitService, type CommitChangedFile, type FileStatus } from "../lib/GitService";
  import { confirm } from "../lib/confirmation.svelte";
  import { prompt } from "../lib/prompt.svelte";
  import { toast } from "../lib/toast.svelte";
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
  import type { BranchContextMenuState } from "./common/branch-context-menu-types";
  import type {
      CommitContextMenuAction,
      CommitContextMenuState
  } from "./common/commit-context-menu-types";
  import type {
      StashCommitContextMenuAction,
      StashCommitContextMenuState
  } from "./common/stash-commit-context-menu-types";
  import FileChangeStatusBadge from "./common/FileChangeStatusBadge.svelte";

  interface Props {
    nodes?: GraphNode[];
    lanes?: LanePath[];
    connections?: ConnectionPath[];
    repoPath?: string;
    pendingPushCount?: number;
    onGraphReload?: () => Promise<void>;
    onNavigateToCommitPanel?: () => void;
  }

  let { nodes = [], lanes = [], connections = [], repoPath, pendingPushCount = 0, onGraphReload, onNavigateToCommitPanel }: Props = $props();

  const ROW_HEIGHT = GRAPH_CONFIG.ROW_HEIGHT;
  const COL_WIDTH = GRAPH_CONFIG.COLUMN_WIDTH;
  const STROKE_WIDTH = GRAPH_CONFIG.STROKE_WIDTH;
  const PADDING_TOP = GRAPH_CONFIG.PADDING_TOP;
  const PADDING_LEFT = GRAPH_CONFIG.PADDING_LEFT;
  const TOOLTIP_OFFSET_X = GRAPH_CONFIG.TOOLTIP_OFFSET_X;
  const TOOLTIP_OFFSET_Y = GRAPH_CONFIG.TOOLTIP_OFFSET_Y;
  const TOOLTIP_MAX_WIDTH = GRAPH_CONFIG.TOOLTIP_MAX_WIDTH;
  const TOOLTIP_MAX_HEIGHT = GRAPH_CONFIG.TOOLTIP_MAX_HEIGHT;
  const AVATAR_SIZE = GRAPH_CONFIG.AVATAR_SIZE;
  const AVATAR_RADIUS = AVATAR_SIZE / 2;
  const STASH_AVATAR_CORNER_RADIUS = GRAPH_CONFIG.STASH_AVATAR_CORNER_RADIUS;
  const STASH_AVATAR_IMAGE_OPACITY = GRAPH_CONFIG.STASH_AVATAR_IMAGE_OPACITY;
  const STASH_AVATAR_BASE_OPACITY = GRAPH_CONFIG.STASH_AVATAR_BASE_OPACITY;
  const STASH_AVATAR_DASH = GRAPH_CONFIG.STASH_AVATAR_DASH;
  const SVG_INSTANCE_ID = `graph-${Math.random().toString(36).slice(2, 9)}`;
  const AVATAR_CLIP_ID = `${SVG_INSTANCE_ID}-avatar-clip`;
  const AVATAR_STASH_CLIP_ID = `${SVG_INSTANCE_ID}-avatar-stash-clip`;
  const AVATAR_SHADOW_ID = `${SVG_INSTANCE_ID}-avatar-shadow`;
  const CHANGED_FILES_VIEW_MODE_KEY = "commit_graph_changed_files_view_mode";
  const PATH_LABEL_MAX_LENGTH = GRAPH_CONFIG.PATH_LABEL_MAX_LENGTH;
  const PATH_COLLAPSE_TOKEN = GRAPH_CONFIG.PATH_COLLAPSE_TOKEN;
  const CHANGED_FILE_CONTEXT_MENU_WIDTH = GRAPH_CONFIG.CHANGED_FILE_CONTEXT_MENU_WIDTH;
  const CHANGED_FILE_CONTEXT_MENU_ITEM_HEIGHT = GRAPH_CONFIG.CHANGED_FILE_CONTEXT_MENU_ITEM_HEIGHT;
  const CHANGED_FILE_CONTEXT_MENU_PADDING_Y = GRAPH_CONFIG.CHANGED_FILE_CONTEXT_MENU_PADDING_Y;

  const HEADER_BASE = "h-8 flex items-center bg-[#111827] border-b border-[#1e293b] shrink-0";
  
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

  async function selectCommit(node: GraphNode) {
      if (selectedCommit?.hash === node.hash && !isWipRowSelected) return;
      
      selectedCommit = node;
      isWipRowSelected = false;
      changedFiles = [];
      changedFilesCollapsedDirs = new Set();
      closeChangedFileContextMenu();
      closeBranchContextMenu();
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

  function selectWipRow() {
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
      onNavigateToCommitPanel?.();
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

  function buildChangedFilesTree(items: CommitChangedFile[]): ChangedFilesTreeDirectory {
      const root: ChangedFilesTreeDirectory = {
          name: "",
          path: "",
          children: new Map(),
          files: []
      };

      for (const file of items) {
          const treePath = getTreePath(file.path);
          const parts = treePath.split("/").filter(Boolean);

          if (parts.length <= 1) {
              root.files.push(file);
              continue;
          }

          parts.pop();
          let current = root;
          for (const part of parts) {
              let child = current.children.get(part);
              if (!child) {
                  const childPath = current.path ? `${current.path}/${part}` : part;
                  child = {
                      name: part,
                      path: childPath,
                      children: new Map(),
                      files: []
                  };
                  current.children.set(part, child);
              }
              current = child;
          }
          current.files.push(file);
      }

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

  function getChangedFileContextMenuHeight(): number {
      let actionCount = 1; // Copy file path is always available.
      if (repoPath) actionCount += 1;
      return actionCount * CHANGED_FILE_CONTEXT_MENU_ITEM_HEIGHT + CHANGED_FILE_CONTEXT_MENU_PADDING_Y * 2;
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

  async function handleOpenChangedFile(): Promise<void> {
      if (!repoPath || !changedFileContextMenu.file) return;
      const targetPath = changedFileContextMenu.file.path;
      closeChangedFileContextMenu();
      await GitService.openRepoFile(targetPath, repoPath);
  }

  async function handleCopyChangedFilePath(): Promise<void> {
      if (!changedFileContextMenu.file) return;
      const targetPath = changedFileContextMenu.file.path;
      closeChangedFileContextMenu();

      try {
          await navigator.clipboard.writeText(targetPath);
          toast.success(`Copied path: ${targetPath}`);
      } catch (e) {
          console.error("Copy file path failed", e);
          toast.error("Copy file path failed");
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

  // Parse refs and rank: current branch first, then local branches, remotes, tags.
  function getRankedRefBadges(refs: string[]): RefBadge[] {
      const parsed: RefBadge[] = [];
      const seen = new Set<string>();

      function pushBadge(text: string, type: RefBadgeType, isCurrent: boolean, index: number) {
          const normalized = text.trim();
          if (!normalized) return;
          const dedupeKey = `${type}:${normalized.toLowerCase()}`;
          if (seen.has(dedupeKey)) return;
          seen.add(dedupeKey);
          parsed.push({ text: normalized, type, isCurrent, originalIndex: index });
      }

      refs.forEach((rawRef, index) => {
          const ref = rawRef.trim();
          if (!ref) return;
          if (isStashRefBadge(ref)) return;

          if (ref.includes("HEAD ->")) {
              const currentBranch = ref.split("HEAD ->")[1]?.trim() ?? "";
              pushBadge(currentBranch, "branch", true, index);
              return;
          }

          if (ref.startsWith("tag:")) {
              pushBadge(ref.replace("tag:", "").trim(), "tag", false, index);
              return;
          }

          if (ref.includes("/")) {
              pushBadge(ref, "remote", false, index);
              return;
          }

          pushBadge(ref, "branch", false, index);
      });

      const priority = (badge: RefBadge) => {
          if (badge.isCurrent) return 300;
          if (badge.type === "branch") return 200;
          if (badge.type === "remote") return 100;
          return 50;
      };

      // Hide remote tracking refs like origin/main when local main exists on the same commit.
      const localBranchNames = new Set(
          parsed
              .filter((badge) => badge.type === "branch")
              .map((badge) => badge.text.toLowerCase())
      );

      const withoutDuplicatedRemotes = parsed.filter((badge) => {
          if (badge.type !== "remote") return true;
          const trackingName = badge.text.split("/").slice(1).join("/").trim().toLowerCase();
          if (!trackingName) return true;
          return !localBranchNames.has(trackingName);
      });

      return withoutDuplicatedRemotes.sort((a, b) => {
          const byPriority = priority(b) - priority(a);
          if (byPriority !== 0) return byPriority;
          return a.originalIndex - b.originalIndex;
      });
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

  async function handleCommitContextAction(action: CommitContextMenuAction, menu: CommitContextMenuState) {
      const shortHash = menu.node.hash.slice(0, 8);

      try {
          switch (action.type) {
              case "pull":
                  await handlePull();
                  return;
              case "push":
                  await handlePush();
                  return;
              case "fetch":
                  await handleFetch();
                  return;
              case "set-upstream": {
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

                  const res = await GitService.setUpstream(branchName, upstream, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "checkout-local":
                  await checkoutFromBadge({
                      text: action.branch,
                      type: "branch",
                      isCurrent: action.branch === currentBranchName,
                      originalIndex: 0
                  });
                  return;
              case "checkout-remote":
                  await checkoutFromBadge({
                      text: action.remoteRef,
                      type: "remote",
                      isCurrent: false,
                      originalIndex: 0
                  });
                  return;
              case "checkout-detached": {
                  if (!repoPath) return;
                  const confirmed = await confirm({
                      title: "Detached HEAD Checkout",
                      message: `Checkout commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span> in detached HEAD state?`,
                      isHtmlMessage: true,
                      confirmLabel: "Checkout",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;

                  const res = await GitService.checkout(menu.node.hash, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "create-branch-here": {
                  if (!repoPath) return;

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

                  const res = await GitService.createBranch(branchName, menu.node.hash, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "reset": {
                  if (!repoPath) return;

                  const resetLabel = action.mode.toUpperCase();
                  const resetConfirm = action.mode === "hard"
                      ? await confirm({
                          title: "Hard Reset",
                          message: `Hard reset to <code>${shortHash}</code>?<br/><br/><strong class="text-[#f85149]">Warning: all uncommitted changes will be permanently lost.</strong>`,
                          isHtmlMessage: true,
                          confirmLabel: "Hard Reset",
                          cancelLabel: "Cancel"
                      })
                      : await confirm({
                          title: `${resetLabel} Reset`,
                          message: `${resetLabel} reset to commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?`,
                          isHtmlMessage: true,
                          confirmLabel: "Reset",
                          cancelLabel: "Cancel"
                      });

                  if (!resetConfirm) return;

                  const res = await GitService.resetToCommit(menu.node.hash, action.mode, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "revert": {
                  if (!repoPath) return;
                  const confirmed = await confirm({
                      title: "Revert Commit",
                      message: `Revert commit <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?<br/><em>"${menu.node.subject}"</em><br/><br/>This creates a new commit that undoes this commit.`,
                      isHtmlMessage: true,
                      confirmLabel: "Revert",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;

                  const res = await GitService.revertCommit(menu.node.hash, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "rename-branch": {
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

                  const res = await GitService.renameBranch(oldName, newName, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "delete-local-branch": {
                  if (!repoPath) return;
                  const targetBranch = action.branch.trim();
                  if (!targetBranch) return;

                  const confirmed = await confirm({
                      title: "Delete Local Branch",
                      message: `Delete local branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${targetBranch}</span>?`,
                      isHtmlMessage: true,
                      confirmLabel: "Delete",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;

                  const res = await GitService.deleteBranch(targetBranch, false, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "delete-remote-branch": {
                  if (!repoPath) return;
                  const parsed = parseRemoteRef(action.remoteRef);
                  if (!parsed) {
                      toast.error("Invalid remote branch reference");
                      return;
                  }

                  const confirmed = await confirm({
                      title: "Delete Remote Branch",
                      message: `Delete remote branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${action.remoteRef}</span>?`,
                      isHtmlMessage: true,
                      confirmLabel: "Delete",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;

                  const res = await GitService.deleteRemoteBranch(parsed.remote, parsed.branch, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "delete-local-and-remote": {
                  if (!repoPath) return;
                  const parsed = parseRemoteRef(action.remoteRef);
                  if (!parsed) {
                      toast.error("Invalid remote branch reference");
                      return;
                  }

                  const confirmed = await confirm({
                      title: "Delete Local and Remote Branch",
                      message: `Delete local branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${action.branch}</span> and remote branch <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${action.remoteRef}</span>?`,
                      isHtmlMessage: true,
                      confirmLabel: "Delete Both",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;

                  const remoteRes = await GitService.deleteRemoteBranch(parsed.remote, parsed.branch, repoPath);
                  if (!remoteRes.success) return;

                  const localRes = await GitService.deleteBranch(action.branch, false, repoPath);
                  if (localRes.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
              case "copy-commit-sha":
                  await navigator.clipboard.writeText(menu.node.hash);
                  toast.success(`Copied: ${shortHash}`);
                  return;
              case "copy-branch-name":
                  await navigator.clipboard.writeText(action.branch);
                  toast.success(`Copied: ${action.branch}`);
                  return;
              case "create-patch-from-commit": {
                  if (!repoPath) return;
                  const patch = await GitService.createPatchFromCommit(menu.node.hash, repoPath);
                  if (!patch.trim()) {
                      toast.error("No patch content available for this commit");
                      return;
                  }
                  await navigator.clipboard.writeText(patch);
                  toast.success(`Copied patch for ${shortHash}`);
                  return;
              }
              case "create-tag": {
                  if (!repoPath) return;

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

                  const res = await GitService.createTag(tagName, menu.node.hash, tagMessage, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      await loadToolbarBranches();
                  }
                  return;
              }
          }
      } catch (e) {
          console.error("Commit context menu action failed", e);
      }
  }

  async function handleStashCommitContextAction(
      action: StashCommitContextMenuAction,
      menu: StashCommitContextMenuState
  ) {
      if (!repoPath) return;
      const shortHash = menu.node.hash.slice(0, 8);

      try {
          switch (action.type) {
              case "apply-stash": {
                  const res = await GitService.applyStash(menu.node.hash, repoPath);
                  if (res.success) {
                      const hasConflicts = await GitService.checkConflictState(repoPath).catch(() => false);
                      if (hasConflicts) {
                          toast.error("Stash applied with conflicts. Open the Conflicts tab to resolve.");
                      }
                  }
                  return;
              }
              case "pop-stash": {
                  const confirmed = await confirm({
                      title: "Pop Stash",
                      message: `Pop stash <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?<br/><br/>This applies changes and removes the stash entry.`,
                      isHtmlMessage: true,
                      confirmLabel: "Pop Stash",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;

                  const res = await GitService.popStash(menu.node.hash, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                      const hasConflicts = await GitService.checkConflictState(repoPath).catch(() => false);
                      if (hasConflicts) {
                          toast.error("Stash popped with conflicts. Open the Conflicts tab to resolve.");
                      }
                  }
                  return;
              }
              case "delete-stash": {
                  const confirmed = await confirm({
                      title: "Delete Stash",
                      message: `Delete stash <span class="font-mono text-[#58a6ff] bg-[#1f6feb]/10 px-1 rounded">${shortHash}</span>?`,
                      isHtmlMessage: true,
                      confirmLabel: "Delete Stash",
                      cancelLabel: "Cancel"
                  });
                  if (!confirmed) return;

                  const res = await GitService.deleteStash(menu.node.hash, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                  }
                  return;
              }
              case "edit-stash-message": {
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

                  const res = await GitService.editStashMessage(menu.node.hash, newMessage, repoPath);
                  if (res.success) {
                      await onGraphReload?.();
                  }
                  return;
              }
              case "share-stash-cloud-patch": {
                  const patch = await GitService.createPatchFromStash(menu.node.hash, repoPath);
                  if (!patch.trim()) {
                      toast.error("No patch content available for this stash");
                      return;
                  }
                  await navigator.clipboard.writeText(patch);
                  toast.success(`Cloud Patch not configured, copied stash patch ${shortHash} instead`);
                  return;
              }
              case "hide":
                  return;
          }
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

  function handleBranchBadgeContextMenu(event: MouseEvent, badge: RefBadge) {
      event.preventDefault();
      event.stopPropagation();
      if (!canCheckoutFromBadge(badge)) return;
      closeChangedFileContextMenu();
      closeCommitContextMenu();
      closeStashCommitContextMenu();
      branchContextMenu = {
          x: event.clientX,
          y: event.clientY,
          payload: badge,
          disableMerge: isCurrentBranchBadge(badge)
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

      isBranchCheckoutLoading = true;
      try {
          const res = await GitService.merge(branchRef, repoPath);
          if (res.success) {
              await onGraphReload?.();
              await loadToolbarBranches();
          }
      } catch (e) {
          console.error("Failed to merge from graph branch badge", e);
      } finally {
          isBranchCheckoutLoading = false;
      }
  }

  async function handleBranchContextCheckout(payload: unknown) {
      await checkoutFromBadge(payload as RefBadge);
  }

  async function handleBranchContextMerge(payload: unknown) {
      await mergeFromBadge(payload as RefBadge);
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
                    <div class="{HEADER_BASE} px-2 justify-between">
                        <div class="flex items-center gap-2 overflow-hidden flex-1 mr-4">
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
        <div class="{HEADER_BASE} px-2 relative justify-center">
            <div class="absolute left-2 top-1/2 -translate-y-1/2 min-w-0 max-w-[44%]">
                <div class="inline-flex items-center gap-1.5 max-w-full px-2 py-1 rounded border border-[#1e293b] bg-[#0f172a]/80">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#8b949e" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <line x1="6" y1="3" x2="6" y2="15"></line>
                        <circle cx="18" cy="6" r="3"></circle>
                        <circle cx="6" cy="18" r="3"></circle>
                        <path d="M18 9a9 9 0 0 1-9 9"></path>
                    </svg>
                    <span class="text-[10px] uppercase tracking-wider text-[#8b949e] shrink-0">Current</span>
                    <div class="relative min-w-[126px] max-w-[240px]">
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
                                {#each toolbarLocalBranches as branch}
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
            <div class="flex items-center gap-1">
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

            <div class="absolute right-2 top-1/2 -translate-y-1/2 z-50">
                <button  
                    onclick={() => showMenu = !showMenu}
                    class="text-xs text-[#8b949e] hover:text-[#c9d1d9] px-2 py-1 rounded hover:bg-[#1e293b] flex items-center gap-1 transition-colors"
                >
                    <span>Columns</span>
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
                </button>

                {#if showMenu}
                    <div class="absolute top-8 right-0 bg-[#111827] border border-[#1e293b] rounded-md shadow-xl z-[70] p-2 w-40 animate-in fade-in zoom-in-95 duration-100">
                        {#each columns as col}
                            <label class="flex items-center gap-2 p-1.5 hover:bg-[#1e293b] rounded cursor-pointer text-xs text-[#c9d1d9]">
                                <input type="checkbox" bind:checked={col.visible} class="rounded border-[#1e293b] bg-[#0f172a] text-[#238636] focus:ring-0">
                                {col.label}
                            </label>
                        {/each}
                    </div>
                {/if}
            </div>

            {#if showMenu}
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

        <div class="flex-1 overflow-auto custom-scrollbar relative" bind:this={graphViewportEl} onscroll={hideCommitTooltip}>
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
                        {#each visibleColumns as col}
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
                        {#each visibleColumns as col}
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
                                            oncontextmenu={(e) => handleBranchBadgeContextMenu(e, primaryBadge)}
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
                                        {#each secondaryBadges as badge}
                                            <div class="branch-dropdown-item">
                                                {#if canCheckoutFromBadge(badge)}
                                                    <button
                                                        type="button"
                                                        class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0 bg-transparent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-sky-500/80 cursor-pointer hover:brightness-110 {getRefBadgeClass(badge)}"
                                                        title={`${badge.text} (double-click to checkout)`}
                                                        onclick={(e) => e.stopPropagation()}
                                                        ondblclick={(e) => handleBranchBadgeClick(e, badge)}
                                                        oncontextmenu={(e) => handleBranchBadgeContextMenu(e, badge)}
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
      <ResizablePanel side="left" initialSize={320} minSize={200} maxSize={600}>
          <div class="h-full flex flex-col bg-[#0f172a] border-l border-[#1e293b]">
              <!-- Header -->
              <div class="{HEADER_BASE} justify-between px-2">
                  <span class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider">{isWipRowSelected ? "Working Changes" : "Commit Details"}</span>
                  <button class="text-[#8b949e] hover:text-white p-1 rounded" onclick={closeDetails} title="Close">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                  </button>
              </div>
              
              <div class="flex-1 overflow-auto p-3 custom-scrollbar">
                  <!-- Metadata -->
                  <div class="mb-4 space-y-2">
                       {#if isWipRowSelected}
                           <div class="select-text font-mono text-[10px] text-[#8b949e] bg-[#111827] p-1.5 rounded border border-[#1e293b]">
                               // WIP
                           </div>

                           <div class="text-sm font-medium text-[#c9d1d9] leading-tight py-1">
                               Uncommitted working tree changes
                           </div>

                           <div class="flex items-center gap-2 text-[11px]">
                               <span class="px-2 py-0.5 rounded border border-emerald-700/50 bg-emerald-900/30 text-emerald-300">
                                   +{wipSummary.stagedCount} staged
                               </span>
                               <span class="px-2 py-0.5 rounded border border-amber-700/50 bg-amber-900/30 text-amber-300">
                                   +{wipSummary.unstagedCount} unstaged
                               </span>
                               <span class="text-[#8b949e]">{wipSummary.totalCount} file(s)</span>
                           </div>

                           {#if onNavigateToCommitPanel}
                               <div class="pt-1">
                                   <button
                                       type="button"
                                       class="px-3 py-1.5 text-xs font-medium text-white bg-[#1f6feb] hover:bg-[#388bfd] rounded border border-[rgba(240,246,252,0.1)] transition-colors"
                                       onclick={navigateToCommitPanel}
                                   >
                                       Go to Commit panel
                                   </button>
                               </div>
                           {/if}
                       {:else if selectedCommit}
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
                              {isWipRowSelected ? "Working tree is clean" : "No changes found"}
                          </div>
                      {/if}
                  </div>
              </div>
          </div>
      </ResizablePanel>
  {/if}
</div>

{#if changedFileContextMenu.visible}
  <div
    class="fixed z-[70] bg-[#1f2428] border border-[#30363d] rounded-md shadow-xl py-1 min-w-[190px] changed-file-context-menu"
    style="top: {changedFileContextMenu.y}px; left: {changedFileContextMenu.x}px;"
    role="menu"
    tabindex="-1"
    oncontextmenu={(e) => {
      e.preventDefault();
      e.stopPropagation();
    }}
  >
    <button
      class="w-full text-left px-4 py-2 text-xs text-[#c9d1d9] hover:bg-[#1f6feb] hover:text-white flex items-center gap-2"
      onclick={() => void handleCopyChangedFilePath()}
    >
      <span>Copy file path</span>
    </button>
    {#if repoPath}
      <button
      class="w-full text-left px-4 py-2 text-xs text-[#c9d1d9] hover:bg-[#1f6feb] hover:text-white flex items-center gap-2"
      onclick={() => void handleOpenChangedFile()}
    >
      <span>Open file</span>
    </button>
    {/if}
  </div>
{/if}

<BranchContextMenu
  menu={branchContextMenu}
  onClose={closeBranchContextMenu}
  onCheckout={handleBranchContextCheckout}
  onMerge={handleBranchContextMerge}
/>

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

