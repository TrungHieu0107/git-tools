<script lang="ts">
  import { getConfirmationState } from "../lib/confirmation.svelte";
  
  const state = getConfirmationState();
  
  function handleConfirm() {
      state.resolve(true);
  }
  
  function handleCancel() {
      state.resolve(false);
  }

  function onKeydown(e: KeyboardEvent) {
      if (!state.isOpen) return;
      if (e.key === "Escape") {
          e.stopPropagation();
          handleCancel();
      }
      if (e.key === "Enter") {
          e.stopPropagation();
          handleConfirm(); // Experimental: Allow enter to confirm
      }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if state.isOpen}
  <!-- Backdrop -->
  <div class="fixed inset-0 z-[100] bg-black/60 backdrop-blur-sm flex items-center sm:items-start justify-center p-4 sm:pt-10" role="presentation">
      <!-- Modal Container - Centered, Compact -->
      <div 
          class="bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl overflow-hidden animate-in fade-in slide-in-from-top-4 duration-200 flex flex-col"
          style="width: min(420px, 100%);" 
          role="dialog"
      >
          <!-- Header -->
          <div class="px-4 py-3 border-b border-[#30363d] bg-[#0d1117] flex justify-between items-center shrink-0">
              <h3 class="text-sm font-semibold text-white">{state.options.title}</h3>
              <button onclick={handleCancel} class="text-[#8b949e] hover:text-[#c9d1d9] p-1 rounded hover:bg-[#30363d] transition-colors">
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
              </button>
          </div>
          
          <!-- Content -->
          <div class="p-4 bg-[#161b22] overflow-y-auto">
              <div class="text-xs text-[#c9d1d9] mb-4 leading-relaxed">
                  {#if state.options.isHtmlMessage}
                      {@html state.options.message}
                  {:else}
                      {state.options.message}
                  {/if}
              </div>

              <div class="flex flex-wrap justify-end gap-2 pt-1">
                   <button 
                      class="px-3 py-1.5 text-xs font-medium text-[#c9d1d9] hover:text-white bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded transition-colors"
                      onclick={handleCancel}
                   >
                      {state.options.cancelLabel}
                   </button>
                   <button 
                      class="px-3 py-1.5 text-xs font-medium text-white bg-[#238636] hover:bg-[#2ea043] rounded border border-[rgba(240,246,252,0.1)] shadow-sm transition-colors focus:ring-2 focus:ring-[#238636] focus:outline-none"
                      onclick={handleConfirm}
                      autofocus
                   >
                      {state.options.confirmLabel}
                   </button>
              </div>
          </div>
      </div>
  </div>
{/if}
