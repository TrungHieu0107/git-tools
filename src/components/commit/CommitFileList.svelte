<script lang="ts">
  import type { FileStatus } from "../../lib/GitService";

  interface Props {
      title: string;
      files: FileStatus[];
      selectedFile: FileStatus | null;
      onSelect: (file: FileStatus) => void;
      onAction: (file: FileStatus) => void; // Stage or Unstage action
      actionLabel: string; // "Stage" or "Unstage" text for tooltip/aria (or implied by context)
      actionIcon?: string; // Optional custom icon?
      onActionAll?: () => void;
      actionAllLabel?: string;
  }
  let { title, files, selectedFile, onSelect, onAction, actionLabel, onActionAll, actionAllLabel }: Props = $props();

  function getStatusColor(code: string) {
      switch (code) {
          case 'M': return 'text-yellow-400';
          case 'A': return 'text-green-400';
          case 'D': return 'text-red-400';
          case 'R': return 'text-blue-400';
          case '??': return 'text-gray-400';
          default: return 'text-gray-400';
      }
  }
</script>

<div class="flex flex-col flex-1 overflow-hidden min-h-0 border-b border-[#30363d] last:border-b-0">
    <div class="h-8 px-3 flex items-center bg-[#21262d] font-semibold text-xs uppercase tracking-wider text-[#8b949e] shrink-0 justify-between group/header">
        <span>{title} ({files.length})</span>
        {#if files.length > 0 && onActionAll}
            <button 
                class="opacity-90 hover:opacity-100 transition-opacity px-2 py-1 rounded hover:bg-[#30363d] text-[#58a6ff] hover:text-[#79c0ff] text-xs font-medium flex items-center gap-1.5"
                onclick={(e) => { e.stopPropagation(); onActionAll(); }}
                title={actionAllLabel}
            >
                {#if actionAllLabel?.toLowerCase().includes('unstage')}
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
                {:else}
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14"/></svg>
                {/if}
                {actionAllLabel}
            </button>
        {/if}
    </div>
    <div class="flex-1 overflow-y-auto custom-scrollbar p-1">
        {#if files.length === 0}
            <div class="text-[#8b949e] text-xs text-center mt-4 italic opacity-60">Empty</div>
        {:else}
            {#each files as file}
                <div 
                    class="group flex items-center gap-2 px-2 py-1.5 text-xs rounded cursor-pointer transition-colors relative
                           {selectedFile === file ? 'bg-[#30363d] text-white' : 'hover:bg-[#21262d] text-[#c9d1d9]'}"
                    onclick={() => onSelect(file)}
                >
                    <span class="{getStatusColor(file.status)} font-mono w-4 shrink-0 text-center">{file.status}</span>
                    <span class="truncate flex-1" title={file.path}>{file.path}</span>
                    
                    <!-- Action Button (appears on hover) -->
                    <button 
                        class="opacity-0 group-hover:opacity-100 p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-white transition-opacity"
                        onclick={(e) => { e.stopPropagation(); onAction(file); }}
                        title={actionLabel}
                    >
                        {#if actionLabel === 'Stage'}
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                        {:else}
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                        {/if}
                    </button>
                </div>
            {/each}
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
