<script lang="ts">
  import { onMount } from "svelte";
  import { GitService, type FileStatus, type GitOperationState } from "../lib/GitService";
  import { toast } from "../lib/toast.svelte";
  import { computeDiff, isLargeFile, extractHunks, type DiffResult, type DiffHunk, type DiffStageLineTarget } from "../lib/diff";
  import { confirm } from "../lib/confirmation.svelte";
  import { rebaseStore } from "../lib/rebaseStore";

  import CommitFileList from "./commit/CommitFileList.svelte";
  import ConflictResolveModal from "./commit/ConflictResolveModal.svelte";

  import DiffToolbar from "./diff/DiffToolbar.svelte";
  import type { ViewMode } from "./diff/DiffToolbar.svelte";
  import CommitActions from "./commit/CommitActions.svelte";
  import DiffView from "./diff/DiffView.svelte";
  import FileChangeStatusBadge from "./common/FileChangeStatusBadge.svelte";

  interface Props {
      repoPath?: string;
      isActive?: boolean;
      selectedFile?: FileStatus | null;
      onShowHistory?: (filePath: string) => void;
      onShowBlame?: (filePath: string) => void;
  }
  let {
      repoPath,
      isActive = false,
      selectedFile = $bindable(null),
      onShowHistory,
      onShowBlame
  }: Props = $props();

  let stagedFiles = $state<FileStatus[]>([]);
  let unstagedFiles = $state<FileStatus[]>([]);
  // selectedFile is now a prop
  let baseContent = $state<string>("");
  let modifiedContent = $state<string>("");
  let loadingStatus = $state(false);
  let loadingDiff = $state(false);
  type CommitActionState = "idle" | "committing" | "generatingMessage" | "aborting";
  let commitActionState = $state<CommitActionState>("idle");
  let committing = $derived(commitActionState === "committing");
  let abortingOperation = $derived(commitActionState === "aborting");
  let generatingCommitMessage = $derived(commitActionState === "generatingMessage");
  let commitMessage = $state("");
  let selectedEncoding = $state<string | undefined>(undefined);
  let fileViewMode = $state<"tree" | "path">("path");
  let conflictPaths = $state<Set<string>>(new Set());
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
  let resolvingConflictFilePath = $state<string | null>(null);
  let fileListsContainerEl = $state<HTMLDivElement | null>(null);
  let fileListsResizeObserver: ResizeObserver | null = null;
  let fileSplitTopHeight = $state<number | null>(null);
  let isFileSplitDragging = $state(false);

  const FILE_VIEW_MODE_KEY = "commit_panel_file_view_mode";
  const FILE_LIST_MIN_SECTION_HEIGHT = 80;

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
          conflictPaths: (state.conflictPaths ?? []).map((path) => resolvePathForActions(path)),
          oursCommit: state.oursCommit ?? null,
          oursBranch: state.oursBranch ?? null,
          theirsCommit: state.theirsCommit ?? null,
          theirsBranch: state.theirsBranch ?? null
      };
  }

  function mergeStatusFilesWithConflictPaths(
      statusFiles: FileStatus[],
      conflictFilePaths: string[]
  ): FileStatus[] {
      if (conflictFilePaths.length === 0) return statusFiles;

      const merged = [...statusFiles];
      const existing = new Set(statusFiles.map((file) => resolvePathForActions(file.path)));

      for (const conflictPath of conflictFilePaths) {
          const normalized = resolvePathForActions(conflictPath);
          if (!normalized || existing.has(normalized)) continue;
          merged.push({
              path: normalized,
              status: "U",
              staged: false
          });
          existing.add(normalized);
      }

      return merged;
  }



  // Lift diff computation so all view modes share a single result
  let diffResult = $derived.by<DiffResult | null>(() => {
      if (!baseContent && !modifiedContent) return null;
      if (isLargeFile(baseContent) || isLargeFile(modifiedContent)) return null;
      return computeDiff(baseContent, modifiedContent);
  });

  let isTooLarge = $derived(
      isLargeFile(baseContent) || isLargeFile(modifiedContent)
  );

  // Extract change hunks with ±3 lines of context
  let hunks = $derived.by<DiffHunk[]>(() => {
      if (!diffResult) return [];
      return extractHunks(diffResult, 3);
  });

  // Load Status
  async function reconcileSelectedFile(files: FileStatus[], refreshDiff: boolean): Promise<void> {
      if (!selectedFile) return;

      const matched = files.find((file) => file.path === selectedFile?.path && file.staged === selectedFile?.staged);
      if (matched) {
          if (refreshDiff) {
              await loadDiff(matched);
          }
          return;
      }

      const alternate = files.find((file) => file.path === selectedFile?.path && file.staged !== selectedFile?.staged);
      if (alternate) {
          selectedFile = alternate;
          await loadDiff(alternate);
          return;
      }

      selectedFile = null;
      baseContent = "";
      modifiedContent = "";
  }

  async function loadStatus(refreshDiff = false) {
      if (!repoPath) return;
      loadingStatus = true;
      try {
          const [files, conflicts, nextOperationState] = await Promise.all([
              GitService.getStatusFiles(repoPath),
              GitService.getConflicts(repoPath).catch(() => [] as string[]),
              GitService.getOperationState(repoPath).catch(() => DEFAULT_OPERATION_STATE)
          ]);

          operationState = normalizeOperationState(nextOperationState);

          const conflictCandidates = operationState.conflictPaths.length > 0
              ? operationState.conflictPaths
              : conflicts.map((path) => resolvePathForActions(path));

          const mergedFiles = mergeStatusFilesWithConflictPaths(files, conflictCandidates);

          stagedFiles = mergedFiles.filter(f => f.staged);
          unstagedFiles = mergedFiles.filter(f => !f.staged);
          conflictPaths = new Set(conflictCandidates.map((path) => resolvePathForActions(path)));
          await reconcileSelectedFile(mergedFiles, refreshDiff);
      } catch (e: any) {
          console.error("Failed to load status:", e);
          conflictPaths = new Set();
          operationState = DEFAULT_OPERATION_STATE;
      } finally {
          loadingStatus = false;
      }
  }

  // Load file contents for diff
  async function loadDiff(file: FileStatus) {
      if (!repoPath) return;
      loadingDiff = true;
      baseContent = "";
      modifiedContent = "";
      try {
          // For staged entries: HEAD -> index.
          // For unstaged entries: index -> working tree.
          const [base, modified] = await Promise.all([
              GitService.getFileBaseContent(file.path, file.staged, repoPath, selectedEncoding),
              GitService.getFileModifiedContent(file.path, file.staged, repoPath, selectedEncoding),
          ]);
          baseContent = base;
          modifiedContent = modified;
      } catch (e: any) {
          console.error("Failed to load file contents for diff:", e);
          baseContent = "";
          modifiedContent = "";
      } finally {
          loadingDiff = false;
      }
  }

  function handleSelect(file: FileStatus) {
      selectedFile = file;
      selectedEncoding = undefined; // Reset encoding on new file
      // Refresh file lists so changes made outside the app (e.g. in an
      // editor) are picked up whenever the user switches files.
      loadStatus(true);
  }

  function handleEncodingChange(encoding: string) {
      selectedEncoding = encoding;
      if (selectedFile) {
          loadDiff(selectedFile);
      }
  }

  // Actions
  async function handleStage(file: FileStatus) {
      if (!repoPath) return;
      try {
          await GitService.stageFile(file.path, repoPath);
          await loadStatus();
      } catch (e) { /* toast handled in service */ }
  }

  async function handleStageLine(line: DiffStageLineTarget) {
      if (!repoPath || !selectedFile || selectedFile.staged) return;
      try {
          await GitService.stageLine(selectedFile.path, line, repoPath);
          await loadStatus(true);
      } catch (e) { /* toast handled in service */ }
  }

  async function handleUnstage(file: FileStatus) {
       if (!repoPath) return;
      try {
          await GitService.unstageFile(file.path, repoPath);
          await loadStatus();
      } catch (e) { /* toast handled in service */ }
  }

  async function handleUnstageLine(line: DiffStageLineTarget) {
      if (!repoPath || !selectedFile || !selectedFile.staged) return;
      try {
          await GitService.unstageLine(selectedFile.path, line, repoPath);
          await loadStatus(true);
      } catch (e) { /* toast handled in service */ }
  }

  async function handleCommit(message: string, push: boolean) {
      if (!repoPath || commitActionState !== "idle") return;
      commitActionState = "committing";
      try {
          await GitService.commit(message, repoPath);
          if (push) {
              await GitService.push(repoPath);
          }
          await loadStatus();
          // Message clear is handled by component, but we can reset selection if needed
          selectedFile = null;
          baseContent = "";
          modifiedContent = "";
      } catch (e: any) {
          // Toast handled in service mostly, but here for double check
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
          await loadStatus(true);
          selectedFile = null;
          baseContent = "";
          modifiedContent = "";
      } finally {
          commitActionState = "idle";
      }
  }

  async function handleSkipRebase() {
      if (!repoPath || commitActionState !== "idle") return;
      commitActionState = "aborting"; // Use same busy state
      try {
          await rebaseStore.skip(repoPath);
          await loadStatus(true);
      } finally {
          commitActionState = "idle";
      }
  }

  async function handleStageAll() {
      if (!repoPath) return;
      try {
          await GitService.stageAll(repoPath);
          await loadStatus();
      } catch (e) { /* toast handled */ }
  }

  async function handleUnstageAll() {
      if (!repoPath) return;
      try {
          await GitService.unstageAll(repoPath);
          await loadStatus();
      } catch (e) { /* toast handled */ }
  }

  function getUniqueFilesForDiscard(files: FileStatus[]): FileStatus[] {
      const uniqueByPath = new Map<string, FileStatus>();
      for (const file of files) {
          const existing = uniqueByPath.get(file.path);
          if (!existing) {
              uniqueByPath.set(file.path, file);
              continue;
          }
          // Prefer tracked status over untracked if duplicates ever appear.
          if (existing.status === "??" && file.status !== "??") {
              uniqueByPath.set(file.path, file);
          }
      }
      return [...uniqueByPath.values()];
  }

  async function handleDiscardFile(file: FileStatus) {
      if (!repoPath) return;
      const confirmed = await confirm({
          title: "Discard Change",
          message: `Discard all changes in "${file.path}"?\nThis action cannot be undone.`,
          confirmLabel: "Discard",
          cancelLabel: "Cancel"
      });
      if (!confirmed) return;

      try {
          await GitService.discardChanges([file], repoPath);
          await loadStatus(true);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleDiscardAll() {
      if (!repoPath) return;
      const files = getUniqueFilesForDiscard([...unstagedFiles, ...stagedFiles]);
      if (files.length === 0) return;

      const confirmed = await confirm({
          title: "Discard All Changes",
          message: `Discard all local changes in ${files.length} file(s)?\nThis action cannot be undone.`,
          confirmLabel: "Discard All",
          cancelLabel: "Cancel"
      });
      if (!confirmed) return;

      try {
          await GitService.discardChanges(files, repoPath);
          await loadStatus(true);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleStashFile(file: FileStatus) {
      if (!repoPath) return;
      try {
          await GitService.stashFile(file, repoPath);
          await loadStatus(true);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleStashAll() {
      if (!repoPath) return;
      try {
          await GitService.stashAll(repoPath);
          await loadStatus(true);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleOpenFile(file: FileStatus) {
      if (!repoPath) return;
      try {
          await GitService.openRepoFile(file.path, repoPath);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleIgnoreFile(pattern: string) {
      if (!repoPath) return;
      try {
          await GitService.ignoreFile(pattern, repoPath);
          await loadStatus(true);
      } catch (e) {
          // toast handled in service
      }
  }

  function handleShowFileHistory(file: FileStatus) {
      const targetPath = resolvePathForActions(file.path);
      selectedFile = { ...file, path: targetPath };
      onShowHistory?.(targetPath);
  }

  function handleShowFileBlame(file: FileStatus) {
      const targetPath = resolvePathForActions(file.path);
      selectedFile = { ...file, path: targetPath };
      onShowBlame?.(targetPath);
  }

  async function handleOpenInDiffTool(file: FileStatus) {
      if (!repoPath) return;
      try {
          await GitService.openInDiffTool(file.path, file.staged, repoPath);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleOpenInEditor(file: FileStatus) {
      if (!repoPath) return;
      try {
          await GitService.openInEditor(file.path, repoPath);
      } catch (e) {
          // toast handled in service
      }
  }

  async function handleShowInFolder(file: FileStatus) {
      if (!repoPath) return;
      try {
          await GitService.showInFolder(file.path, repoPath);
      } catch (e) {
          // toast handled in service
      }
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
      } catch (e) {
          // toast handled in service
      }
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
          cancelLabel: "Cancel"
      });
      if (!confirmed) return;

      try {
          await GitService.deleteFile(file.path, repoPath);
          await loadStatus(true);
      } catch (e) {
          // toast handled in service
      }
  }

  function handleResolveConflict(file: FileStatus) {
      resolvingConflictFilePath = resolvePathForActions(file.path);
  }

  function handleCloseConflictResolver() {
      resolvingConflictFilePath = null;
  }

  async function handleConflictResolved(_filePath: string) {
      resolvingConflictFilePath = null;
      await loadStatus(true);
  }

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

      const fallback = Math.max(
          metrics.minTop,
          Math.min(metrics.maxTop, Math.floor(metrics.totalHeight * 0.5))
      );

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
      if (handle) {
          handle.setPointerCapture(event.pointerId);
      }
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



  $effect(() => {
      if (repoPath) {
          loadStatus();
      }
  });

  onMount(() => {
      const saved = localStorage.getItem(FILE_VIEW_MODE_KEY);
      if (saved === "tree" || saved === "path") {
          fileViewMode = saved;
      }
  });

  onMount(() => {
      if (typeof ResizeObserver !== "undefined" && fileListsContainerEl) {
          fileListsResizeObserver = new ResizeObserver(() => {
              syncFileSplitHeight();
          });
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

  $effect(() => {
      localStorage.setItem(FILE_VIEW_MODE_KEY, fileViewMode);
  });

  // Refresh data when the Commit tab becomes active, so file lists
  // and diffs always reflect the current repository state.
  let prevIsActive = false;
  let lastRefreshTime = 0;
  const DEBOUNCE_MS = 300;

  $effect(() => {
      if (isActive && !prevIsActive && repoPath) {
          const now = Date.now();
          if (now - lastRefreshTime > DEBOUNCE_MS) {
              lastRefreshTime = now;
              loadStatus(true);
          }
      }
      prevIsActive = isActive ?? false;
  });

  export function refresh() {
      // Force refresh of diff as well
      loadStatus(true);
  }

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

  let canStageSelectedLine = $derived(
      !!selectedFile &&
      !selectedFile.staged &&
      selectedFile.status !== "??" &&
      !selectedFile.path.includes(" -> ")
  );

  let canUnstageSelectedLine = $derived(
      !!selectedFile &&
      selectedFile.staged &&
      !selectedFile.path.includes(" -> ")
  );
</script>

<div class="flex h-full w-full bg-[#0d1117] overflow-hidden text-[#c9d1d9] max-[1024px]:flex-col">
    <!-- Left Sidebar -->
    <div class="w-1/3 min-w-[300px] max-w-[450px] flex flex-col border-r border-[#30363d] bg-[#161b22] max-[1024px]:w-full max-[1024px]:min-w-0 max-[1024px]:max-w-none max-[1024px]:h-[52%] max-[1024px]:border-r-0 max-[1024px]:border-b">
        <div class="h-9 px-3 border-b border-[#30363d] bg-[#21262d] flex items-center justify-between shrink-0 gap-2">
            <span class="text-[11px] uppercase tracking-wider font-semibold text-[#8b949e]">Files View</span>
            <div class="inline-flex shrink-0 rounded border border-[#30363d] overflow-hidden">
                <button
                    type="button"
                    class="px-2.5 py-1 text-[11px] font-medium transition-colors {fileViewMode === 'tree' ? 'bg-[#30363d] text-white' : 'bg-[#161b22] text-[#8b949e] hover:text-[#c9d1d9]'}"
                    onclick={() => fileViewMode = "tree"}
                    title="View files as directory tree"
                >
                    Tree
                </button>
                <button
                    type="button"
                    class="px-2.5 py-1 text-[11px] font-medium border-l border-[#30363d] transition-colors {fileViewMode === 'path' ? 'bg-[#30363d] text-white' : 'bg-[#161b22] text-[#8b949e] hover:text-[#c9d1d9]'}"
                    onclick={() => fileViewMode = "path"}
                    title="View files by full path list"
                >
                    Path
                </button>
            </div>
        </div>

        <!-- Resizable file list sections (scrollable region) -->
        <div bind:this={fileListsContainerEl} class="flex-1 min-h-0 flex flex-col overflow-hidden">
            <!-- Changes (Unstaged) - shown first to match Git workflow -->
            <div
                class="min-h-0 shrink-0 flex flex-col"
                style={fileSplitTopHeight === null ? "height: 50%;" : `height: ${fileSplitTopHeight}px;`}
            >
                <CommitFileList
                    title="Changes"
                    files={unstagedFiles}
                    selectedFile={selectedFile}
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
                    conflictPaths={conflictPaths}
                    onResolveConflict={handleResolveConflict}
                    onActionAll={handleStageAll}
                    actionAllLabel="Stage All"
                />
            </div>

            <!-- Shared splitter between Changes and Staged Changes -->
            <div
                class="relative h-2 shrink-0 cursor-row-resize select-none z-10 group"
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
                           {isFileSplitDragging ? 'bg-[#58a6ff]' : 'bg-[#30363d] group-hover:bg-[#484f58]'}"
                ></div>
            </div>

            <!-- Staged Changes -->
            <div class="min-h-0 flex-1 flex flex-col border-t border-[#30363d]">
                <CommitFileList
                    title="Staged Changes"
                    files={stagedFiles}
                    selectedFile={selectedFile}
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
                    conflictPaths={conflictPaths}
                    onResolveConflict={handleResolveConflict}
                    onActionAll={handleUnstageAll}
                    actionAllLabel="Unstage All"
                />
            </div>
        </div>

        <!-- Commit Actions: fixed at bottom, non-resizable -->
        <div class="shrink-0 border-t border-[#30363d]">
            <CommitActions
                stagedCount={stagedFiles.length}
                busy={committing || abortingOperation || loadingStatus}
                generating={generatingCommitMessage}
                bind:message={commitMessage}
                onCommit={handleCommit}
                onGenerate={handleGenerateCommitMessage}
                showAbortOperation={showAbortOperationButton}
                primaryActionLabel={primaryOperationLabel}
                abortOperationLabel={abortOperationLabel}
                onAbortOperation={handleAbortOperation}
            />
            {#if operationState.isRebasing && !operationState.hasConflicts && commitActionState === 'idle'}
                <div class="px-1.5 pb-2">
                    <button
                        type="button"
                        class="w-full h-8 px-3 rounded-sm border border-[#30363d] bg-[#21262d] text-[#c9d1d9] hover:bg-[#30363d] transition-colors text-[12px] font-semibold"
                        onclick={handleSkipRebase}
                    >
                        Skip Current Commit
                    </button>
                </div>
            {/if}
        </div>
    </div>

    <!-- Right Content: Diff -->
    <div class="flex-1 flex flex-col bg-[#0d1117] overflow-hidden max-[1024px]:h-[48%]">
        {#if !selectedFile}
             <div class="flex-1 flex items-center justify-center text-[#8b949e] text-sm select-none">
                 Select a file to view diff
             </div>
        {:else}
             <DiffView 
                 {diffResult}
                 {hunks}
                 loading={loadingDiff}
                 {isTooLarge}
                 {selectedEncoding}
                 onEncodingChange={handleEncodingChange}
                 canStageLine={canStageSelectedLine}
                 onStageLine={handleStageLine}
                 canUnstageLine={canUnstageSelectedLine}
                 onUnstageLine={handleUnstageLine}
             >
                {#snippet header(toolbarProps)}
                    <!-- File header bar -->
                    <div class="h-8 px-3 flex items-center gap-2 border-b border-[#30363d] bg-[#161b22] shrink-0 min-w-0">
                        <FileChangeStatusBadge status={selectedFile.status} compact={true} showCode={true} className="shrink-0" />
                        <span class="text-xs font-mono text-[#8b949e] truncate min-w-0" title={selectedFile.path}>{selectedFile.path}</span>
                        <span class="text-[10px] px-1.5 py-0.5 rounded border border-[#30363d] text-[#8b949e]">
                            {selectedFile.staged ? 'STAGED' : 'UNSTAGED'}
                        </span>
                    </div>

                    <!-- Diff Toolbar passed from DiffView state -->
                    <div class="border-b border-[#30363d] bg-[#161b22]">
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
                {/snippet}
             </DiffView>
        {/if}
    </div>
</div>

{#if resolvingConflictFilePath}
    <ConflictResolveModal
        repoPath={repoPath}
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
