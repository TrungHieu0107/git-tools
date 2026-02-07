<script lang="ts">
  import { GitService, type FileStatus } from "../lib/GitService";
  import { toast } from "../lib/toast.svelte";
  import { computeDiff, isLargeFile, extractHunks, type DiffResult, type DiffHunk } from "../lib/diff";

  import CommitFileList from "./commit/CommitFileList.svelte";
  import SideBySideDiffViewer from "./commit/SideBySideDiffViewer.svelte";
  import InlineDiffViewer from "./commit/InlineDiffViewer.svelte";
  import DiffToolbar from "./commit/DiffToolbar.svelte";
  import type { ViewMode } from "./commit/DiffToolbar.svelte";
  import CommitActions from "./commit/CommitActions.svelte";

  interface Props {
      repoPath?: string;
  }
  let { repoPath }: Props = $props();

  let stagedFiles = $state<FileStatus[]>([]);
  let unstagedFiles = $state<FileStatus[]>([]);
  let selectedFile = $state<FileStatus | null>(null);
  let baseContent = $state<string>("");
  let modifiedContent = $state<string>("");
  let loadingStatus = $state(false);
  let loadingDiff = $state(false);
  let committing = $state(false);

  // View mode and hunk navigation state
  let viewMode = $state<ViewMode>("side-by-side");
  let currentHunkIndex = $state(0);

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
  async function loadStatus() {
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
              GitService.getFileBaseContent(file.path, repoPath),
              GitService.getFileModifiedContent(file.path, file.staged, repoPath),
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
      currentHunkIndex = 0;
      loadDiff(file);
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

  // ── View mode & hunk navigation ────────────────────────────────
  function handleViewModeChange(mode: ViewMode) {
      viewMode = mode;
      currentHunkIndex = 0;
  }

  function handlePrevHunk() {
      if (currentHunkIndex > 0) {
          currentHunkIndex--;
          scrollToHunk(currentHunkIndex);
      }
  }

  function handleNextHunk() {
      if (currentHunkIndex < hunks.length - 1) {
          currentHunkIndex++;
          scrollToHunk(currentHunkIndex);
      }
  }

  function scrollToHunk(index: number) {
      const el = document.querySelector(`[data-hunk-id="hunk-${index}"]`);
      el?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  $effect(() => {
      if (repoPath) {
          loadStatus();
      }
  });

  export function refresh() {
      loadStatus();
  }
</script>

<div class="flex h-full w-full bg-[#0d1117] overflow-hidden text-[#c9d1d9]">
    <!-- Left Sidebar -->
    <div class="w-1/3 min-w-[300px] max-w-[450px] flex flex-col border-r border-[#30363d] bg-[#161b22]">

        <!-- Staged List -->
        <div class="flex-1 flex flex-col min-h-0">
             <CommitFileList
                 title="Staged Changes"
                 files={stagedFiles}
                 selectedFile={selectedFile}
                 onSelect={handleSelect}
                 onAction={handleUnstage}
                 actionLabel="Unstage"
             />
        </div>

        <!-- Unstaged List -->
        <div class="flex-1 flex flex-col min-h-0 border-t border-[#30363d]">
             <CommitFileList
                 title="Changes"
                 files={unstagedFiles}
                 selectedFile={selectedFile}
                 onSelect={handleSelect}
                 onAction={handleStage}
                 actionLabel="Stage"
             />
        </div>

        <!-- Commit Actions -->
        <div class="shrink-0">
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
             <!-- File header bar -->
             <div class="h-8 px-3 flex items-center gap-2 border-b border-[#30363d] bg-[#161b22] shrink-0">
                 <span class="text-xs font-mono text-[#8b949e]">{selectedFile.path}</span>
                 <span class="text-[10px] px-1.5 py-0.5 rounded border border-[#30363d] text-[#8b949e]">
                     {selectedFile.staged ? 'STAGED' : 'UNSTAGED'}
                 </span>
             </div>

             <!-- Diff toolbar: view mode toggle + hunk navigation -->
             <DiffToolbar
                 {viewMode}
                 onViewModeChange={handleViewModeChange}
                 {currentHunkIndex}
                 totalHunks={hunks.length}
                 onPrevHunk={handlePrevHunk}
                 onNextHunk={handleNextHunk}
             />

             <!-- Diff viewer: conditional on selected mode -->
             {#if viewMode === "inline"}
                 {#if diffResult}
                     <InlineDiffViewer {diffResult} {hunks} loading={loadingDiff} />
                 {:else if isTooLarge}
                     <div class="flex-1 flex items-center justify-center text-[#8b949e] text-sm italic">
                         File too large for diff view
                     </div>
                 {:else if !loadingDiff}
                     <div class="flex-1 flex items-center justify-center text-[#8b949e] text-sm italic">
                         No diff content
                     </div>
                 {/if}
             {:else}
                 <SideBySideDiffViewer
                     {diffResult}
                     loading={loadingDiff}
                     {isTooLarge}
                     hunks={viewMode === "hunk" ? hunks : null}
                 />
             {/if}
        {/if}
    </div>
</div>
