<script lang="ts">
  import { getPromptState } from "../../lib/prompt.svelte";

  const promptState = getPromptState();

  let inputEl: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (!promptState.isOpen) return;
    requestAnimationFrame(() => {
      inputEl?.focus();
      inputEl?.select();
    });
  });

  function handleCancel() {
    promptState.resolve(null);
  }

  function handleConfirm() {
    promptState.resolve(promptState.value);
  }

  function onKeydown(event: KeyboardEvent) {
    if (!promptState.isOpen) return;

    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      handleCancel();
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      handleConfirm();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if promptState.isOpen}
  <div
    class="fixed inset-0 z-[110] bg-black/60 backdrop-blur-sm flex items-start justify-center pt-10"
    role="presentation"
    onmousedown={(event) => {
      if (event.target === event.currentTarget) {
        handleCancel();
      }
    }}
  >
    <div
      class="bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl overflow-hidden animate-in fade-in slide-in-from-top-4 duration-200 flex flex-col"
      style="width: 480px; max-width: 90vw;"
      role="dialog"
      aria-label={promptState.options.title}
    >
      <div class="px-4 py-3 border-b border-[#30363d] bg-[#0d1117] flex justify-between items-center shrink-0">
        <h3 class="text-sm font-semibold text-white">{promptState.options.title}</h3>
        <button
          onclick={handleCancel}
          class="text-[#8b949e] hover:text-[#c9d1d9] p-1 rounded hover:bg-[#30363d] transition-colors"
          aria-label="Close"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <div class="p-4 bg-[#161b22] flex flex-col gap-4">
        <div class="text-xs text-[#c9d1d9] leading-relaxed">
          {#if promptState.options.isHtmlMessage}
            {@html promptState.options.message}
          {:else}
            {promptState.options.message}
          {/if}
        </div>

        <input
          type="text"
          bind:this={inputEl}
          bind:value={promptState.value}
          placeholder={promptState.options.placeholder}
          class="w-full px-3 py-1.5 text-xs text-[#c9d1d9] bg-[#0d1117] border border-[#30363d] rounded-md focus:border-[#388bfd] focus:outline-none focus:ring-1 focus:ring-[#388bfd] transition-colors placeholder-[#484f58]"
        />
      </div>

      <div class="px-4 py-3 border-t border-[#30363d] bg-[#0d1117] flex justify-end gap-2">
        <button
          class="px-3 py-1.5 text-xs font-medium text-[#c9d1d9] hover:text-white bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded transition-colors"
          onclick={handleCancel}
        >
          {promptState.options.cancelLabel}
        </button>
        <button
          class="px-3 py-1.5 text-xs font-medium text-white bg-[#238636] hover:bg-[#2ea043] rounded border border-[rgba(240,246,252,0.1)] shadow-sm transition-colors focus:ring-2 focus:ring-[#238636] focus:outline-none"
          onclick={handleConfirm}
        >
          {promptState.options.confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}
