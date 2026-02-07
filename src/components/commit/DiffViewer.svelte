<script lang="ts">
  interface Props {
      diff: string;
      loading: boolean;
  }
  let { diff, loading }: Props = $props();

  // Colorize logic
  function formatDiff(d: string): string {
        if (!d) return "";
        return d.split('\n').map(line => {
            if (line.startsWith('+')) {
                return `<span class="block bg-[#2ea043]/10 text-[#3fb950] w-full">${escapeHtml(line)}</span>`;
            } else if (line.startsWith('-')) {
                return `<span class="block bg-[#da3633]/10 text-[#f85149] w-full">${escapeHtml(line)}</span>`;
            } else if (line.startsWith('@@')) {
                return `<span class="block text-[#79c0ff] my-1 pt-1 border-t border-[#30363d]/30">${escapeHtml(line)}</span>`;
            } else {
                return `<span class="block text-[#c9d1d9] opacity-80">${escapeHtml(line)}</span>`;
            }
        }).join('');
    }

    function escapeHtml(unsafe: string) {
        return unsafe
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
    }
</script>

<div class="flex-1 overflow-auto custom-scrollbar p-0 bg-[#0d1117] h-full relative">
    {#if loading}
        <div class="absolute inset-0 flex items-center justify-center bg-[#0d1117]/50 z-10">
             <svg class="animate-spin h-6 w-6 text-[#8b949e]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>
        </div>
    {/if}

    {#if !diff && !loading}
        <div class="flex items-center justify-center p-8 text-[#8b949e] text-sm italic">
            No diff content
        </div>
    {:else}
        <code class="block text-xs font-mono whitespace-pre p-4">
            {@html formatDiff(diff)}
        </code>
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
</style>
