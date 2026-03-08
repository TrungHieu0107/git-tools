<script lang="ts">
  import { onMount } from 'svelte';
  import { GitService, type AppSettings } from '../lib/GitService';
  import { toast } from '../lib/toast.svelte';
  import EncodingSelector from '../lib/components/EncodingSelector.svelte';

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

  let activeAiProvider = $state<"gemini" | "openrouter">("gemini");
  let openRouterToken = $state("");
  let openRouterModel = $state("google/gemini-2.5-flash");
  let openRouterModelOptions = $state<string[]>([]);
  let savingOpenRouterToken = $state(false);
  let savingOpenRouterModel = $state(false);
  let loadingOpenRouterModels = $state(false);
  let openRouterSaveError = $state("");
  let openRouterModelsError = $state("");

  let globalPrompt = $state("");
  let repoPrompt = $state("");
  let defaultAiPrompt = $state("");
  let savingGlobalPrompt = $state(false);
  let savingRepoPrompt = $state(false);
  let promptSaveError = $state("");

  let repoDefaultEncoding = $state("UTF-8");
  let savingRepoEncoding = $state(false);

  const DEFAULT_GEMINI_MODEL = "gemini-2.5-flash";

  function normalizeGeminiModel(model?: string | null): string {
    const trimmed = (model || "").trim();
    return trimmed || DEFAULT_GEMINI_MODEL;
  }

  function formatGeminiModelLabel(model: string): string {
    return model;
  }

  const DEFAULT_OPEN_ROUTER_MODEL = "google/gemini-2.5-flash";

  function normalizeOpenRouterModel(model?: string | null): string {
    const trimmed = (model || "").trim();
    return trimmed || DEFAULT_OPEN_ROUTER_MODEL;
  }

  function applyLoadedSettings(loaded: AppSettings) {
    settings = loaded;
    if (!settings.excluded_files) {
      settings.excluded_files = [];
    }
    geminiToken = settings.gemini_api_token || "";
    geminiModel = normalizeGeminiModel(settings.gemini_model);
    openRouterToken = settings.open_router_api_token || "";
    openRouterModel = normalizeOpenRouterModel(settings.open_router_model);
    activeAiProvider = settings.active_ai_provider || "gemini";
    globalPrompt = settings.global_commit_prompt || "";
    
    if (repoPath) {
      repoPrompt = (settings.repo_commit_prompts || {})[repoPath] || "";
      const normalizedRepo = repoPath.replace(/\\/g, "/");
      repoDefaultEncoding = (settings.repo_default_encodings || {})[normalizedRepo] || "UTF-8";
    }
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

  async function loadOpenRouterModels(tokenOverride?: string) {
    const token = (tokenOverride || settings?.open_router_api_token || "").trim();
    if (!token) {
      openRouterModelOptions = [];
      openRouterModelsError = "";
      return;
    }

    loadingOpenRouterModels = true;
    openRouterModelsError = "";
    try {
      const models = await GitService.getOpenRouterModels(token);
      openRouterModelOptions = models;

      if (models.length > 0 && !models.includes(openRouterModel) && settings?.open_router_api_token) {
        const fallbackModel = models[0];
        openRouterModel = fallbackModel;
        applyLoadedSettings(await GitService.setOpenRouterModel(fallbackModel));
      }
    } catch (e) {
      openRouterModelOptions = [];
      openRouterModelsError = String(e);
      console.error("Failed to load OpenRouter models", e);
    } finally {
      loadingOpenRouterModels = false;
    }
  }

  onMount(async () => {
    try {
        const [loaded, defaultPrompt] = await Promise.all([
            GitService.getSettings(),
            GitService.getDefaultAiPrompt()
        ]);
        defaultAiPrompt = defaultPrompt;
        applyLoadedSettings(loaded);
        if (loaded.gemini_api_token) {
          await loadGeminiModels(loaded.gemini_api_token);
        }
        if (loaded.open_router_api_token) {
          await loadOpenRouterModels(loaded.open_router_api_token);
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

  async function saveOpenRouterToken() {
    savingOpenRouterToken = true;
    openRouterSaveError = "";
    try {
      const trimmedToken = openRouterToken.trim();
      applyLoadedSettings(await GitService.setOpenRouterApiToken(trimmedToken));
      await loadOpenRouterModels(trimmedToken);
    } catch (e) {
      openRouterSaveError = String(e);
      console.error("Failed to save OpenRouter token", e);
    } finally {
      savingOpenRouterToken = false;
    }
  }

  async function clearOpenRouterToken() {
    openRouterToken = "";
    await saveOpenRouterToken();
  }

  async function saveOpenRouterModel() {
    if (!settings?.open_router_api_token || !openRouterModelOptions.includes(openRouterModel)) return;
    savingOpenRouterModel = true;
    openRouterSaveError = "";
    try {
      applyLoadedSettings(await GitService.setOpenRouterModel(openRouterModel));
    } catch (e) {
      openRouterSaveError = String(e);
      console.error("Failed to save OpenRouter model", e);
    } finally {
      savingOpenRouterModel = false;
    }
  }

  async function saveActiveAiProvider(provider: "gemini" | "openrouter") {
    activeAiProvider = provider;
    try {
      applyLoadedSettings(await GitService.setActiveAiProvider(provider));
    } catch (e) {
      console.error("Failed to save active AI provider", e);
      toast.error(`Failed to switch AI provider: ${e}`);
    }
  }

  async function saveGlobalPrompt() {
    savingGlobalPrompt = true;
    promptSaveError = "";
    try {
      applyLoadedSettings(await GitService.setGlobalCommitPrompt(globalPrompt));
    } catch (e) {
      promptSaveError = String(e);
    } finally {
      savingGlobalPrompt = false;
    }
  }

  async function saveRepoPrompt() {
    if (!repoPath) return;
    savingRepoPrompt = true;
    promptSaveError = "";
    try {
      applyLoadedSettings(await GitService.setRepoCommitPrompt(repoPath, repoPrompt));
    } catch (e) {
      promptSaveError = String(e);
    } finally {
      savingRepoPrompt = false;
    }
  }

  async function handleRepoEncodingChange(encoding: string) {
    if (savingRepoEncoding || !repoPath) return;
    savingRepoEncoding = true;
    try {
      // Empty string clears it dynamically making it fall back to "UTF-8" default without override stored
      const valueToSave = encoding === "UTF-8" ? "" : encoding;
      applyLoadedSettings(await GitService.setRepoDefaultEncoding(repoPath, valueToSave));
      toast.success(`Repository default encoding set to ${encoding}`);
    } catch (e: any) {
      toast.error(`Failed to save repository encoding: ${e}`);
    } finally {
      savingRepoEncoding = false;
    }
  }
</script>
<div class="h-full flex flex-col p-6 bg-[#0d1117] text-[#c9d1d9] overflow-auto">
  <h2 class="text-xl font-bold mb-6 text-white pb-2 border-b border-[#30363d]">Settings</h2>
  
  {#if repoPath}
      <div class="mb-8 p-4 bg-[#161b22] border border-[#30363d] rounded-lg">
          <h3 class="text-sm font-semibold uppercase text-[#8b949e] mb-2 tracking-wider">Active Repository</h3>
          <div class="font-mono text-xs text-[#58a6ff] select-all break-all">
              {repoPath}
          </div>
      </div>
  {/if}

  <div class="max-w-3xl w-full">
    <!-- AI Provider Selection -->
    <div class="mb-10 pb-8 border-b border-[#30363d]">
      <h3 class="flex items-center gap-2 text-sm font-semibold uppercase text-[#8b949e] mb-4 tracking-wider">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12H3"></path><path d="M12 3v18"></path></svg>
        Active AI Provider
      </h3>
      
      <div class="flex gap-6 items-center">
        <label class="flex items-center gap-2 cursor-pointer text-sm text-[#c9d1d9] hover:text-white transition-colors">
          <input 
            type="radio" 
            name="aiProvider" 
            value="gemini"
            checked={activeAiProvider === 'gemini'}
            onchange={() => saveActiveAiProvider('gemini')}
            class="hidden peer"
          />
          <div class="w-4 h-4 rounded-full border border-[#58a6ff] peer-checked:bg-[#58a6ff] peer-checked:ring-2 peer-checked:ring-[#58a6ff]/30 transition-all flex items-center justify-center">
             <div class="w-1.5 h-1.5 bg-[#0d1117] rounded-full opacity-0 peer-checked:opacity-100 transition-opacity"></div>
          </div>
          Google Gemini
        </label>
        
        <label class="flex items-center gap-2 cursor-pointer text-sm text-[#c9d1d9] hover:text-white transition-colors">
          <input 
            type="radio" 
            name="aiProvider" 
            value="openrouter"
            checked={activeAiProvider === 'openrouter'}
            onchange={() => saveActiveAiProvider('openrouter')}
            class="hidden peer"
          />
          <div class="w-4 h-4 rounded-full border border-[#58a6ff] peer-checked:bg-[#58a6ff] peer-checked:ring-2 peer-checked:ring-[#58a6ff]/30 transition-all flex items-center justify-center">
             <div class="w-1.5 h-1.5 bg-[#0d1117] rounded-full opacity-0 peer-checked:opacity-100 transition-opacity"></div>
          </div>
          OpenRouter
        </label>
      </div>
    </div>

    {#if activeAiProvider === 'gemini'}
      <!-- Gemini Config -->
      <div class="mb-10 pb-8 border-b border-[#30363d] animate-fade-in">
        <h3 class="flex items-center gap-2 text-sm font-semibold uppercase text-[#8b949e] mb-4 tracking-wider">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>
          Gemini AI Configuration
        </h3>
        
        <div class="space-y-4">
          <div>
            <label for="gemini-token" class="text-xs text-[#8b949e] block mb-2">API Token</label>
            <div class="flex flex-wrap gap-2 items-center">
              <div class="flex-1 min-w-[220px]">
                <input
                  id="gemini-token"
                  type="password"
                  bind:value={geminiToken}
                  placeholder="Enter Gemini API token..."
                  class="w-full bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-sm outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] placeholder-[#484f58] transition-all font-mono text-xs"
                />
              </div>
              <button
                onclick={saveGeminiToken}
                disabled={savingGeminiToken}
                class="shrink-0 px-4 py-2 bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 text-white rounded-md text-xs font-bold transition-all border border-[rgba(240,246,252,0.1)] shadow-sm"
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
          </div>

          {#if settings?.gemini_api_token}
            <div>
              <label for="gemini-model" class="text-xs text-[#8b949e] block mb-2">Model</label>
              <div class="flex flex-wrap items-center gap-2">
                <select
                  id="gemini-model"
                  bind:value={geminiModel}
                  onchange={saveGeminiModel}
                  disabled={savingGeminiModel || loadingGeminiModels || geminiModelOptions.length === 0}
                  class="w-full sm:w-auto bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-xs outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-all min-w-[220px]"
                >
                  {#if geminiModelOptions.length === 0}
                    <option value="" disabled>{loadingGeminiModels ? "Loading models..." : "No models available"}</option>
                  {:else}
                    {#each geminiModelOptions as model}
                      <option value={model}>{model}</option>
                    {/each}
                  {/if}
                </select>
                {#if loadingGeminiModels}
                  <span class="text-[11px] text-[#8b949e] animate-pulse">Loading models...</span>
                {/if}
              </div>
            </div>
          {/if}

          {#if geminiSaveError || geminiModelsError}
            <p class="text-[11px] text-[#f85149] mt-2 bg-[#f85149]/10 p-2 rounded border border-[#f85149]/20 break-all">
              {geminiSaveError || geminiModelsError}
            </p>
          {/if}
        </div>
      </div>
    {/if}

    {#if activeAiProvider === 'openrouter'}
      <!-- OpenRouter Config -->
      <div class="mb-10 pb-8 border-b border-[#30363d] animate-fade-in">
        <h3 class="flex items-center gap-2 text-sm font-semibold uppercase text-[#8b949e] mb-4 tracking-wider">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line></svg>
          OpenRouter AI Configuration
        </h3>
        
        <div class="space-y-4">
          <div>
            <label for="openrouter-token" class="text-xs text-[#8b949e] block mb-2">API Token</label>
            <div class="flex flex-wrap gap-2 items-center">
              <div class="flex-1 min-w-[220px]">
                <input
                  id="openrouter-token"
                  type="password"
                  bind:value={openRouterToken}
                  placeholder="Enter OpenRouter API token..."
                  class="w-full bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-sm outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] placeholder-[#484f58] transition-all font-mono text-xs"
                />
              </div>
              <button
                onclick={saveOpenRouterToken}
                disabled={savingOpenRouterToken}
                class="shrink-0 px-4 py-2 bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 text-white rounded-md text-xs font-bold transition-all border border-[rgba(240,246,252,0.1)] shadow-sm"
              >
                {savingOpenRouterToken ? 'Saving...' : 'Save Token'}
              </button>
              <button
                onclick={clearOpenRouterToken}
                disabled={savingOpenRouterToken || !settings?.open_router_api_token}
                class="shrink-0 px-4 py-2 bg-[#21262d] hover:bg-[#30363d] disabled:opacity-50 text-[#c9d1d9] rounded-md text-xs font-bold border border-[#30363d] transition-all"
              >
                Clear
              </button>
            </div>
          </div>

          {#if settings?.open_router_api_token}
            <div>
              <label for="openrouter-model" class="text-xs text-[#8b949e] block mb-2">Model</label>
              <div class="flex flex-wrap items-center gap-2">
                <select
                  id="openrouter-model"
                  bind:value={openRouterModel}
                  onchange={saveOpenRouterModel}
                  disabled={savingOpenRouterModel || loadingOpenRouterModels || openRouterModelOptions.length === 0}
                  class="w-full sm:w-auto bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-xs outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-all min-w-[220px] max-w-full"
                >
                  {#if openRouterModelOptions.length === 0}
                    <option value="" disabled>{loadingOpenRouterModels ? "Loading models..." : "No models available"}</option>
                  {:else}
                    {#each openRouterModelOptions as model}
                      <option value={model}>{model}</option>
                    {/each}
                  {/if}
                </select>
                {#if loadingOpenRouterModels}
                  <span class="text-[11px] text-[#8b949e] animate-pulse">Loading models...</span>
                {/if}
              </div>
            </div>
          {/if}

          {#if openRouterSaveError || openRouterModelsError}
            <p class="text-[11px] text-[#f85149] mt-2 bg-[#f85149]/10 p-2 rounded border border-[#f85149]/20 break-all">
              {openRouterSaveError || openRouterModelsError}
            </p>
          {/if}
        </div>
      </div>
    {/if}

    <!-- AI Prompt Settings -->
    <div class="mb-10 pb-8 border-b border-[#30363d]">
      <h3 class="flex items-center gap-2 text-sm font-semibold uppercase text-[#8b949e] mb-4 tracking-wider">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
        AI Commit Message Prompt
      </h3>
      
      <p class="text-xs text-[#8b949e] mb-6 leading-relaxed">
        Customize the instructions sent to Gemini. Use this to enforce specific styles like <code>feat: description</code> or <code>[Project] Message</code>.
        The AI will prioritize the <strong>Repository Prompt</strong> over the <strong>Global Prompt</strong>.
      </p>

      <!-- Global Prompt -->
      <div class="mb-8">
        <div class="flex items-center justify-between mb-2">
          <label class="text-xs font-medium text-[#c9d1d9]">Global Default Prompt</label>
          <span class="text-[10px] bg-[#21262d] text-[#8b949e] px-2 py-0.5 rounded border border-[#30363d]">Default fallback</span>
        </div>
        <textarea
          bind:value={globalPrompt}
          placeholder={defaultAiPrompt || "Loading default prompt..."}
          class="w-full h-32 bg-[#0d1117] border border-[#30363d] p-3 rounded-md text-xs font-mono outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] placeholder-[#484f58] transition-all"
        ></textarea>
        <div class="mt-2 flex justify-end gap-2">
          <button
            onclick={() => { globalPrompt = ""; saveGlobalPrompt(); }}
            class="px-3 py-1.5 text-[11px] text-[#8b949e] hover:text-[#c9d1d9] transition-colors"
          >
            Reset to Default
          </button>
          <button
            onclick={saveGlobalPrompt}
            disabled={savingGlobalPrompt}
            class="px-4 py-1.5 bg-[#21262d] hover:bg-[#30363d] disabled:opacity-50 text-white rounded text-[11px] font-bold border border-[#30363d] transition-all"
          >
            {savingGlobalPrompt ? 'Saving...' : 'Save Global Prompt'}
          </button>
        </div>
      </div>

      <!-- Repo Specific Prompt -->
      {#if repoPath}
        <div class="mt-8 p-4 bg-[#1f242b]/30 border border-[#30363d] rounded-lg">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-2">
              <label class="text-xs font-bold text-[#58a6ff]">Repository Specific Prompt</label>
              <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="text-[#58a6ff]"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L22 22"/></svg>
            </div>
            <span class="text-[10px] bg-[#14485c] text-[#9ee7ff] px-2 py-0.5 rounded border border-[#36a9da]/30">Override active</span>
          </div>
          <textarea
            bind:value={repoPrompt}
            placeholder={defaultAiPrompt || "Loading default prompt..."}
            class="w-full h-32 bg-[#0d1117] border border-[#30363d] p-3 rounded-md text-xs font-mono outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] placeholder-[#484f58] transition-all"
          ></textarea>
          <div class="mt-3 flex justify-end gap-2">
            <button
              onclick={() => { repoPrompt = ""; saveRepoPrompt(); }}
              class="px-3 py-1.5 text-[11px] text-[#8b949e] hover:text-[#c9d1d9] transition-colors"
            >
              Clear Override
            </button>
            <button
              onclick={saveRepoPrompt}
              disabled={savingRepoPrompt}
              class="px-4 py-1.5 bg-[#58a6ff] hover:bg-[#388bfd] disabled:bg-[#1f242b] disabled:text-[#484f58] text-[#0d1117] rounded text-[11px] font-bold transition-all shadow-sm"
            >
              {savingRepoPrompt ? 'Saving...' : 'Apply to This Repo'}
            </button>
          </div>
          
          <!-- Repository Default Encoding -->
          <div class="mt-6 pt-6 border-t border-[#30363d] flex flex-col items-start gap-2">
            <label class="text-xs font-medium text-[#c9d1d9]">Repository Default File Encoding</label>
            <p class="text-xs text-[#8b949e]">
              The default text encoding Git Tools uses when parsing files and diffs in this repository.
            </p>
            <div class="mt-2" class:opacity-50={savingRepoEncoding} class:pointer-events-none={savingRepoEncoding}>
              <EncodingSelector 
                selectedEncoding={repoDefaultEncoding} 
                on:change={(e) => handleRepoEncodingChange(e.detail)} 
              />
            </div>
          </div>
        </div>
      {/if}

      {#if promptSaveError}
        <p class="text-[11px] text-[#f85149] mt-4 bg-[#f85149]/10 p-2 rounded border border-[#f85149]/20">
          {promptSaveError}
        </p>
      {/if}
    </div>

    <!-- Exclusions -->
    <div>
      <h3 class="flex items-center gap-2 text-sm font-semibold uppercase text-[#8b949e] mb-4 tracking-wider">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="9" y1="9" x2="15" y2="15"></line><line x1="15" y1="9" x2="9" y2="15"></line></svg>
        Global File Exclusions
      </h3>
      <p class="text-xs text-[#8b949e] mb-4 leading-relaxed">
        Patterns to virtually ignore in this app (e.g., <code>*.log</code>, <code>dist/**</code>).
      </p>

      <div class="bg-[#161b22] border border-[#30363d] rounded-md overflow-hidden mb-4">
          {#if !settings?.excluded_files || settings.excluded_files.length === 0}
              <div class="p-6 text-center text-xs text-[#484f58] italic">No excluded patterns defined.</div>
          {:else}
              {#each settings.excluded_files as exc, i}
                  <div class="flex items-center gap-3 px-4 py-2.5 border-b border-[#30363d] last:border-0 hover:bg-[#0d1117] transition-colors group">
                      <span class="text-[#484f58] select-none text-[10px] font-mono">{i + 1}.</span>
                      <span class="flex-1 font-mono text-xs text-[#c9d1d9]">{exc}</span>
                      <button 
                          onclick={() => removeExclusion(i)} 
                          class="text-[#8b949e] hover:text-[#f85149] p-1 rounded transition-colors"
                      >
                          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                      </button>
                  </div>
              {/each}
          {/if}
      </div>

      <div class="flex flex-wrap gap-2 items-center">
        <input 
          type="text" 
          bind:value={newExclusion} 
          placeholder="Enter glob pattern..." 
          class="flex-1 bg-[#0d1117] border border-[#30363d] px-3 py-2 rounded-md text-sm outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] placeholder-[#484f58] transition-all font-mono text-xs"
          onkeydown={(e) => e.key === 'Enter' && addExclusion()}
        />
        <button 
          onclick={addExclusion}
          disabled={!newExclusion.trim()}
          class="px-4 py-2 bg-[#21262d] hover:bg-[#30363d] disabled:opacity-50 text-[#c9d1d9] rounded-md text-xs font-bold border border-[#30363d] transition-all shadow-sm"
        >
          Add Pattern
        </button>
      </div>
    </div>
  </div>
</div>
