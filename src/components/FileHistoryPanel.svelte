<script lang="ts">
  import { GitService } from "../lib/GitService";
  import type { FileCommit, CommitDiff, DiffHunk as BackendDiffHunk } from "../lib/types";
  import { computeDiff, isLargeFile, extractHunks, type DiffResult, type DiffHunk } from "../lib/diff";
  import DiffView from "./diff/DiffView.svelte";
  import DiffToolbar from "./diff/DiffToolbar.svelte";

  interface Props {
    repoPath: string;
    filePath: string | null;
    onFileSelect?: (path: string) => void;
  }

  let { repoPath, filePath, onFileSelect }: Props = $props();

  let commits = $state<FileCommit[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Search state - load on demand
  let searchQuery = $state("");
  let searchResults = $state<string[]>([]);
  let filesLoading = $state(false);
  let filesError = $state<string | null>(null);
  let showDropdown = $state(false);
  let selectedIndex = $state(-1);
  let searchDebounceTimer: number | null = null; // Removed $state as it's not needed for UI

  // Diff View State
  let selectedCommitHash = $state<string | null>(null);
  let diffLoading = $state(false);
  let baseContent = $state("");
  let modifiedContent = $state("");
  let commitHunks = $state<BackendDiffHunk[]>([]);
  let selectedEncoding = $state<string | undefined>(undefined);

  // Derived: full-file diff for side-by-side and hunk modes (same pattern as CommitPanel)
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

  let totalHunks = $derived(hunks.length);

  // Effect to load history when filePath or repoPath changes
  $effect(() => {
    if (!repoPath || !filePath) {
      commits = [];
      error = null;
      selectedCommitHash = null;
      baseContent = "";
      modifiedContent = "";
      commitHunks = [];
      return;
    }

    loadHistory();
  });

  function handleSearchInput() {
    selectedIndex = -1;
    showDropdown = true;
    
    if (searchDebounceTimer) {
        clearTimeout(searchDebounceTimer);
    }

    if (!searchQuery.trim()) {
        searchResults = [];
        return;
    }

    filesLoading = true;
    searchDebounceTimer = window.setTimeout(() => {
        performSearch(searchQuery);
    }, 300);
  }

  async function performSearch(query: string) {
    if (!repoPath) return;
    
    filesError = null;
    try {
        // Call backend with query
        searchResults = await GitService.searchRepoFiles(query, repoPath);
    } catch (e: any) {
        console.error("Search failed:", e);
        filesError = typeof e === "string" ? e : e.message || String(e);
        searchResults = [];
    } finally {
        filesLoading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!showDropdown || searchResults.length === 0) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, searchResults.length - 1);
        break;
      case "ArrowUp":
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, -1);
        break;
      case "Enter":
        e.preventDefault();
        if (selectedIndex >= 0 && selectedIndex < searchResults.length) {
          selectFile(searchResults[selectedIndex]);
        } else if (searchResults.length > 0) {
          selectFile(searchResults[0]);
        }
        break;
      case "Escape":
        e.preventDefault();
        showDropdown = false;
        selectedIndex = -1;
        break;
    }
  }

  function selectFile(path: string) {
    searchQuery = ""; // Optionally keep the query or clear it? Keeping it clear for now.
    showDropdown = false;
    selectedIndex = -1;
    selectedCommitHash = null;
    baseContent = "";
    modifiedContent = "";
    commitHunks = [];
    if (onFileSelect) {
      onFileSelect(path);
    }
  }

  function handleEncodingChange(encoding: string) {
      selectedEncoding = encoding;
      // Re-load current commit if selected
      const currentCommit = commits.find(c => c.hash === selectedCommitHash);
      if (currentCommit) {
          // Force reload by clearing previous state?
          // Actually selectCommit checks for hash equality and returns early.
          // We need to bypass that check or reset hash first.
          // Or split load logic.
          // For now, let's just reset hash momentarily or make selectCommit smarter?
          selectedCommitHash = null; // Reset to force reload
          selectCommit(currentCommit);
      }
  }

  function closeDropdown() {
    setTimeout(() => {
      showDropdown = false;
      selectedIndex = -1;
    }, 150);
  }

  async function loadHistory() {
    if (!repoPath || !filePath) return;

    loading = true;
    error = null;
    commits = [];

    try {
      commits = await GitService.getFileHistory(filePath, 100, repoPath);
    } catch (e: any) {
      console.error("Failed to load file history:", e);
      error = typeof e === "string" ? e : e.message || String(e);
    } finally {
      loading = false;
    }
  }

  async function selectCommit(commit: FileCommit) {
      if (selectedCommitHash === commit.hash || !filePath) return;
      selectedCommitHash = commit.hash;
      diffLoading = true;
      baseContent = "";
      modifiedContent = "";
      commitHunks = [];


      try {
          // Step 1: Get file-scoped diff (hunks + parent hash)
          const diff = await GitService.getCommitDiff(commit.hash, repoPath, filePath, selectedEncoding);
          // Extract hunks for the single file (should be 0 or 1 file entries)
          if (diff.files.length > 0) {
              commitHunks = diff.files[0].hunks;
          }

          // Step 2: Fetch full file contents in parallel for side-by-side view
          const promises: Promise<string>[] = [];
          // Modified content (file at selected commit)
          promises.push(
              GitService.getFileAtCommit(commit.hash, filePath, repoPath, selectedEncoding)
                  .catch(() => "") // File might not exist (deleted)
          );
          // Base content (file at parent commit)
          if (diff.parentHash) {
              promises.push(
                  GitService.getFileAtCommit(diff.parentHash, filePath, repoPath, selectedEncoding)
                      .catch(() => "") // File might not exist at parent (newly added)
              );
          } else {
              promises.push(Promise.resolve("")); // Root commit — no parent
          }

          const [mod, base] = await Promise.all(promises);
          modifiedContent = mod;
          baseContent = base;
      } catch (e) {
          console.error("Failed to load commit diff:", e);
      } finally {
          diffLoading = false;
      }
  }



  function formatDate(dateStr: string): string {
    if (!dateStr) return "";
    return new Date(dateStr).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  function highlightMatch(text: string, query: string): string {
    if (!query) return text;
    const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return text.replace(regex, '<mark class="bg-[#58a6ff]/30 text-[#c9d1d9]">$1</mark>');
  }
</script>

<div class="flex flex-col h-full bg-[#0d1117] text-[#c9d1d9] overflow-hidden">
  <!-- Header -->
  <div class="px-4 py-2 border-b border-[#30363d] bg-[#161b22] shrink-0 flex items-center justify-between gap-2 min-w-0">
    <div class="font-semibold text-sm truncate flex items-center gap-2 min-w-0">
      {#if filePath}
        History: <span class="text-[#58a6ff] truncate min-w-0" title={filePath}>{filePath}</span>
      {:else}
        File History
      {/if}
    </div>
    {#if filePath}
      <button
        onclick={loadHistory}
        class="p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-[#c9d1d9] transition-colors"
        title="Refresh History"
        aria-label="Refresh History"
      >
        <svg
          class="w-4 h-4 {loading ? 'animate-spin' : ''}"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
          />
        </svg>
      </button>
    {/if}
  </div>

  <!-- Search Bar -->
  <div class="px-4 py-2 border-b border-[#30363d] bg-[#161b22] relative shrink-0">
    <div class="relative">
      <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[#6e7681]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
      <input
        type="text"
        bind:value={searchQuery}
        oninput={handleSearchInput}
        onkeydown={handleKeydown}
        onfocus={() => { if (searchQuery) showDropdown = true; }}
        onblur={closeDropdown}
        placeholder="Search files..."
        class="w-full pl-9 pr-8 py-1.5 text-sm bg-[#0d1117] border border-[#30363d] rounded focus:border-[#58a6ff] focus:outline-none placeholder-[#6e7681]"
      />
      {#if filesLoading}
        <div class="absolute right-3 top-1/2 -translate-y-1/2">
          <svg class="animate-spin h-4 w-4 text-[#8b949e]" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
          </svg>
        </div>
      {:else if searchQuery}
        <button
          onclick={() => { searchQuery = ""; showDropdown = false; selectedIndex = -1; }}
          class="absolute right-3 top-1/2 -translate-y-1/2 text-[#8b949e] hover:text-[#c9d1d9]"
          aria-label="Clear search"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      {/if}
    </div>

    {#if showDropdown && searchQuery}
      <div class="absolute left-4 right-4 top-full mt-1 bg-[#161b22] border border-[#30363d] rounded shadow-lg z-50 max-h-72 overflow-auto custom-scrollbar">
        {#if searchResults.length > 0}
          {#each searchResults as file, i}
            <button
              onmousedown={() => selectFile(file)}
              onmouseenter={() => selectedIndex = i}
              class="w-full px-3 py-2 text-left text-sm hover:bg-[#30363d] flex items-center gap-2 transition-colors {selectedIndex === i ? 'bg-[#30363d]' : ''}"
            >
              <svg class="w-4 h-4 text-[#8b949e] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <span class="truncate">{@html highlightMatch(file, searchQuery)}</span>
            </button>
          {/each}
        {:else if !filesLoading}
          <div class="px-3 py-3 text-sm text-[#8b949e]">
            No files found matching "{searchQuery}"
          </div>
        {/if}
      </div>
    {/if}

    {#if filesError}
      <div class="text-xs text-[#f85149] mt-1">Failed to load files: {filesError}</div>
    {/if}
  </div>

  <!-- Main Content: Two-column layout -->
  <div class="flex-1 flex min-h-0 overflow-hidden max-[1024px]:flex-col">
    {#if !filePath}
      <div class="flex-1 flex flex-col items-center justify-center text-[#8b949e] p-8 text-center">
        <svg class="w-12 h-12 mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        <p>Search for a file above to view its history</p>
        <p class="text-xs mt-2 opacity-75">Type a filename to search</p>
      </div>
    {:else if error}
      <div class="flex-1 p-4">
        <div class="text-[#f85149] bg-[#da3633]/10 p-4 rounded border border-[#da3633]/20">
          <h3 class="font-semibold mb-1">Error loading history</h3>
          <p class="text-sm opacity-90">{error}</p>
        </div>
      </div>
    {:else}
      <!-- Left Column: Commit List -->
      <div class="w-80 min-w-[280px] max-w-[360px] shrink-0 border-r border-[#30363d] flex flex-col overflow-hidden bg-[#0d1117] max-[1024px]:w-full max-[1024px]:min-w-0 max-[1024px]:max-w-none max-[1024px]:h-[42%] max-[1024px]:border-r-0 max-[1024px]:border-b">
        <div class="flex-1 overflow-auto custom-scrollbar relative">
          {#if loading}
            <div class="absolute inset-0 flex items-center justify-center bg-[#0d1117]/50 z-10">
              <svg
                class="animate-spin h-6 w-6 text-[#58a6ff]"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
            </div>
          {/if}

          {#if commits.length === 0 && !loading}
            <div class="h-full flex flex-col items-center justify-center text-[#8b949e] p-6 text-center">
              <p class="text-sm">No history found for this file.</p>
              <p class="text-xs mt-2 opacity-75">It might be a new file or untracked.</p>
            </div>
          {:else}
            <div class="p-3 space-y-2">
              {#each commits as commit}
                <button
                  class="w-full text-left rounded px-3 py-2.5 transition-colors border
                         {selectedCommitHash === commit.hash
                           ? 'bg-[#30363d] border-[#58a6ff]'
                           : 'bg-[#161b22] border-[#30363d] hover:border-[#8b949e]'}"
                  onclick={() => selectCommit(commit)}
                >
                  <div class="flex items-start justify-between gap-2 mb-1">
                    <h4 class="font-semibold text-[#c9d1d9] text-xs leading-snug line-clamp-2" title={commit.message}>
                      {commit.message}
                    </h4>
                    <span class="font-mono text-[10px] text-[#58a6ff] bg-[#58a6ff]/10 px-1 py-0.5 rounded shrink-0 select-all">
                      {commit.hash.substring(0, 7)}
                    </span>
                  </div>
                  <div class="flex items-center gap-2 text-[10px] text-[#8b949e]">
                    <span class="truncate max-w-[120px]" title={commit.author}>{commit.author}</span>
                    <span>·</span>
                    <span>{formatDate(commit.date)}</span>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <!-- Right Column: Diff View -->
      <div class="flex-1 overflow-hidden flex flex-col bg-[#0d1117] max-[1024px]:h-[58%]">
        {#if selectedCommitHash}
          <DiffView 
              {diffResult}
              {hunks}
              loading={diffLoading}
              {isTooLarge}
              {selectedEncoding}
              onEncodingChange={handleEncodingChange}
          >
            {#snippet header(toolbarProps)}
                <!-- Toolbar -->
                <div class="bg-[#161b22] border-b border-[#30363d] px-3 sm:px-4 py-2 flex flex-wrap items-center justify-between gap-2 shrink-0">
                    <div class="text-xs font-mono text-[#8b949e]">
                    Commit {selectedCommitHash?.substring(0, 7)}
                    </div>
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
        {:else}
          <div class="h-full flex flex-col items-center justify-center text-[#8b949e] text-sm">
            <svg class="w-10 h-10 mb-3 opacity-40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122" />
            </svg>
            <p>Select a commit to view changes</p>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

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
