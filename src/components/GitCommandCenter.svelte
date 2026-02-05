<script lang="ts">
  import { onMount } from 'svelte';
  import { GitService } from '../lib/GitService';
  import { GIT_COMMANDS, type GitCommand } from '../lib/GitCommandService';

  let activeRepo = $state(false);
  let selectedCommand = $state<GitCommand | null>(null);
  let commandInput = $state('');
  let output = $state('');
  let error = $state<string | null>(null);
  let loading = $state(false);

  onMount(async () => {
      try {
          const repo = await GitService.getActiveRepo();
          activeRepo = !!repo;
          if (activeRepo) {
              selectedCommand = GIT_COMMANDS[0];
          }
      } catch (e) {
          console.error(e);
      }
  });

  function selectCommand(cmd: GitCommand) {
      selectedCommand = cmd;
      commandInput = '';
      output = '';
      error = null;
  }

  async function runCommand() {
      if (!selectedCommand) return;
      
      loading = true;
      output = '';
      error = null;

      try {
          const result = await selectedCommand.run(selectedCommand.needsInput ? commandInput : undefined);
          if (Array.isArray(result)) {
              output = result.join('\n');
          } else {
              output = result;
          }
      } catch (e) {
          error = String(e);
      } finally {
          loading = false;
      }
  }

  function copyOutput() {
      if (!output) return;
      navigator.clipboard.writeText(output).then(() => {
          // Optional: show toast
      });
  }

  function clearOutput() {
      output = '';
      error = null;
  }
</script>

<div class="flex h-full bg-gray-950 text-gray-200">
    <!-- Sidebar: Command List -->
    <div class="w-64 border-r border-gray-800 bg-gray-900 flex flex-col">
        <div class="p-4 border-b border-gray-800 font-bold text-gray-400 uppercase text-xs tracking-wider">
            Git Commands
        </div>
        <div class="flex-1 overflow-y-auto hidden-scrollbar p-2 space-y-1">
            {#each GIT_COMMANDS as cmd}
                <button 
                    class="w-full text-left px-3 py-2 rounded text-sm transition-colors {selectedCommand?.id === cmd.id ? 'bg-blue-600/20 text-blue-400 border border-blue-600/50' : 'hover:bg-gray-800 text-gray-300 border border-transparent'}"
                    onclick={() => selectCommand(cmd)}
                >
                    <div class="font-medium">{cmd.name}</div>
                </button>
            {/each}
        </div>
    </div>

    <!-- Main Content: Execution Area -->
    <div class="flex-1 flex flex-col min-w-0">
        {#if !activeRepo}
            <div class="flex-1 flex items-center justify-center text-gray-500">
                Please select a repository first.
            </div>
        {:else if selectedCommand}
            <!-- Command Header & Input -->
            <div class="p-6 border-b border-gray-800 bg-gray-900/50">
                <h2 class="text-xl font-bold mb-2 flex items-center gap-2">
                    {selectedCommand.name}
                    {#if loading}
                        <span class="text-xs font-normal text-blue-400 animate-pulse">(Running...)</span>
                    {/if}
                </h2>
                <p class="text-sm text-gray-500 mb-4">{selectedCommand.description}</p>

                <div class="flex gap-4 items-start">
                    {#if selectedCommand.needsInput}
                        <div class="flex-1">
                            <input 
                                type="text" 
                                class="w-full bg-gray-950 border border-gray-700 rounded px-4 py-2 text-sm focus:border-blue-500 focus:outline-none"
                                placeholder={selectedCommand.inputPlaceholder}
                                bind:value={commandInput}
                                onkeydown={(e) => e.key === 'Enter' && !loading && runCommand()}
                            />
                        </div>
                    {/if}
                    <button 
                        class="px-6 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded font-medium text-sm flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-lg shadow-blue-900/20"
                        onclick={runCommand}
                        disabled={loading || (selectedCommand.needsInput && !commandInput)}
                    >
                         {#if loading}
                             <div class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></div>
                         {:else}
                            <span>▶</span>
                         {/if}
                        Run Command
                    </button>
                </div>
            </div>

            <!-- Output Area -->
            <div class="flex-1 flex flex-col min-h-0 bg-gray-950 p-6">
                <div class="flex items-center justify-between mb-2">
                     <span class="text-xs font-bold text-gray-500 uppercase tracking-wider">Output / Console</span>
                     <div class="flex gap-2">
                         <button class="text-xs px-2 py-1 text-gray-500 hover:text-gray-300" onclick={copyOutput} title="Copy Output">📋 Copy</button>
                         <button class="text-xs px-2 py-1 text-gray-500 hover:text-red-400" onclick={clearOutput} title="Clear Output">🧹 Clear</button>
                     </div>
                </div>
                
                <div class="flex-1 bg-black rounded-lg border border-gray-800 p-4 font-mono text-xs overflow-auto shadow-inner relative">
                    {#if loading && !output && !error}
                        <div class="absolute inset-0 flex items-center justify-center bg-black/50 backdrop-blur-sm z-10">
                            <div class="text-blue-400">Executing...</div>
                        </div>
                    {/if}

                    {#if error}
                        <div class="text-red-400 whitespace-pre-wrap">{error}</div>
                    {:else if output}
                        <div class="text-green-400 whitespace-pre-wrap">{output}</div>
                    {:else}
                         <div class="text-gray-600 italic">Ready to run...</div>
                    {/if}
                </div>
            </div>
        {/if}
    </div>
</div>
