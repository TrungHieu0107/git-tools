<script lang="ts">
  import { toast } from "../lib/toast.svelte";
  import { fly } from "svelte/transition";

  // Icons
  const Icons = {
    Success: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>`,
    Error: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>`,
    Info: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>`
  };
</script>

<div class="fixed bottom-4 right-4 flex flex-col gap-2 z-[9999] pointer-events-none">
  {#each toast.toasts as t (t.id)}
    <div
      class="pointer-events-auto min-w-[300px] max-w-[400px] rounded-md shadow-lg border border-[#30363d] overflow-hidden flex items-start gap-3 p-3 text-sm
          {t.type === 'success' ? 'bg-[#0d1117] text-[#3fb950] border-[#2ea043]/30' : 
           t.type === 'error' ? 'bg-[#0d1117] text-[#f85149] border-[#da3633]/30' : 
           'bg-[#0d1117] text-[#58a6ff] border-[#1f6feb]/30'}"
      in:fly={{ y: 20, duration: 300 }}
      out:fly={{ x: 20, duration: 200 }}
    >
      <span class="mt-0.5 shrink-0">
          {#if t.type === 'success'}
            {@html Icons.Success}
          {:else if t.type === 'error'}
            {@html Icons.Error}
          {:else}
            {@html Icons.Info}
          {/if}
      </span>
      <div class="flex-1 text-[#c9d1d9] leading-tight break-words">
          <div class="font-semibold mb-0.5 capitalize {t.type === 'success' ? 'text-[#3fb950]' : t.type === 'error' ? 'text-[#f85149]' : 'text-[#58a6ff]'}">{t.type}</div>
          {t.message}
      </div>
      <button 
        class="text-[#8b949e] hover:text-[#c9d1d9] transition-colors -mt-1 -mr-1 p-1 rounded hover:bg-[#21262d]"
        onclick={() => toast.remove(t.id)}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>
  {/each}
</div>
