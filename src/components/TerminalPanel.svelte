<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { GitService } from '../lib/GitService';
  
  interface Props {
      repoPath: string;
      isActive: boolean;
  }
  let { repoPath, isActive }: Props = $props();

  let outputLines = $state<string[]>([]);
  let commandInput = $state('');
  let terminalEndRef: HTMLElement | undefined = $state();
  let unlisten: UnlistenFn | null = null;
  let isRunning = $state(false);

  // Auto-scroll to bottom of terminal
  $effect(() => {
     if (outputLines.length && isActive) {
         scrollToBottom();
     }
  });

  function scrollToBottom() {
      if (terminalEndRef) {
          terminalEndRef.scrollIntoView({ behavior: 'smooth' });
      }
  }

  // Start terminal session when repoPath changes or component mounts
  $effect(() => {
      if (repoPath) {
          startSession();
      }
      return () => {
          // Optional: stop session on unmount? 
          // Requirements say "Switching repositories must create or rebind".
          // If we unmount, we might want to keep it running if just switching tabs?
          // But if repoPath changes, we definitely switch session.
          // For now, let's keep it running in backend, but we unsubscribe frontend.
      };
  });

  async function startSession() {
      outputLines = []; // Clear output on new session or attach?
      // Ideally we'd fetch previous history if we want persistence, but for now we just attach.
      // If we are just switching tabs, outputLines is preserved in state if this component is kept alive.
      // If `TerminalPanel` is destroyed/remounted (e.g. `{#if activeTab === 'terminal'}`), state is lost.
      // In App.svelte, we use `class:invisible` often to preserve state. 
      // I'll check App.svelte usage.
      
      try {
          await GitService.startTerminal(repoPath);
          isRunning = true;
      } catch (e) {
          outputLines.push(`Error starting terminal: ${e}`);
          isRunning = false;
      }
  }

  onMount(async () => {
      unlisten = await listen<{ repoPath: string, type: string, data: string }>('terminal-output', (event) => {
          if (event.payload.repoPath === repoPath) {
              // Append data
              // data might contain newlines.
              // PowerShell often emits raw lines.
              outputLines.push(event.payload.data);
          }
      });
  });

  onDestroy(() => {
      if (unlisten) unlisten();
  });

  async function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'Enter') {
          e.preventDefault();
          if (!commandInput.trim()) return;
          
          const cmd = commandInput;
          commandInput = ''; // Clear input immediately
          
          // Echo input for visibility (PowerShell might allow echo, but explicit echo is good)
          outputLines.push(`> ${cmd}`); 

          try {
              await GitService.writeTerminal(repoPath, cmd);
          } catch (e) {
              outputLines.push(`Error writing to terminal: ${e}`);
          }
      }
  }

</script>

<div class="flex flex-col h-full bg-[#0d1117] font-mono text-xs text-[#c9d1d9] p-4">
    <!-- Output Area -->
    <div class="flex-1 overflow-auto custom-scrollbar mb-2 space-y-0.5">
        {#each outputLines as line}
            <div class="whitespace-pre-wrap break-words leading-tight min-h-[1.2em]">{line}</div>
        {/each}
        <div bind:this={terminalEndRef}></div>
    </div>

    <!-- Input Area -->
    <div class="flex items-center gap-2 border-t border-[#30363d] pt-2">
        <span class="text-[#58a6ff]">❯</span>
        <input 
            type="text" 
            bind:value={commandInput}
            onkeydown={handleKeydown}
            class="flex-1 bg-transparent outline-none placeholder-[#484f58]"
            placeholder={isRunning ? "Enter command..." : "Terminal not ready"}
            disabled={!isRunning}
            spellcheck="false"
            autocomplete="off"
        />
    </div>
</div>
