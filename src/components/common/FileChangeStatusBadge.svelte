<script lang="ts">
  import { getFileChangeMeta } from "../../lib/file-change";

  interface Props {
    status: string;
    compact?: boolean;
    showCode?: boolean;
    className?: string;
  }

  let { status, compact = false, showCode = true, className = "" }: Props = $props();

  let meta = $derived(getFileChangeMeta(status));
</script>

<span
  class={`inline-flex items-center gap-1 rounded-md border font-medium ${compact ? "px-1 py-0.5 text-[10px]" : "px-1.5 py-0.5 text-[10px]"} ${meta.textClass} ${meta.bgClass} ${meta.borderClass} ${className}`}
  title={`${meta.label} (${meta.code})`}
>
  {#if meta.kind === "added" || meta.kind === "untracked"}
    <svg class="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M12 5v14M5 12h14"></path>
    </svg>
  {:else if meta.kind === "modified" || meta.kind === "unknown"}
    <svg class="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M12 20h9"></path>
      <path d="M16.5 3.5a2.1 2.1 0 1 1 3 3L7 19l-4 1 1-4Z"></path>
    </svg>
  {:else if meta.kind === "deleted"}
    <svg class="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M3 6h18"></path>
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"></path>
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
    </svg>
  {:else if meta.kind === "renamed" || meta.kind === "copied"}
    <svg class="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M16 3h5v5"></path>
      <path d="m4 20 17-17"></path>
      <path d="M21 16v5h-5"></path>
      <path d="M15 21 4 10"></path>
    </svg>
  {:else if meta.kind === "type"}
    <svg class="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M4 7h16"></path>
      <path d="M4 12h16"></path>
      <path d="M4 17h10"></path>
    </svg>
  {:else if meta.kind === "conflict"}
    <svg class="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M12 2 2 20h20L12 2Z"></path>
      <path d="M12 9v4"></path>
      <path d="M12 17h.01"></path>
    </svg>
  {/if}

  {#if compact}
    {#if showCode}
      <span class="font-mono leading-none">{meta.code}</span>
    {/if}
  {:else}
    <span class="leading-none">{meta.label}</span>
  {/if}
</span>
