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

<div class="border-t border-[#30363d] bg-[linear-gradient(180deg,#1a202d,#151a23)] px-3 pt-2.5 pb-3 flex flex-col gap-2.5">
  <div class="flex items-center gap-2 text-[13px] text-[#dce7f8] font-semibold">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="3"></circle>
      <line x1="2" y1="12" x2="7" y2="12"></line>
      <line x1="17" y1="12" x2="22" y2="12"></line>
    </svg>
    <span>Commit</span>
  </div>

  <div class="rounded-md border border-[#2d3e57] bg-[#101722] p-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
    <div class="flex items-center gap-2">
      <input
        value={summary}
        oninput={handleSummaryInput}
        placeholder="Commit summary"
        class="flex-1 bg-transparent text-[13px] text-[#d7e3f3] placeholder-[#7f93ac] outline-none"
        disabled={busy || generating}
      />
      <span class="text-xs {summaryTooLong ? 'text-[#f85149]' : 'text-[#8f79d9]'}">{summary.length}</span>
      <button
        type="button"
        class="h-7 w-7 rounded-md border border-[#6340c1] bg-[linear-gradient(135deg,#301f4d,#233a5c)] text-[#d8c9ff] hover:text-white hover:bg-[linear-gradient(135deg,#442971,#2f4d79)] disabled:opacity-45 disabled:cursor-not-allowed inline-flex items-center justify-center transition-colors"
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
      class="mt-2 w-full bg-transparent text-[12px] leading-relaxed text-[#c7d6ea] placeholder-[#788ca7] outline-none resize-none h-10"
      placeholder="Description"
      disabled={busy || generating}
    ></textarea>
  </div>

  <div class="flex items-center justify-between gap-2">
    <button
      type="button"
      class="inline-flex items-center gap-1.5 text-xs text-[#96abc7] hover:text-[#d1e1f6] transition-colors"
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
      class="h-7 px-2.5 rounded border border-[#6e41db] bg-[linear-gradient(135deg,#3d1f7b,#4f30aa)] text-[#ede4ff] text-[11px] font-semibold hover:bg-[linear-gradient(135deg,#4b2394,#613ec5)] disabled:opacity-45 disabled:cursor-not-allowed transition-colors inline-flex items-center gap-1.5"
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
    <div class="rounded-md border border-[#2a3f5f] bg-[#111b2a] p-2 flex flex-col gap-1.5">
      <label class="inline-flex items-center gap-2 text-xs text-[#a8c2e2] cursor-pointer select-none">
        <input
          type="checkbox"
          bind:checked={pushAfterCommit}
          disabled={busy || generating}
          class="rounded border-[#30363d] bg-[#0d1117] text-[#238636] focus:ring-0 focus:ring-offset-0"
        />
        <span>Commit & Push</span>
      </label>
      <label class="inline-flex items-center gap-2 text-xs text-[#6f81a0] cursor-not-allowed select-none">
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
      class="w-full h-8 rounded-sm bg-[linear-gradient(135deg,#1f754d,#229b61)] text-[#e8fff3] text-xs font-semibold hover:bg-[linear-gradient(135deg,#24865a,#2abd76)] disabled:opacity-45 disabled:cursor-not-allowed transition-colors border border-[#38b774] shadow-sm flex items-center justify-center gap-2"
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
