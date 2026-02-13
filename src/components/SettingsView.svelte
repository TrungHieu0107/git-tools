<script lang="ts">
  import { onMount } from 'svelte';
  import { GitService, type AppSettings } from '../lib/GitService';

  interface Props {
      repoPath?: string;
  }
  let { repoPath }: Props = $props();

  let settings = $state<AppSettings | null>(null);
  let newExclusion = $state("");
  let geminiToken = $state("");
  let geminiModel = $state("gemini-2.5-flash");
  let geminiModelOptions = $state<string[]>([]);
  let savingGeminiToken = $state(false);
  let savingGeminiModel = $state(false);
  let loadingGeminiModels = $state(false);
  let geminiSaveError = $state("");
  let geminiModelsError = $state("");

  const DEFAULT_GEMINI_MODEL = "gemini-2.5-flash";

  function normalizeGeminiModel(model?: string | null): string {
    const trimmed = (model || "").trim();
    return trimmed || DEFAULT_GEMINI_MODEL;
  }

  function formatGeminiModelLabel(model: string): string {
    return model;
  }

  function applyLoadedSettings(loaded: AppSettings) {
    settings = loaded;
    if (!settings.excluded_files) {
      settings.excluded_files = [];
    }
    geminiToken = settings.gemini_api_token || "";
    geminiModel = normalizeGeminiModel(settings.gemini_model);
  }

  async function loadGeminiModels(tokenOverride?: string) {
    const token = (tokenOverride || settings?.gemini_api_token || "").trim();
    if (!token) {
      geminiModelOptions = [];
      geminiModelsError = "";
      return;
    }

    loadingGeminiModels = true;
    geminiModelsError = "";
    try {
      const models = await GitService.getGeminiModels(token);
      geminiModelOptions = models;

      if (models.length > 0 && !models.includes(geminiModel) && settings?.gemini_api_token) {
        const fallbackModel = models[0];
        geminiModel = fallbackModel;
        applyLoadedSettings(await GitService.setGeminiModel(fallbackModel));
      }
    } catch (e) {
      geminiModelOptions = [];
      geminiModelsError = String(e);
      console.error("Failed to load Gemini models", e);
    } finally {
      loadingGeminiModels = false;
    }
  }

  onMount(async () => {
    try {
        const loaded = await GitService.getSettings();
        applyLoadedSettings(loaded);
        if (loaded.gemini_api_token) {
          await loadGeminiModels(loaded.gemini_api_token);
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
        applyLoadedSettings(await GitService.setExcludedFiles(exclusions));
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
        applyLoadedSettings(await GitService.setExcludedFiles(exclusions));
    } catch (e) {
        console.error("Failed to remove exclusion", e);
    }
  }

  async function saveGeminiToken() {
    savingGeminiToken = true;
    geminiSaveError = "";
    try {
      const trimmedToken = geminiToken.trim();
      applyLoadedSettings(await GitService.setGeminiApiToken(trimmedToken));
      await loadGeminiModels(trimmedToken);
    } catch (e) {
      geminiSaveError = String(e);
      console.error("Failed to save Gemini token", e);
    } finally {
      savingGeminiToken = false;
    }
  }

  async function clearGeminiToken() {
    geminiToken = "";
    await saveGeminiToken();
  }

  async function saveGeminiModel() {
    if (!settings?.gemini_api_token || !geminiModelOptions.includes(geminiModel)) return;
    savingGeminiModel = true;
    geminiSaveError = "";
    try {
      applyLoadedSettings(await GitService.setGeminiModel(geminiModel));
    } catch (e) {
      geminiSaveError = String(e);
      console.error("Failed to save Gemini model", e);
    } finally {
      savingGeminiModel = false;
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

  <div class="max-w-2xl w-full">
    <div class="mb-8">
      <h3 class="text-sm font-semibold uppercase text-[#8b949e] mb-2 tracking-wider">Gemini Commit Message</h3>
      <p class="text-xs text-[#8b949e] mb-4 leading-relaxed">
        Configure a global Gemini API token to auto-generate commit messages from staged changes.
        Token is stored locally in this app settings file.
      </p>

      <div class="flex flex-wrap gap-2 items-center">
        <div class="flex-1 min-w-[220px]">
          <input
            type="password"
            bind:value={geminiToken}
            placeholder="Enter Gemini API token..."
            class="w-full bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-sm outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] placeholder-[#484f58] transition-all font-mono text-xs"
            onkeydown={(e) => e.key === 'Enter' && saveGeminiToken()}
          />
        </div>
        <button
          onclick={saveGeminiToken}
          disabled={savingGeminiToken}
          class="shrink-0 px-4 py-2 bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 disabled:hover:bg-[#238636] text-white rounded-md text-xs font-bold border border-[rgba(240,246,252,0.1)] transition-all shadow-sm active:scale-[0.98]"
        >
          {savingGeminiToken ? 'Saving...' : 'Save Token'}
        </button>
        <button
          onclick={clearGeminiToken}
          disabled={savingGeminiToken || !settings?.gemini_api_token}
          class="shrink-0 px-4 py-2 bg-[#21262d] hover:bg-[#30363d] disabled:opacity-50 text-[#c9d1d9] rounded-md text-xs font-bold border border-[#30363d] transition-all"
        >
          Clear
        </button>
      </div>

      {#if settings?.gemini_api_token}
        <p class="text-[11px] text-[#8b949e] mt-2">Gemini token is configured.</p>
      {:else}
        <p class="text-[11px] text-[#8b949e] mt-2">No Gemini token configured.</p>
      {/if}

      {#if settings?.gemini_api_token}
        <div class="mt-4">
          <label for="gemini-model" class="text-xs text-[#8b949e] block mb-1.5">Gemini model</label>
          <div class="flex flex-wrap items-center gap-2">
            <select
              id="gemini-model"
              bind:value={geminiModel}
              onchange={saveGeminiModel}
              disabled={savingGeminiModel || savingGeminiToken || loadingGeminiModels || geminiModelOptions.length === 0}
              class="w-full sm:w-auto bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-xs outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-all min-w-0 sm:min-w-[220px]"
            >
              {#if geminiModelOptions.length === 0}
                <option value="" disabled>
                  {loadingGeminiModels ? "Loading models..." : "No models available"}
                </option>
              {:else}
                {#each geminiModelOptions as model}
                  <option value={model}>{formatGeminiModelLabel(model)}</option>
                {/each}
              {/if}
            </select>
            {#if loadingGeminiModels}
              <span class="text-[11px] text-[#8b949e]">Loading models...</span>
            {:else if savingGeminiModel}
              <span class="text-[11px] text-[#8b949e]">Saving model...</span>
            {/if}
          </div>
          {#if geminiModelsError}
            <p class="text-[11px] text-[#f85149] mt-2 break-all">{geminiModelsError}</p>
          {/if}
        </div>
      {/if}

      {#if geminiSaveError}
        <p class="text-[11px] text-[#f85149] mt-2 break-all">{geminiSaveError}</p>
      {/if}
    </div>

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
                        class="text-[#8b949e] hover:text-[#f85149] opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity p-1 rounded"
                        title="Remove exclusion"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
            {/each}
        {/if}
    </div>

    <!-- Add -->
    <div class="flex flex-wrap gap-2 items-center">
      <div class="flex-1 min-w-[220px] relative">
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
         class="w-full sm:w-auto px-4 py-2 bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 disabled:hover:bg-[#238636] text-white rounded-md text-xs font-bold border border-[rgba(240,246,252,0.1)] transition-all shadow-sm active:scale-[0.98]"
      >
        Add Pattern
      </button>
    </div>
  </div>
</div>
