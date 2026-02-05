<script lang="ts">
  import { runGit, type GitResponse, type GitError } from "./lib/git";

  let repoPath = $state("C:/path/to/your/repo");
  let subcommand = $state("status");
  let response = $state<GitResponse | null>(null);
  let error = $state<GitError | null>(null);
  let loading = $state(false);

  async function execute() {
    loading = true;
    error = null;
    response = null;

    try {
      // Split subcommand string by spaces for git engine
      const cmdArgs = subcommand.trim().split(/\s+/);
      response = await runGit(repoPath, cmdArgs);
    } catch (e) {
      error = e as GitError;
    } finally {
      loading = false;
    }
  }
</script>

<main class="min-h-screen bg-neutral-900 text-neutral-100 p-8 font-sans">
  <div class="max-w-4xl mx-auto space-y-8">
    <header class="space-y-2">
      <h1 class="text-4xl font-bold tracking-tight text-indigo-400">GitKraken Mini</h1>
      <p class="text-neutral-400">High-performance Rust-powered Git GUI</p>
    </header>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 bg-neutral-800 p-6 rounded-2xl shadow-xl border border-neutral-700">
      <div class="space-y-2">
        <label for="repo" class="text-sm font-medium text-neutral-300">Repository Path</label>
        <input
          id="repo"
          type="text"
          bind:value={repoPath}
          placeholder="e.g. C:/Projects/my-app"
          class="w-full bg-neutral-700 border border-neutral-600 rounded-lg px-4 py-2 focus:ring-2 focus:ring-indigo-500 outline-none transition-all"
        />
      </div>

      <div class="space-y-2">
        <label for="command" class="text-sm font-medium text-neutral-300">Command</label>
        <div class="flex gap-2">
          <span class="inline-flex items-center px-3 rounded-l-lg border border-r-0 border-neutral-600 bg-neutral-700 text-neutral-400 text-sm">
            git
          </span>
          <input
            id="command"
            type="text"
            bind:value={subcommand}
            placeholder="status"
            class="flex-1 min-w-0 bg-neutral-700 border border-neutral-600 rounded-r-lg px-4 py-2 focus:ring-2 focus:ring-indigo-500 outline-none transition-all"
          />
        </div>
      </div>

      <div class="md:col-span-2">
        <button
          onclick={execute}
          disabled={loading}
          class="w-full bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white font-semibold py-3 rounded-xl transition-all shadow-lg shadow-indigo-500/20 active:scale-[0.98]"
        >
          {loading ? "Executing..." : "Run Command"}
        </button>
      </div>
    </div>

    {#if error}
      <div class="bg-red-500/10 border border-red-500/50 p-6 rounded-2xl animate-in fade-in slide-in-from-top-4">
        <h3 class="text-red-400 font-bold flex items-center gap-2">
          <span>⚠️</span> Error: {error.type}
        </h3>
        <p class="mt-2 text-red-200/80 font-mono text-sm whitespace-pre-wrap">{error.message}</p>
      </div>
    {/if}

    {#if response}
      <div class="space-y-4 animate-in fade-in slide-in-from-bottom-4">
        <div class="flex items-center justify-between">
          <h2 class="text-xl font-semibold text-emerald-400">Output</h2>
          <span class="text-xs font-mono px-2 py-1 bg-neutral-800 rounded border border-neutral-700">
            Exit Code: {response.exit_code}
          </span>
        </div>
        
        <div class="bg-neutral-950 p-6 rounded-2xl border border-neutral-800 font-mono text-sm overflow-x-auto shadow-inner">
          {#if response.stdout}
            <pre class="text-emerald-300/90">{response.stdout}</pre>
          {/if}
          {#if response.stderr}
            <pre class="text-rose-400/90 mt-2">{response.stderr}</pre>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    scrollbar-width: thin;
    scrollbar-color: #4f46e5 #171717;
  }
</style>
