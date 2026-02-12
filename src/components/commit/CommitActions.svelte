<script lang="ts">
  interface Props {
    stagedCount: number;
    busy: boolean;
    generating: boolean;
    onCommit: (message: string, push: boolean) => Promise<void> | void;
    onGenerate: () => Promise<void> | void;
    message?: string;
  }

  let {
    stagedCount,
    busy,
    generating,
    onCommit,
    onGenerate,
    message = $bindable(""),
  }: Props = $props();

  let pushAfterCommit = $state(false);
  let showOptions = $state(false);
  let amendPrevious = $state(false);
  let summary = $state("");
  let description = $state("");

  function composeMessage(summaryText: string, descriptionText: string): string {
    const head = summaryText.trim();
    const body = descriptionText.trim();
    if (!head) return body;
    return body ? `${head}\n\n${body}` : head;
  }

  function parseMessage(rawMessage: string) {
    const normalized = rawMessage.replace(/\r\n/g, "\n");
    const [head = "", ...rest] = normalized.split("\n");
    summary = head;
    description = rest.join("\n").replace(/^\n+/, "");
  }

  function syncMessageFromFields() {
    const next = composeMessage(summary, description);
    if (next !== message) {
      message = next;
    }
  }

  function handleSummaryInput(event: Event) {
    summary = (event.currentTarget as HTMLInputElement).value;
    syncMessageFromFields();
  }

  function handleDescriptionInput(event: Event) {
    description = (event.currentTarget as HTMLTextAreaElement).value;
    syncMessageFromFields();
  }

  $effect(() => {
    const composed = composeMessage(summary, description);
    if ((message ?? "") !== composed) {
      parseMessage(message ?? "");
    }
  });

  let summaryTooLong = $derived(summary.length > 72);
  let canGenerate = $derived(stagedCount > 0 && !busy && !generating);
  let canCommit = $derived(stagedCount > 0 && !!summary.trim() && !busy && !generating);

  async function handleCommit() {
    if (!canCommit) return;
    await onCommit(message, pushAfterCommit);
    message = "";
    summary = "";
    description = "";
  }

  async function handleGenerate() {
    if (!canGenerate) return;
    await onGenerate();
  }
</script>

<div class="border-t border-[#30363d] bg-[#161b22] px-3 pt-2.5 pb-3 flex flex-col gap-2.5">
  <div class="flex items-center gap-2 text-[13px] text-[#e6edf3] font-semibold">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="3"></circle>
      <line x1="2" y1="12" x2="7" y2="12"></line>
      <line x1="17" y1="12" x2="22" y2="12"></line>
    </svg>
    <span>Commit</span>
  </div>

  <div class="rounded-md border border-[#30363d] bg-[#0f141d] p-2.5">
    <div class="flex items-center gap-2">
      <input
        value={summary}
        oninput={handleSummaryInput}
        placeholder="Commit summary"
        class="flex-1 bg-transparent text-[13px] text-[#d0d7de] placeholder-[#7d8590] outline-none"
        disabled={busy || generating}
      />
      <span class="text-xs {summaryTooLong ? 'text-[#f85149]' : 'text-[#8b949e]'}">{summary.length}</span>
      <button
        type="button"
        class="h-7 w-7 rounded-md border border-[#2f3d55] bg-[#1a2232] text-[#9fb3c8] hover:text-[#d0d7de] hover:bg-[#23314a] disabled:opacity-45 disabled:cursor-not-allowed inline-flex items-center justify-center transition-colors"
        disabled={!canGenerate}
        onclick={handleGenerate}
        title="Generate commit message"
        aria-label="Generate commit message"
      >
        {#if generating}
          <svg class="animate-spin h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 3l1.8 4.2L18 9l-4.2 1.8L12 15l-1.8-4.2L6 9l4.2-1.8L12 3z"></path>
            <path d="M19 14l.9 2.1L22 17l-2.1.9L19 20l-.9-2.1L16 17l2.1-.9L19 14z"></path>
          </svg>
        {/if}
      </button>
    </div>

    <textarea
      value={description}
      oninput={handleDescriptionInput}
      class="mt-2 w-full bg-transparent text-[12px] leading-relaxed text-[#c9d1d9] placeholder-[#7d8590] outline-none resize-none h-10"
      placeholder="Description"
      disabled={busy || generating}
    ></textarea>
  </div>

  <div class="flex items-center justify-between gap-2">
    <button
      type="button"
      class="inline-flex items-center gap-1.5 text-xs text-[#8b949e] hover:text-[#c9d1d9] transition-colors"
      onclick={() => (showOptions = !showOptions)}
      aria-expanded={showOptions}
    >
      <svg
        width="11"
        height="11"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        class="transition-transform {showOptions ? 'rotate-90' : ''}"
      >
        <path d="M9 6l6 6-6 6"></path>
      </svg>
      Commit options
    </button>

    <button
      type="button"
      class="h-7 px-2.5 rounded border border-[#2f3d55] bg-[#1a2232] text-[#c5d3e8] text-[11px] font-semibold hover:bg-[#23314a] disabled:opacity-45 disabled:cursor-not-allowed transition-colors inline-flex items-center gap-1.5"
      disabled={!canGenerate}
      onclick={handleGenerate}
    >
      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 3l1.8 4.2L18 9l-4.2 1.8L12 15l-1.8-4.2L6 9l4.2-1.8L12 3z"></path>
      </svg>
      Compose with AI
    </button>
  </div>

  {#if showOptions}
    <div class="rounded-md border border-[#30363d] bg-[#111722] p-2 flex flex-col gap-1.5">
      <label class="inline-flex items-center gap-2 text-xs text-[#8b949e] cursor-pointer select-none">
        <input
          type="checkbox"
          bind:checked={pushAfterCommit}
          disabled={busy || generating}
          class="rounded border-[#30363d] bg-[#0d1117] text-[#238636] focus:ring-0 focus:ring-offset-0"
        />
        <span>Commit & Push</span>
      </label>
      <label class="inline-flex items-center gap-2 text-xs text-[#6e7681] cursor-not-allowed select-none">
        <input
          type="checkbox"
          bind:checked={amendPrevious}
          disabled={true}
          class="rounded border-[#30363d] bg-[#0d1117] text-[#238636] focus:ring-0 focus:ring-offset-0"
        />
        <span>Amend previous commit (soon)</span>
      </label>
    </div>
  {/if}

  <div class="pt-1 px-1 pb-1">
    <button
      class="w-full h-8 rounded-sm bg-[#1d3a2b] text-[#b3d7c2] text-xs font-semibold hover:bg-[#245039] disabled:opacity-45 disabled:cursor-not-allowed transition-colors border border-[#2f6f4f] shadow-sm flex items-center justify-center gap-2"
      disabled={!canCommit}
      onclick={handleCommit}
    >
      {#if busy}
        <span class="flex items-center gap-1.5">
          <svg class="animate-spin h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
          Working...
        </span>
      {:else}
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"></circle>
          <line x1="2" y1="12" x2="7" y2="12"></line>
          <line x1="17" y1="12" x2="22" y2="12"></line>
        </svg>
        {#if stagedCount === 0}
          Stage Changes to Commit
        {:else}
          Commit Changes
        {/if}
      {/if}
    </button>
  </div>
</div>
