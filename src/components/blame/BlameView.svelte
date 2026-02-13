<script lang="ts">
  import { GitService } from "../../lib/GitService";
  import type { BlameLine } from "../../lib/types";
  import { toast } from "../../lib/toast.svelte";

  interface Props {
    repoPath: string;
    filePath: string | null;
    onCommitSelect?: (commitHash: string) => void | Promise<void>;
  }

  let { repoPath, filePath, onCommitSelect }: Props = $props();

  let lines = $state<BlameLine[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  function shortHash(hash: string): string {
    return hash ? hash.slice(0, 8) : "";
  }

  function formatDate(raw: string): string {
    const unixSeconds = Number.parseInt(raw, 10);
    if (!Number.isNaN(unixSeconds) && unixSeconds > 0) {
      return new Date(unixSeconds * 1000).toLocaleString();
    }
    if (!raw) return "";
    const parsed = new Date(raw);
    if (Number.isNaN(parsed.getTime())) return raw;
    return parsed.toLocaleString();
  }

  async function loadBlame(): Promise<void> {
    if (!repoPath || !filePath) return;

    loading = true;
    error = null;
    lines = [];

    try {
      lines = await GitService.getBlame(filePath, repoPath);
    } catch (e: any) {
      console.error("Failed to load file blame:", e);
      error = typeof e === "string" ? e : e.message || String(e);
    } finally {
      loading = false;
    }
  }

  async function handleCommitClick(hash: string): Promise<void> {
    if (!hash) return;
    if (onCommitSelect) {
      await onCommitSelect(hash);
      return;
    }

    try {
      await navigator.clipboard.writeText(hash);
      toast.success(`Copied commit hash: ${hash}`);
    } catch (e) {
      toast.error("Copy commit hash failed");
    }
  }

  $effect(() => {
    if (!repoPath || !filePath) {
      lines = [];
      loading = false;
      error = null;
      return;
    }
    loadBlame();
  });
</script>

<div class="flex h-full flex-col bg-[#0d1117] text-[#c9d1d9] overflow-hidden">
  <div class="px-4 py-2 border-b border-[#30363d] bg-[#161b22] shrink-0 flex items-center justify-between gap-2 min-w-0">
    <div class="font-semibold text-sm truncate min-w-0">
      {#if filePath}
        Blame: <span class="text-[#58a6ff] truncate min-w-0" title={filePath}>{filePath}</span>
      {:else}
        File Blame
      {/if}
    </div>
    {#if filePath}
      <button
        onclick={() => void loadBlame()}
        class="p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-[#c9d1d9] transition-colors"
        title="Refresh Blame"
        aria-label="Refresh Blame"
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

  {#if !filePath}
    <div class="flex-1 flex flex-col items-center justify-center text-[#8b949e] p-8 text-center">
      <p>Select a file to view blame information</p>
    </div>
  {:else if error}
    <div class="flex-1 p-4">
      <div class="text-[#f85149] bg-[#da3633]/10 p-4 rounded border border-[#da3633]/20">
        <h3 class="font-semibold mb-1">Error loading blame</h3>
        <p class="text-sm opacity-90">{error}</p>
      </div>
    </div>
  {:else if loading}
    <div class="flex-1 flex items-center justify-center">
      <svg class="animate-spin h-6 w-6 text-[#58a6ff]" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
      </svg>
    </div>
  {:else if lines.length === 0}
    <div class="flex-1 flex items-center justify-center text-[#8b949e] p-6 text-center">
      No blame data available for this file.
    </div>
  {:else}
    <div class="flex-1 overflow-auto custom-scrollbar">
      <div class="min-w-[860px]">
        <div class="sticky top-0 z-10 grid grid-cols-[72px_120px_230px_1fr] bg-[#161b22] border-b border-[#30363d] text-[11px] uppercase tracking-wider text-[#8b949e]">
          <div class="px-3 py-2 text-right">Line</div>
          <div class="px-3 py-2">Commit</div>
          <div class="px-3 py-2">Author / Date</div>
          <div class="px-3 py-2 border-l border-[#30363d]">Content</div>
        </div>

        {#each lines as line, i (`${line.lineNumber}:${line.commitHash}:${i}`)}
          <div class="blame-row grid grid-cols-[72px_120px_230px_1fr] border-b border-[#21262d] hover:bg-[#161b22]/60">
            <div class="px-3 py-1.5 text-right text-xs text-[#8b949e] font-mono">{line.lineNumber}</div>
            <div class="px-3 py-1.5 text-xs">
              <button
                class="font-mono text-[#58a6ff] hover:text-[#79c0ff] hover:underline"
                onclick={() => void handleCommitClick(line.commitHash)}
                title={line.commitHash}
              >
                {shortHash(line.commitHash)}
              </button>
            </div>
            <div class="px-3 py-1.5 text-xs text-[#8b949e] truncate" title={`${line.author} • ${formatDate(line.date)}`}>
              {line.author} • {formatDate(line.date)}
            </div>
            <div class="px-2 py-1.5 border-l border-[#21262d]">
              <pre class="blame-content-block">{line.content || " "}</pre>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
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

  .blame-content-block {
    margin: 0;
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid #25324e;
    background: #0a1221;
    color: #d6e4ff;
    font-size: 12px;
    line-height: 1.4;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
    white-space: pre;
    overflow-x: auto;
  }

  .blame-row:hover .blame-content-block {
    border-color: #344873;
    background: #0d1730;
  }
</style>
