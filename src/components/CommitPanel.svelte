<script lang="ts">
  import { onMount } from "svelte";
  import { GitService, type FileStatus } from "../lib/GitService";
  import { toast } from "../lib/toast.svelte";
  import { computeDiff, isLargeFile, extractHunks, type DiffResult, type DiffHunk } from "../lib/diff";
  import { confirm } from "../lib/confirmation.svelte";

  import CommitFileList from "./commit/CommitFileList.svelte";

  import DiffToolbar from "./diff/DiffToolbar.svelte";
  import type { ViewMode } from "./diff/DiffToolbar.svelte";
  import CommitActions from "./commit/CommitActions.svelte";
  import ResizableSection from "./resize/ResizableSection.svelte";
  import DiffView from "./diff/DiffView.svelte";
  import FileChangeStatusBadge from "./common/FileChangeStatusBadge.svelte";

  interface Props {
      repoPath?: string;
      isActive?: boolean;
      selectedFile?: FileStatus | null;
  }
  let { repoPath, isActive = false, selectedFile = $bindable(null) }: Props = $props();

  let stagedFiles = $state<FileStatus[]>([]);
  let unstagedFiles = $state<FileStatus[]>([]);
  // selectedFile is now a prop
  let baseContent = $state<string>("");
  let modifiedContent = $state<string>("");
  let loadingStatus = $state(false);
  let loadingDiff = $state(false);
  let committing = $state(false);
  let selectedEncoding = $state<string | undefined>(undefined);
  let fileViewMode = $state<"tree" | "path">("path");

  const FILE_VIEW_MODE_KEY = "commit_panel_file_view_mode";



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
  async function loadStatus(refreshDiff = false) {
      if (!repoPath) return;
      loadingStatus = true;
      try {
          const files = await GitService.getStatusFiles(repoPath);
          stagedFiles = files.filter(f => f.staged);
          unstagedFiles = files.filter(f => !f.staged);

          // Validate selection
          if (selectedFile) {
              const stillExists = files.find(f => f.path === selectedFile?.path && f.staged === selectedFile?.staged);
              if (!stillExists) {
                  // Try to find same file in other list?
                  const otherState = files.find(f => f.path === selectedFile?.path && f.staged !== selectedFile?.staged);
                  if (otherState) {
                      selectedFile = otherState;
                      loadDiff(otherState); // Reload diff as it might change (staged vs unstaged diff)
                  } else {
                      selectedFile = null;
                      baseContent = "";
                      modifiedContent = "";
                  }
              } else if (refreshDiff) {
                  // If file still exists and we want a refresh, reload its diff
                  // This ensures we see changes made externally even if selection didn't change
                  loadDiff(selectedFile);
              }
          }
      } catch (e: any) {
          console.error("Failed to load status:", e);
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
          // Fetch base (HEAD) and modified content in parallel
          const [base, modified] = await Promise.all([
              GitService.getFileBaseContent(file.path, repoPath, selectedEncoding),
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

  async function handleUnstage(file: FileStatus) {
       if (!repoPath) return;
      try {
          await GitService.unstageFile(file.path, repoPath);
          await loadStatus();
      } catch (e) { /* toast handled in service */ }
  }

  async function handleCommit(message: string, push: boolean) {
      if (!repoPath) return;
      committing = true;
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
      } finally {
          committing = false;
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
</script>

<div class="flex h-full w-full bg-[#0d1117] overflow-hidden text-[#c9d1d9]">
    <!-- Left Sidebar -->
    <div class="w-1/3 min-w-[300px] max-w-[450px] flex flex-col border-r border-[#30363d] bg-[#161b22]">
        <div class="h-9 px-3 border-b border-[#30363d] bg-[#21262d] flex items-center justify-between shrink-0">
            <span class="text-[11px] uppercase tracking-wider font-semibold text-[#8b949e]">Files View</span>
            <div class="inline-flex rounded border border-[#30363d] overflow-hidden">
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
        <div class="flex-1 min-h-0 flex flex-col overflow-auto">
            <!-- Changes (Unstaged) - shown first to match Git workflow -->
            <ResizableSection initialSize={180} minSize={80} maxSize={400}>
                <div class="h-full flex flex-col">
                    <CommitFileList
                        title="Changes"
                        files={unstagedFiles}
                        selectedFile={selectedFile}
                        onSelect={handleSelect}
                        onAction={handleStage}
                        actionLabel="Stage"
                        onDiscard={handleDiscardFile}
                        onDiscardAll={handleDiscardAll}
                        discardAllLabel="Discard All"
                        showDiscardAll={unstagedFiles.length + stagedFiles.length > 0}
                        viewMode={fileViewMode}
                        onActionAll={handleStageAll}
                        actionAllLabel="Stage All"
                    />
                </div>
            </ResizableSection>

            <!-- Staged Changes -->
            <ResizableSection initialSize={180} minSize={80} maxSize={400}>
                <div class="h-full flex flex-col border-t border-[#30363d]">
                    <CommitFileList
                        title="Staged Changes"
                        files={stagedFiles}
                        selectedFile={selectedFile}
                        onSelect={handleSelect}
                        onAction={handleUnstage}
                        actionLabel="Unstage"
                        onDiscard={handleDiscardFile}
                        viewMode={fileViewMode}
                        onActionAll={handleUnstageAll}
                        actionAllLabel="Unstage All"
                    />
                </div>
            </ResizableSection>
        </div>

        <!-- Commit Actions: fixed at bottom, non-resizable -->
        <div class="shrink-0 border-t border-[#30363d]">
            <CommitActions
                stagedCount={stagedFiles.length}
                loading={committing || loadingStatus}
                onCommit={handleCommit}
            />
        </div>
    </div>

    <!-- Right Content: Diff -->
    <div class="flex-1 flex flex-col bg-[#0d1117] overflow-hidden">
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
             >
                {#snippet header(toolbarProps)}
                    <!-- File header bar -->
                    <div class="h-8 px-3 flex items-center gap-2 border-b border-[#30363d] bg-[#161b22] shrink-0">
                        <FileChangeStatusBadge status={selectedFile.status} compact={true} showCode={true} className="shrink-0" />
                        <span class="text-xs font-mono text-[#8b949e]">{selectedFile.path}</span>
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
