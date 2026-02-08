<script lang="ts">
  import { type RepoEntry } from '../lib/GitService';

  interface Props {
    openRepos: RepoEntry[];
    activeRepoId: string | null;
    onActivate: (id: string) => void;
    onClose: (id: string) => void;
    onAdd: () => void;
  }

  let { openRepos, activeRepoId, onActivate, onClose, onAdd }: Props = $props();

  function handleWheel(e: WheelEvent) {
      if (e.deltaY !== 0) {
          e.preventDefault();
          (e.currentTarget as HTMLElement).scrollBy({ left: e.deltaY, behavior: 'auto' });
      }
  }
</script>

<div class="flex h-12 bg-[#0d1117] border-b border-[#30363d] select-none overflow-hidden shrink-0">
  <!-- Tabs Container -->
  <div 
    class="flex-1 flex custom-scrollbar items-end px-2 gap-1"
    onwheel={handleWheel}
    role="tablist"
    tabindex="0"
    aria-label="Open Repositories"
  >
    {#each openRepos as repo (repo.id)}
      <div 
        class="group relative flex items-center h-9 max-w-[200px] min-w-[120px] rounded-t-md px-3 border-t border-l border-r border-transparent cursor-pointer transition-colors
               {activeRepoId === repo.id 
                 ? 'bg-[#161b22] border-[#30363d] text-[#c9d1d9] z-10' 
                 : 'bg-[#0d1117] hover:bg-[#161b22]/50 text-[#8b949e] hover:text-[#c9d1d9]'}"
        onclick={() => onActivate(repo.id)}
        role="tab"
        aria-selected={activeRepoId === repo.id}
        tabindex="0"
        onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onActivate(repo.id);
            }
        }}
        title={repo.path}
      >
        <!-- Icon -->
        <span class="mr-2 opacity-70">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>
        </span>
        
        <!-- Title -->
        <span class="truncate text-xs font-medium flex-1">
          {repo.name}
        </span>

        <!-- Close Button -->
        <button
          class="ml-1 p-0.5 rounded-md text-[#8b949e] opacity-0 group-hover:opacity-100 hover:bg-[#30363d] hover:text-white transition-all focus:opacity-100"
          onclick={(e) => {
            e.stopPropagation();
            onClose(repo.id);
          }}
          title="Close Tab"
          aria-label="Close Tab"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
        
        {#if activeRepoId === repo.id}
            <div class="absolute bottom-[-1px] left-0 right-0 h-[1px] bg-[#161b22]"></div>
        {/if}
      </div>
    {/each}
  </div>

  <!-- Add Button / Repo Manager -->
  <button
    class="flex items-center justify-center w-10 h-full text-[#8b949e] hover:text-[#c9d1d9] hover:bg-[#161b22] transition-colors border-l border-[#30363d]"
    onclick={onAdd}
    title="Open Repository Manager"
  >
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
  </button>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    height: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #30363d;
    border-radius: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #484f58;
  }
</style>
