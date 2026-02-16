<script lang="ts">
  interface Props {
    stagedCount: number;
    busy: boolean;
    generating: boolean;
    onCommit: (message: string, push: boolean) => Promise<void> | void;
    onGenerate: () => Promise<void> | void;
    onAbortOperation?: () => Promise<void> | void;
    showAbortOperation?: boolean;
    abortOperationLabel?: string;
    primaryActionLabel?: string;
    message?: string;
  }

  let {
    stagedCount,
    busy,
    generating,
    onCommit,
    onGenerate,
    onAbortOperation,
    showAbortOperation = false,
    abortOperationLabel = "Abort Merge",
    primaryActionLabel,
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
  let canAbortOperation = $derived(showAbortOperation && !busy && !generating);
  let commitButtonLabel = $derived(
    stagedCount === 0
      ? "Stage Changes to Commit"
      : `Commit Changes to ${stagedCount} File${stagedCount === 1 ? "" : "s"}`
  );
  let resolvedPrimaryActionLabel = $derived(primaryActionLabel?.trim() || commitButtonLabel);

  const commitButtonClass =
    "bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 text-white font-medium py-1.5 px-3 rounded-md shadow-sm transition-all active:scale-[0.98] flex items-center justify-center gap-2 border border-[rgba(240,246,252,0.1)] text-xs focus:outline-none";

  const operationPrimaryButtonClass =
    "w-full h-8 px-3 rounded-sm border border-[#2ea043] bg-[#163222] text-[#d0d7de] hover:bg-[#1b3d29] disabled:opacity-45 disabled:cursor-not-allowed transition-colors text-[13px] font-semibold inline-flex items-center justify-center whitespace-nowrap";

  const operationAbortButtonClass =
    "w-full h-8 px-3 rounded-sm border border-[#f85149] bg-[#3a1b23] text-[#f3d7db] hover:bg-[#4a232d] disabled:opacity-45 disabled:cursor-not-allowed transition-colors text-[13px] font-semibold inline-flex items-center justify-center whitespace-nowrap";

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

  async function handleAbortOperation() {
    if (!canAbortOperation || !onAbortOperation) return;
    await onAbortOperation();
  }
</script>

<div class="border-t border-[#30363d] bg-[#1c2128] px-1.5 pt-2 pb-2 flex flex-col gap-2">
  <div class="flex items-center justify-between px-1 text-[13px] text-[#d0d7de] font-semibold">
    <span class="inline-flex items-center gap-1.5">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M3.5 12h4.5M16 12h4.5"></path>
      </svg>
      Commit
    </span>
    <span class="inline-flex items-center gap-2 text-[#8b949e]">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 3v12"></path>
        <path d="m7 10 5 5 5-5"></path>
        <rect x="4" y="18" width="16" height="3" rx="1"></rect>
      </svg>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M7 7a5 5 0 0 1 9.7-1.8A4 4 0 1 1 17 19H7a4 4 0 1 1 0-8"></path>
        <path d="m12 12 0 8"></path>
        <path d="m9 16 3-4 3 4"></path>
      </svg>
    </span>
  </div>

  <label class="inline-flex items-center gap-2 px-1 text-[12px] text-[#c9d1d9] select-none cursor-not-allowed">
    <input
      type="checkbox"
      bind:checked={amendPrevious}
      disabled={true}
      class="h-3.5 w-3.5 rounded-[2px] border border-[#484f58] bg-[#161b22] text-[#58a6ff] focus:ring-0 focus:ring-offset-0 cursor-not-allowed"
    />
    <span>Amend previous commit</span>
  </label>

  <div class="rounded border border-[#30363d] bg-[#161b22] overflow-hidden">
    <div class="flex flex-wrap items-center gap-2 px-2.5 py-2">
      <input
        value={summary}
        oninput={handleSummaryInput}
        placeholder="Commit summary"
        class="flex-1 min-w-[180px] bg-transparent text-[13px] leading-none text-[#c9d1d9] placeholder-[#8b949e] outline-none"
        disabled={busy || generating}
      />
      <span class="text-[13px] leading-none {summaryTooLong ? 'text-[#f85149]' : 'text-[#8b949e]'}">{summary.length}</span>
      <button
        type="button"
        class="h-8 w-8 shrink-0 rounded-md border border-[#3f4b63] bg-[linear-gradient(145deg,#2a2f3d_0%,#1f2735_100%)] text-[#c6d6f7] hover:border-[#5c6f96] hover:bg-[linear-gradient(145deg,#323b52_0%,#263247_100%)] hover:text-[#e5eeff] hover:shadow-[0_6px_14px_rgba(18,32,64,0.45)] disabled:opacity-45 disabled:cursor-not-allowed inline-flex items-center justify-center transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#79c0ff]"
        disabled={!canGenerate}
        onclick={handleGenerate}
        title="Generate commit message"
        aria-label="Generate commit message"
      >
        {#if generating}
          <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
        {:else}
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 3l1.8 4.2L18 9l-4.2 1.8L12 15l-1.8-4.2L6 9l4.2-1.8L12 3z"></path>
            <path d="M19 14l.9 2.1L22 17l-2.1.9L19 20l-.9-2.1L16 17l2.1-.9L19 14z"></path>
          </svg>
        {/if}
      </button>
    </div>

    <textarea
      value={description}
      oninput={handleDescriptionInput}
      class="h-12 w-full px-2.5 py-2 bg-transparent text-[12px] leading-relaxed text-[#c9d1d9] placeholder-[#8b949e] outline-none resize-none"
      placeholder="Description"
      disabled={busy || generating}
    ></textarea>
  </div>

  <div class="flex flex-wrap items-center gap-2">
    <button
      type="button"
      class="inline-flex items-center gap-1.5 h-7 px-1 text-[12px] text-[#8b949e] hover:text-[#c9d1d9] transition-colors focus:outline-none"
      onclick={() => (showOptions = !showOptions)}
      aria-expanded={showOptions}
    >
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        class="transition-transform {showOptions ? 'rotate-90' : ''}"
      >
        <path d="M9 6l6 6-6 6"></path>
      </svg>
      Commit options
    </button>

    <button
      type="button"
      class="ml-auto max-[420px]:ml-0 h-7 px-3 rounded-sm border border-[#5b2ea4] bg-[#261444] text-[#d0c0ff] text-[12px] font-semibold hover:bg-[#2f1755] disabled:opacity-45 disabled:cursor-not-allowed transition-colors inline-flex items-center gap-1.5 focus:outline-none"
      disabled={!canGenerate}
      onclick={handleGenerate}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 3l1.8 4.2L18 9l-4.2 1.8L12 15l-1.8-4.2L6 9l4.2-1.8L12 3z"></path>
      </svg>
      Compose commits with AI
    </button>
  </div>

  {#if showOptions}
    <div class="rounded border border-[#30363d] bg-[#161b22] px-2 py-1.5">
      <label class="inline-flex items-center gap-2 text-xs text-[#c9d1d9] cursor-pointer select-none">
        <input
          type="checkbox"
          bind:checked={pushAfterCommit}
          disabled={busy || generating}
          class="h-3.5 w-3.5 rounded-[2px] border border-[#484f58] bg-[#0d1117] text-[#58a6ff] focus:ring-0 focus:ring-offset-0"
        />
        <span>Commit & Push</span>
      </label>
    </div>
  {/if}

  <div class={showAbortOperation ? "grid grid-cols-2 gap-2" : "flex items-stretch gap-2"}>
    <button
      class={`${showAbortOperation ? operationPrimaryButtonClass : `${commitButtonClass} text-center break-words w-full`}`}
      disabled={!canCommit}
      onclick={handleCommit}
    >
      {#if busy}
        <span class="flex items-center gap-1.5">
          <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
          Working...
        </span>
      {:else}
        {#if showAbortOperation}
          {resolvedPrimaryActionLabel}
        {:else}
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <circle cx="12" cy="12" r="3"></circle>
            <line x1="2" y1="12" x2="7" y2="12"></line>
            <line x1="17" y1="12" x2="22" y2="12"></line>
          </svg>
          {commitButtonLabel}
        {/if}
      {/if}
    </button>

    {#if showAbortOperation}
      <button
        type="button"
        class={operationAbortButtonClass}
        disabled={!canAbortOperation}
        onclick={handleAbortOperation}
      >
        {abortOperationLabel}
      </button>
    {/if}
  </div>
</div>
