<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { GitService, type FileStatus, type GitOperationState } from "../../lib/GitService";
  import { toast } from "../../lib/toast.svelte";
  import { confirm } from "../../lib/confirmation.svelte";
  import { rebaseStore } from "../../lib/rebaseStore";

  import CommitFileList from "./CommitFileList.svelte";
  import CommitActions from "./CommitActions.svelte";
  import ConflictResolveModal from "./ConflictResolveModal.svelte";
  import FileChangeStatusBadge from "../common/FileChangeStatusBadge.svelte";

  interface Props {
    repoPath: string;
    onFileSelect?: (file: FileStatus) => void;
    onClose?: () => void;
    onCommitSuccess?: () => void;
    onShowHistory?: (filePath: string) => void;
    onShowBlame?: (filePath: string) => void;
  }

  let {
    repoPath,
    onFileSelect,
    onClose,
    onCommitSuccess,
    onShowHistory,
    onShowBlame,
  }: Props = $props();

  // State
  let stagedFiles = $state<FileStatus[]>([]);
  let unstagedFiles = $state<FileStatus[]>([]);
  let selectedFile = $state<FileStatus | null>(null);
  let loadingStatus = $state(false);
  let conflictPaths = $state<Set<string>>(new Set());
  let fileViewMode = $state<"tree" | "path">("path");
  let commitMessage = $state("");
  let resolvingConflictFilePath = $state<string | null>(null);

  type CommitActionState = "idle" | "committing" | "generatingMessage" | "aborting";
  let commitActionState = $state<CommitActionState>("idle");
  let committing = $derived(commitActionState === "committing");
  let abortingOperation = $derived(commitActionState === "aborting");
  let generatingCommitMessage = $derived(commitActionState === "generatingMessage");

  const DEFAULT_OPERATION_STATE: GitOperationState = {
    isMerging: false,
    isRebasing: false,
    isCherryPicking: false,
    isReverting: false,
    hasConflicts: false,
    conflictPaths: [],
    oursCommit: null,
    oursBranch: null,
    theirsCommit: null,
    theirsBranch: null,
  };
  let operationState = $state<GitOperationState>(DEFAULT_OPERATION_STATE);

  // Resizable splitter
  let fileListsContainerEl = $state<HTMLDivElement | null>(null);
  let fileSplitTopHeight = $state<number | null>(null);
  let isFileSplitDragging = $state(false);
  let fileListsResizeObserver: ResizeObserver | null = null;
  const FILE_LIST_MIN_SECTION_HEIGHT = 60;
  const FILE_VIEW_MODE_KEY = "graph_wip_file_view_mode";
  let statusLoadInFlight = false;
  let pendingStatusRefresh = false;

  // --- Utility functions ---

  function resolvePathForActions(path: string): string {
    const normalized = path.replaceAll("\\", "/").trim();
    const parts = normalized.split(" -> ");
    return (parts[parts.length - 1] ?? normalized).trim();
  }

  function normalizeOperationState(state: GitOperationState | null | undefined): GitOperationState {
    if (!state) return DEFAULT_OPERATION_STATE;
    return {
      isMerging: !!state.isMerging,
      isRebasing: !!state.isRebasing,
      isCherryPicking: !!state.isCherryPicking,
      isReverting: !!state.isReverting,
      hasConflicts: !!state.hasConflicts,
      conflictPaths: (state.conflictPaths ?? []).map((p) => resolvePathForActions(p)),
      oursCommit: state.oursCommit ?? null,
      oursBranch: state.oursBranch ?? null,
      theirsCommit: state.theirsCommit ?? null,
      theirsBranch: state.theirsBranch ?? null,
    };
  }

  function mergeStatusFilesWithConflictPaths(
    statusFiles: FileStatus[],
    conflictFilePaths: string[]
  ): FileStatus[] {
    if (conflictFilePaths.length === 0) return statusFiles;
    const merged = [...statusFiles];
    const existing = new Set(statusFiles.map((f) => resolvePathForActions(f.path)));
    for (const conflictPath of conflictFilePaths) {
      const normalized = resolvePathForActions(conflictPath);
      if (!normalized || existing.has(normalized)) continue;
      merged.push({ path: normalized, status: "U", staged: false });
      existing.add(normalized);
    }
    return merged;
  }

  function getUniqueFilesForDiscard(files: FileStatus[]): FileStatus[] {
    const uniqueByPath = new Map<string, FileStatus>();
    for (const file of files) {
      const existing = uniqueByPath.get(file.path);
      if (!existing) {
        uniqueByPath.set(file.path, file);
        continue;
      }
      if (existing.status === "??" && file.status !== "??") {
        uniqueByPath.set(file.path, file);
      }
    }
    return [...uniqueByPath.values()];
  }

  // --- Data loading ---

  async function loadStatus() {
    if (!repoPath) return;
    if (statusLoadInFlight) {
      pendingStatusRefresh = true;
      return;
    }

    statusLoadInFlight = true;
    loadingStatus = true;
    const requestRepoPath = repoPath;
    try {
      const [files, conflicts, nextOperationState] = await Promise.all([
        GitService.getStatusFiles(requestRepoPath),
        GitService.getConflicts(requestRepoPath).catch(() => [] as string[]),
        GitService.getOperationState(requestRepoPath).catch(() => DEFAULT_OPERATION_STATE),
      ]);

      if (repoPath !== requestRepoPath) {
        return;
      }

      const normalizedOperationState = normalizeOperationState(nextOperationState);
      operationState = normalizedOperationState;

      const conflictCandidates =
        normalizedOperationState.conflictPaths.length > 0
          ? normalizedOperationState.conflictPaths
          : conflicts.map((p) => resolvePathForActions(p));

      const mergedFiles = mergeStatusFilesWithConflictPaths(files, conflictCandidates);
      stagedFiles = mergedFiles.filter((f) => f.staged);
      unstagedFiles = mergedFiles.filter((f) => !f.staged);
      conflictPaths = new Set(conflictCandidates.map((p) => resolvePathForActions(p)));
    } catch (e: any) {
      console.error("Failed to load status:", e);
      conflictPaths = new Set();
      operationState = DEFAULT_OPERATION_STATE;
    } finally {
      loadingStatus = false;
      statusLoadInFlight = false;
      if (pendingStatusRefresh) {
        pendingStatusRefresh = false;
        void loadStatus();
      }
    }
  }

  // --- File actions ---

  function handleSelect(file: FileStatus) {
    selectedFile = file;
    onFileSelect?.(file);
  }

  async function handleStage(file: FileStatus) {
    if (!repoPath) return;
    try {
      await GitService.stageFile(file.path, repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled in service */ }
  }

  async function handleUnstage(file: FileStatus) {
    if (!repoPath) return;
    try {
      await GitService.unstageFile(file.path, repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled in service */ }
  }

  async function handleStageAll() {
    if (!repoPath) return;
    try {
      await GitService.stageAll(repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  async function handleUnstageAll() {
    if (!repoPath) return;
    try {
      await GitService.unstageAll(repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  async function handleDiscardFile(file: FileStatus) {
    if (!repoPath) return;
    const confirmed = await confirm({
      title: "Discard Change",
      message: `Discard all changes in "${file.path}"?\nThis action cannot be undone.`,
      confirmLabel: "Discard",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    try {
      await GitService.discardChanges([file], repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  async function handleDiscardAll() {
    if (!repoPath) return;
    const files = getUniqueFilesForDiscard([...unstagedFiles, ...stagedFiles]);
    if (files.length === 0) return;
    const confirmed = await confirm({
      title: "Discard All Changes",
      message: `Discard all local changes in ${files.length} file(s)?\nThis action cannot be undone.`,
      confirmLabel: "Discard All",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    try {
      await GitService.discardChanges(files, repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  async function handleStashFile(file: FileStatus) {
    if (!repoPath) return;
    try {
      await GitService.stashFile(file, repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  async function handleStashAll() {
    if (!repoPath) return;
    try {
      await GitService.stashAll(repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  async function handleOpenFile(file: FileStatus) {
    if (!repoPath) return;
    try {
      await GitService.openRepoFile(file.path, repoPath);
    } catch (_e) { /* toast handled */ }
  }

  async function handleIgnoreFile(pattern: string) {
    if (!repoPath) return;
    try {
      await GitService.ignoreFile(pattern, repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  function handleShowFileHistory(file: FileStatus) {
    const targetPath = resolvePathForActions(file.path);
    onShowHistory?.(targetPath);
  }

  function handleShowFileBlame(file: FileStatus) {
    const targetPath = resolvePathForActions(file.path);
    onShowBlame?.(targetPath);
  }

  async function handleOpenInDiffTool(file: FileStatus) {
    if (!repoPath) return;
    try {
      await GitService.openInDiffTool(file.path, file.staged, repoPath);
    } catch (_e) { /* toast handled */ }
  }

  async function handleOpenInEditor(file: FileStatus) {
    if (!repoPath) return;
    try {
      await GitService.openInEditor(file.path, repoPath);
    } catch (_e) { /* toast handled */ }
  }

  async function handleShowInFolder(file: FileStatus) {
    if (!repoPath) return;
    try {
      await GitService.showInFolder(file.path, repoPath);
    } catch (_e) { /* toast handled */ }
  }

  async function handleCreatePatchFromFile(file: FileStatus) {
    if (!repoPath) return;
    try {
      const patchContent = await GitService.createPatch(file.path, file.staged, repoPath);
      if (!patchContent.trim()) {
        toast.error("No patch content available for this file");
        return;
      }
      await navigator.clipboard.writeText(patchContent);
      toast.success(`Copied patch for ${resolvePathForActions(file.path)}`);
    } catch (_e) { /* toast handled */ }
  }

  async function handleEditFile(file: FileStatus) {
    await handleOpenInEditor(file);
  }

  async function handleDeleteFile(file: FileStatus) {
    if (!repoPath) return;
    const targetPath = resolvePathForActions(file.path);
    const confirmed = await confirm({
      title: "Delete File",
      message: `Delete "${targetPath}" permanently?\nThis action cannot be undone.`,
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    try {
      await GitService.deleteFile(file.path, repoPath);
      await loadStatus();
    } catch (_e) { /* toast handled */ }
  }

  function handleResolveConflict(file: FileStatus) {
    resolvingConflictFilePath = resolvePathForActions(file.path);
  }

  function handleCloseConflictResolver() {
    resolvingConflictFilePath = null;
  }

  async function handleConflictResolved(_filePath: string) {
    resolvingConflictFilePath = null;
    await loadStatus();
  }

  // --- Commit actions ---

  async function handleCommit(message: string, push: boolean) {
    if (!repoPath || commitActionState !== "idle") return;
    commitActionState = "committing";
    try {
      await GitService.commit(message, repoPath);
      if (push) {
        await GitService.push(repoPath);
      }
      await loadStatus();
      selectedFile = null;
      onCommitSuccess?.();
    } catch (e: any) {
      throw e;
    } finally {
      commitActionState = "idle";
    }
  }

  async function handleGenerateCommitMessage() {
    if (!repoPath || stagedFiles.length === 0 || commitActionState !== "idle") return;
    commitActionState = "generatingMessage";
    try {
      commitMessage = await GitService.generateCommitMessage(repoPath);
      toast.success("Generated commit message from staged changes");
    } catch (e: any) {
      toast.error(`Generate message failed: ${e}`);
    } finally {
      commitActionState = "idle";
    }
  }

  async function handleAbortOperation() {
    if (!repoPath || commitActionState !== "idle") return;
    commitActionState = "aborting";
    try {
      if (operationState.isRebasing) {
        await rebaseStore.abort(repoPath);
      } else {
        await GitService.abortOperation(repoPath);
      }
      await loadStatus();
      selectedFile = null;
      onCommitSuccess?.();
    } finally {
      commitActionState = "idle";
    }
  }

  async function handleSkipRebase() {
    if (!repoPath || commitActionState !== "idle") return;
    commitActionState = "aborting";
    try {
      await rebaseStore.skip(repoPath);
      await loadStatus();
      onCommitSuccess?.();
    } finally {
      commitActionState = "idle";
    }
  }

  async function handleMarkAllResolved() {
    if (!repoPath) return;
    const paths = [...conflictPaths];
    if (paths.length === 0) return;
    try {
      for (const p of paths) {
        await GitService.markResolved(p, repoPath);
      }
      await loadStatus();
    } catch (e) {
      console.error("Failed to mark all resolved", e);
    }
  }

  // --- Resizable splitter ---

  function getFileSplitMetrics() {
    if (!fileListsContainerEl) return null;
    const rect = fileListsContainerEl.getBoundingClientRect();
    const totalHeight = rect.height;
    const minTop = Math.min(FILE_LIST_MIN_SECTION_HEIGHT, Math.max(0, totalHeight - FILE_LIST_MIN_SECTION_HEIGHT));
    const maxTop = Math.max(minTop, totalHeight - FILE_LIST_MIN_SECTION_HEIGHT);
    return { rect, totalHeight, minTop, maxTop };
  }

  function syncFileSplitHeight() {
    const metrics = getFileSplitMetrics();
    if (!metrics) return;
    const fallback = Math.max(metrics.minTop, Math.min(metrics.maxTop, Math.floor(metrics.totalHeight * 0.5)));
    if (fileSplitTopHeight === null) {
      fileSplitTopHeight = fallback;
      return;
    }
    fileSplitTopHeight = Math.max(metrics.minTop, Math.min(metrics.maxTop, fileSplitTopHeight));
  }

  function handleFileSplitPointerDown(event: PointerEvent) {
    if (!fileListsContainerEl) return;
    event.preventDefault();
    isFileSplitDragging = true;
    syncFileSplitHeight();
    const handle = event.currentTarget as HTMLElement | null;
    if (handle) handle.setPointerCapture(event.pointerId);
    document.body.classList.add("resizing-v");
  }

  function handleFileSplitPointerMove(event: PointerEvent) {
    if (!isFileSplitDragging) return;
    const metrics = getFileSplitMetrics();
    if (!metrics) return;
    const pointerTop = event.clientY - metrics.rect.top;
    fileSplitTopHeight = Math.max(metrics.minTop, Math.min(metrics.maxTop, pointerTop));
  }

  function handleFileSplitPointerUp(event: PointerEvent) {
    if (!isFileSplitDragging) return;
    isFileSplitDragging = false;
    const handle = event.currentTarget as HTMLElement | null;
    if (handle?.hasPointerCapture(event.pointerId)) {
      handle.releasePointerCapture(event.pointerId);
    }
    document.body.classList.remove("resizing-v");
  }

  // --- Derived state ---

  let isOperationInProgress = $derived(
    operationState.isMerging ||
    operationState.isRebasing ||
    operationState.isCherryPicking ||
    operationState.isReverting
  );

  let showAbortOperationButton = $derived(
    isOperationInProgress && (operationState.hasConflicts || conflictPaths.size > 0)
  );

  let abortOperationLabel = $derived.by(() => {
    if (operationState.isRebasing) return "Abort Rebase";
    if (operationState.isMerging) return "Abort Merge";
    if (operationState.isCherryPicking) return "Abort Cherry-pick";
    if (operationState.isReverting) return "Abort Revert";
    return "Abort Operation";
  });

  let primaryOperationLabel = $derived.by(() => {
    if (operationState.isMerging) return "Commit and Merge";
    if (operationState.isRebasing) return "Commit and Continue Rebase";
    if (operationState.isCherryPicking) return "Commit and Continue Cherry-pick";
    if (operationState.isReverting) return "Commit and Continue Revert";
    return "Commit";
  });

  let operationTypeLabel = $derived.by(() => {
    if (operationState.isRebasing) return "Rebase";
    if (operationState.isMerging) return "Merge";
    if (operationState.isCherryPicking) return "Cherry-pick";
    if (operationState.isReverting) return "Revert";
    return "";
  });

  // Conflict-specific derived state
  let conflictedFiles = $derived(
    [...unstagedFiles, ...stagedFiles].filter(f => conflictPaths.has(resolvePathForActions(f.path)))
  );

  let resolvedFiles = $derived(
    stagedFiles.filter(f => !conflictPaths.has(resolvePathForActions(f.path)))
  );

  let hasActiveConflicts = $derived(isOperationInProgress && conflictPaths.size > 0);

  // --- Lifecycle ---

  $effect(() => {
    const currentRepo = repoPath?.trim();
    if (!currentRepo) return;
    untrack(() => {
      void loadStatus();
    });
  });

  onMount(() => {
    const saved = localStorage.getItem(FILE_VIEW_MODE_KEY);
    if (saved === "tree" || saved === "path") {
      fileViewMode = saved;
    }
  });

  $effect(() => {
    localStorage.setItem(FILE_VIEW_MODE_KEY, fileViewMode);
  });

  onMount(() => {
    if (typeof ResizeObserver !== "undefined" && fileListsContainerEl) {
      fileListsResizeObserver = new ResizeObserver(() => syncFileSplitHeight());
      fileListsResizeObserver.observe(fileListsContainerEl);
    }
    syncFileSplitHeight();
    return () => {
      if (fileListsResizeObserver) {
        fileListsResizeObserver.disconnect();
        fileListsResizeObserver = null;
      }
      document.body.classList.remove("resizing-v");
    };
  });

  export function refresh() {
    loadStatus();
  }
</script>

<div class="h-full flex flex-col bg-[#0f172a] text-[#c9d1d9] overflow-hidden">

  {#if hasActiveConflicts}
    <!-- ==================== CONFLICT LAYOUT ==================== -->

    <!-- Conflict Banner -->
    <div class="shrink-0 px-3 py-2.5 bg-[#3a1b23] border-b border-[#f85149]/30">
      <div class="flex items-center gap-2 mb-1.5">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#f85149" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        <span class="text-xs font-semibold text-[#f85149]">{operationTypeLabel} conflicts detected</span>
        {#if onClose}
          <button class="ml-auto text-[#8b949e] hover:text-white p-0.5 rounded transition-colors" onclick={onClose} title="Close">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        {/if}
      </div>
      <div class="flex items-center gap-1.5 text-[11px] text-[#c9d1d9]">
        <span class="text-[#8b949e]">
          {#if operationState.isRebasing}Rebasing{:else if operationState.isMerging}Merging{:else if operationState.isCherryPicking}Cherry-picking{:else}Reverting{/if}
        </span>
        {#if operationState.oursBranch}
          <span class="px-1.5 py-0.5 rounded bg-[#1f6feb]/20 border border-[#1f6feb]/40 text-[#58a6ff] text-[10px] font-mono">{operationState.oursBranch}</span>
        {/if}
        {#if operationState.theirsBranch}
          <span class="text-[#8b949e]">
            {#if operationState.isRebasing}onto{:else}into{/if}
          </span>
          <span class="px-1.5 py-0.5 rounded bg-[#da3633]/20 border border-[#da3633]/40 text-[#ff7b72] text-[10px] font-mono">{operationState.theirsBranch}</span>
        {/if}
      </div>
    </div>

    <!-- View mode toggle bar -->
    <div class="shrink-0 h-8 px-2 flex items-center justify-between border-b border-[#1e293b] bg-[#111827]">
      <div class="flex items-center gap-2">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#8b949e" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="4" y1="21" x2="4" y2="14"/><line x1="4" y1="10" x2="4" y2="3"/>
          <line x1="12" y1="21" x2="12" y2="12"/><line x1="12" y1="8" x2="12" y2="3"/>
          <line x1="20" y1="21" x2="20" y2="16"/><line x1="20" y1="12" x2="20" y2="3"/>
          <line x1="1" y1="14" x2="7" y2="14"/><line x1="9" y1="8" x2="15" y2="8"/><line x1="17" y1="16" x2="23" y2="16"/>
        </svg>
      </div>
      <div class="inline-flex rounded border border-[#1e293b] overflow-hidden">
        <button type="button"
          class="px-2 py-0.5 text-[10px] font-medium transition-colors {fileViewMode === 'path' ? 'bg-[#1e293b] text-white' : 'bg-[#0f172a] text-[#8b949e] hover:text-[#c9d1d9]'}"
          onclick={() => fileViewMode = "path"}>Path</button>
        <button type="button"
          class="px-2 py-0.5 text-[10px] font-medium border-l border-[#1e293b] transition-colors {fileViewMode === 'tree' ? 'bg-[#1e293b] text-white' : 'bg-[#0f172a] text-[#8b949e] hover:text-[#c9d1d9]'}"
          onclick={() => fileViewMode = "tree"}>Tree</button>
      </div>
    </div>

    <!-- Conflict file lists -->
    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar">

      <!-- Conflicted Files Section -->
      <div class="border-b border-[#1e293b]">
        <div class="h-7 px-3 flex items-center justify-between bg-[#111827] shrink-0">
          <span class="text-[11px] font-semibold text-[#f85149] flex items-center gap-1.5">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="m6 9 6 6 6-6"/></svg>
            Conflicted Files ({conflictedFiles.length})
          </span>
          {#if conflictedFiles.length > 0}
            <button
              type="button"
              class="text-[10px] font-medium text-[#58a6ff] hover:text-[#79c0ff] px-1.5 py-0.5 rounded hover:bg-[#1f2f45] transition-colors"
              onclick={handleMarkAllResolved}
            >Mark All Resolved</button>
          {/if}
        </div>
        {#if conflictedFiles.length > 0}
          <div class="p-1">
            {#each conflictedFiles as file (file.path)}
              <div
                class="flex items-center gap-2 px-2 py-1.5 text-xs rounded cursor-pointer transition-colors hover:bg-[#111827] {selectedFile?.path === file.path ? 'bg-[#1e293b] text-white' : ''}"
                onclick={() => handleSelect(file)}
                role="button"
                tabindex="0"
              >
                <FileChangeStatusBadge status="U" compact={true} showCode={true} className="shrink-0" />
                <span class="truncate text-[#c9d1d9] min-w-0">{resolvePathForActions(file.path)}</span>
                <button
                  type="button"
                  class="ml-auto shrink-0 text-[10px] font-medium text-[#8b949e] hover:text-[#58a6ff] px-1.5 py-0.5 rounded hover:bg-[#1f2f45] transition-colors"
                  onclick={(e) => { e.stopPropagation(); handleResolveConflict(file); }}
                >Resolve</button>
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-[11px] text-[#8b949e] italic text-center py-3">No conflicted files</div>
        {/if}
      </div>

      <!-- Resolved Files Section -->
      <div>
        <div class="h-7 px-3 flex items-center bg-[#111827] shrink-0">
          <span class="text-[11px] font-semibold text-[#3fb950] flex items-center gap-1.5">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="m6 9 6 6 6-6"/></svg>
            Resolved Files ({resolvedFiles.length})
          </span>
        </div>
        {#if resolvedFiles.length > 0}
          <div class="p-1">
            {#each resolvedFiles as file (file.path)}
              <div
                class="flex items-center gap-2 px-2 py-1.5 text-xs rounded cursor-pointer transition-colors hover:bg-[#111827] {selectedFile?.path === file.path ? 'bg-[#1e293b] text-white' : ''}"
                onclick={() => handleSelect(file)}
                role="button"
                tabindex="0"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#3fb950" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                <span class="truncate text-[#c9d1d9] min-w-0">{resolvePathForActions(file.path)}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-[11px] text-[#8b949e] italic text-center py-3">No resolved files yet</div>
        {/if}
      </div>
    </div>

    <!-- Commit section for conflict mode -->
    <div class="shrink-0 border-t border-[#30363d] bg-[#1c2128]">
      <CommitActions
        stagedCount={stagedFiles.length}
        busy={committing || abortingOperation || loadingStatus}
        generating={generatingCommitMessage}
        bind:message={commitMessage}
        onCommit={handleCommit}
        onGenerate={handleGenerateCommitMessage}
        showAbortOperation={true}
        primaryActionLabel={primaryOperationLabel}
        {abortOperationLabel}
        onAbortOperation={handleAbortOperation}
      />
      {#if operationState.isRebasing && commitActionState === 'idle'}
        <div class="px-1.5 pb-2">
          <button
            type="button"
            class="w-full h-8 px-3 rounded-sm border border-[#30363d] bg-[#21262d] text-[#c9d1d9] hover:bg-[#30363d] transition-colors text-[12px] font-semibold disabled:opacity-45 disabled:cursor-not-allowed"
            onclick={handleSkipRebase}
            disabled={abortingOperation || committing}
          >Skip Current Commit</button>
        </div>
      {/if}
    </div>

  {:else}
    <!-- ==================== NORMAL LAYOUT ==================== -->

    <!-- Header -->
    <div class="h-8 flex items-center justify-between px-2 bg-[#111827] border-b border-[#1e293b] shrink-0">
      <div class="flex items-center gap-2">
        <span class="text-xs font-semibold text-[#8b949e] uppercase tracking-wider">Working Changes</span>
        {#if loadingStatus}
          <svg class="animate-spin h-3 w-3 text-[#8b949e]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
          </svg>
        {/if}
      </div>
      <div class="flex items-center gap-1">
        <div class="inline-flex rounded border border-[#1e293b] overflow-hidden">
          <button type="button"
            class="px-2 py-0.5 text-[10px] font-medium transition-colors {fileViewMode === 'tree' ? 'bg-[#1e293b] text-white' : 'bg-[#0f172a] text-[#8b949e] hover:text-[#c9d1d9]'}"
            onclick={() => fileViewMode = "tree"} title="Tree view">Tree</button>
          <button type="button"
            class="px-2 py-0.5 text-[10px] font-medium border-l border-[#1e293b] transition-colors {fileViewMode === 'path' ? 'bg-[#1e293b] text-white' : 'bg-[#0f172a] text-[#8b949e] hover:text-[#c9d1d9]'}"
            onclick={() => fileViewMode = "path"} title="Path view">Path</button>
        </div>
        {#if onClose}
          <button class="text-[#8b949e] hover:text-white p-1 rounded transition-colors" onclick={onClose} title="Close">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        {/if}
      </div>
    </div>

    <!-- File lists (resizable) -->
    <div bind:this={fileListsContainerEl} class="flex-1 min-h-0 flex flex-col overflow-hidden">
      <!-- Changes (Unstaged) -->
      <div
        class="min-h-0 shrink-0 flex flex-col"
        style={fileSplitTopHeight === null ? "height: 50%;" : `height: ${fileSplitTopHeight}px;`}
      >
        <CommitFileList
          title="Changes"
          files={unstagedFiles}
          {selectedFile}
          onSelect={handleSelect}
          onAction={handleStage}
          onOpenFile={handleOpenFile}
          actionLabel="Stage"
          onDiscard={handleDiscardFile}
          onStash={handleStashFile}
          onIgnore={handleIgnoreFile}
          onShowHistory={handleShowFileHistory}
          onShowBlame={handleShowFileBlame}
          onOpenInDiffTool={handleOpenInDiffTool}
          onOpenInEditor={handleOpenInEditor}
          onShowInFolder={handleShowInFolder}
          onCreatePatch={handleCreatePatchFromFile}
          onEditFile={handleEditFile}
          onDeleteFile={handleDeleteFile}
          onStashAll={handleStashAll}
          stashAllLabel="Stash All"
          showStashAll={unstagedFiles.length + stagedFiles.length > 0}
          onDiscardAll={handleDiscardAll}
          discardAllLabel="Discard All"
          showDiscardAll={unstagedFiles.length + stagedFiles.length > 0}
          viewMode={fileViewMode}
          {conflictPaths}
          onResolveConflict={handleResolveConflict}
          onActionAll={handleStageAll}
          actionAllLabel="Stage All"
        />
      </div>

      <!-- Splitter -->
      <div
        class="relative h-1.5 shrink-0 cursor-row-resize select-none z-10 group"
        onpointerdown={handleFileSplitPointerDown}
        onpointermove={handleFileSplitPointerMove}
        onpointerup={handleFileSplitPointerUp}
        onpointercancel={handleFileSplitPointerUp}
        role="separator"
        aria-orientation="horizontal"
        tabindex="-1"
        title="Resize Changes and Staged Changes"
      >
        <div
          class="absolute left-0 right-0 top-1/2 -translate-y-1/2 h-px transition-colors
                 {isFileSplitDragging ? 'bg-[#58a6ff]' : 'bg-[#1e293b] group-hover:bg-[#484f58]'}"
        ></div>
      </div>

      <!-- Staged Changes -->
      <div class="min-h-0 flex-1 flex flex-col border-t border-[#1e293b]">
        <CommitFileList
          title="Staged Changes"
          files={stagedFiles}
          {selectedFile}
          onSelect={handleSelect}
          onAction={handleUnstage}
          onOpenFile={handleOpenFile}
          actionLabel="Unstage"
          onDiscard={handleDiscardFile}
          onStash={handleStashFile}
          onIgnore={handleIgnoreFile}
          onShowHistory={handleShowFileHistory}
          onShowBlame={handleShowFileBlame}
          onOpenInDiffTool={handleOpenInDiffTool}
          onOpenInEditor={handleOpenInEditor}
          onShowInFolder={handleShowInFolder}
          onCreatePatch={handleCreatePatchFromFile}
          onEditFile={handleEditFile}
          onDeleteFile={handleDeleteFile}
          viewMode={fileViewMode}
          {conflictPaths}
          onResolveConflict={handleResolveConflict}
          onActionAll={handleUnstageAll}
          actionAllLabel="Unstage All"
        />
      </div>
    </div>

    <!-- Commit Actions -->
    <div class="shrink-0">
      <CommitActions
        stagedCount={stagedFiles.length}
        busy={committing || abortingOperation || loadingStatus}
        generating={generatingCommitMessage}
        bind:message={commitMessage}
        onCommit={handleCommit}
        onGenerate={handleGenerateCommitMessage}
        showAbortOperation={showAbortOperationButton}
        primaryActionLabel={primaryOperationLabel}
        {abortOperationLabel}
        onAbortOperation={handleAbortOperation}
      />
      {#if operationState.isRebasing && !operationState.hasConflicts && commitActionState === 'idle'}
        <div class="px-1.5 pb-2 bg-[#1c2128]">
          <button
            type="button"
            class="w-full h-8 px-3 rounded-sm border border-[#30363d] bg-[#21262d] text-[#c9d1d9] hover:bg-[#30363d] transition-colors text-[12px] font-semibold"
            onclick={handleSkipRebase}
          >Skip Current Commit</button>
        </div>
      {/if}
    </div>

  {/if}
</div>

{#if resolvingConflictFilePath}
  <ConflictResolveModal
    {repoPath}
    filePath={resolvingConflictFilePath}
    {operationState}
    onClose={handleCloseConflictResolver}
    onResolved={handleConflictResolved}
  />
{/if}

<style>
  :global(body.resizing-v) {
    user-select: none !important;
    cursor: row-resize !important;
  }
</style>
