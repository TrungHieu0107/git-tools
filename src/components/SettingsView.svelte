<script lang="ts">
  import { onMount } from 'svelte';
  import { GitService, type AppSettings } from '../lib/GitService';

  interface Props {
      repoPath?: string;
  }
  let { repoPath }: Props = $props();

  let settings = $state<AppSettings | null>(null);
  let newExclusion = $state("");

  onMount(async () => {
    try {
        settings = await GitService.getSettings();
        // Ensure array exists
        if (settings && !settings.excluded_files) {
            settings.excluded_files = [];
        }
    } catch (e) {
        console.error("Failed to load settings", e);
    }
  });

  async function addExclusion() {
    if (!newExclusion.trim() || !settings) return;
    const current = settings.excluded_files || [];
    if (current.includes(newExclusion.trim())) {
        newExclusion = "";
        return;
    }

    const exclusions = [...current, newExclusion.trim()];
    try {
        settings = await GitService.setExcludedFiles(exclusions);
        newExclusion = "";
    } catch (e) {
        console.error("Failed to add exclusion", e);
    }
  }

  async function removeExclusion(index: number) {
    if (!settings || !settings.excluded_files) return;
    const exclusions = [...settings.excluded_files];
    exclusions.splice(index, 1);
    try {
        settings = await GitService.setExcludedFiles(exclusions);
    } catch (e) {
        console.error("Failed to remove exclusion", e);
    }
  }
</script>

<div class="h-full flex flex-col p-6 bg-[#0d1117] text-[#c9d1d9] overflow-auto">
  <h2 class="text-xl font-bold mb-6 text-white pb-2 border-b border-[#30363d]">Settings</h2>
  
  {#if repoPath}
      <div class="mb-8">
          <h3 class="text-sm font-semibold uppercase text-[#8b949e] mb-2 tracking-wider">Repository</h3>
          <div class="bg-[#161b22] border border-[#30363d] rounded p-3 font-mono text-xs text-[#c9d1d9] select-all">
              {repoPath}
          </div>
          <p class="text-xs text-[#8b949e] mt-2">
              Repository-specific settings will be available here soon.
          </p>
      </div>
  {/if}

  <div class="max-w-2xl">
    <h3 class="text-sm font-semibold uppercase text-[#8b949e] mb-2 tracking-wider">Global File Exclusions</h3>
    <p class="text-xs text-[#8b949e] mb-4 leading-relaxed">
      Files matching these glob patterns will be virtually ignored by GitHelper. They strictly won't appear in 
      <span class="text-[#c9d1d9]">Changed</span> or <span class="text-[#c9d1d9]">Staged</span> lists, 
      and will never be committed by this app.
      <br/><br/>
      Examples: <code>*.log</code>, <code>dist/**</code>, <code>temp/cache.json</code>
    </p>

    <!-- List -->
    <div class="bg-[#161b22] border border-[#30363d] rounded-md overflow-hidden mb-4">
        {#if !settings?.excluded_files || settings.excluded_files.length === 0}
            <div class="p-4 text-center text-xs text-[#8b949e] italic">
                No excluded files.
            </div>
        {:else}
            {#each settings.excluded_files as exc, i}
                <div class="flex items-center gap-3 px-3 py-2 border-b border-[#30363d] last:border-0 hover:bg-[#0d1117] transition-colors group">
                    <span class="text-[#8b949e] select-none text-xs font-mono">{@html i + 1}.</span>
                    <span class="flex-1 font-mono text-xs text-[#c9d1d9]">{exc}</span>
                    <button 
                        onclick={() => removeExclusion(i)} 
                        class="text-[#8b949e] hover:text-[#f85149] opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded"
                        title="Remove exclusion"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
            {/each}
        {/if}
    </div>

    <!-- Add -->
    <div class="flex gap-2 items-center">
      <div class="flex-1 relative">
         <input 
            type="text" 
            bind:value={newExclusion} 
            placeholder="Enter glob pattern (e.g. *.log)..." 
            class="w-full bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-sm outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] placeholder-[#484f58] transition-all font-mono text-xs"
            onkeydown={(e) => e.key === 'Enter' && addExclusion()}
         />
      </div>
      <button 
         onclick={addExclusion}
         disabled={!newExclusion.trim()}
         class="px-4 py-2 bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 disabled:hover:bg-[#238636] text-white rounded-md text-xs font-bold border border-[rgba(240,246,252,0.1)] transition-all shadow-sm active:scale-[0.98]"
      >
        Add Pattern
      </button>
    </div>
  </div>
</div>
