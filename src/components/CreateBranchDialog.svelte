<script lang="ts">
  import { getCreateBranchDialogState, closeCreateBranchDialog } from "../lib/create-branch-dialog.svelte";
  import { GitService } from "../lib/GitService";

  const dialogState = getCreateBranchDialogState();

  let branchName = $state("");
  let baseBranch = $state("");
  let isCreating = $state(false);
  let serverError = $state("");
  let nameInput: HTMLInputElement | undefined = $state();

  // Reset form when dialog opens
  $effect(() => {
    if (dialogState.isOpen) {
      branchName = "";
      baseBranch = dialogState.options.currentBranch;
      isCreating = false;
      serverError = "";
      // Focus the input after DOM update
      requestAnimationFrame(() => {
        nameInput?.focus();
      });
    }
  });

  // Live validation
  let validationError = $derived.by<string>(() => {
    const name = branchName.trim();
    if (name.length === 0) return ""; // Empty = no error shown, but button disabled

    if (name.includes(" ")) return "Branch name cannot contain spaces";
    if (name.includes("..")) return "Branch name cannot contain '..'";
    if (/[~^:?*\[\\]/.test(name)) return "Branch name contains invalid character (~, ^, :, ?, *, [, or \\)";
    if (name.startsWith("/") || name.endsWith("/")) return "Branch name cannot start or end with '/'";
    if (name.endsWith(".")) return "Branch name cannot end with '.'";
    if (name.startsWith("-")) return "Branch name cannot start with '-'";
    if (name.endsWith(".lock")) return "Branch name cannot end with '.lock'";
    if (name.includes("//")) return "Branch name cannot contain consecutive slashes '//'";
    if (name.includes("@{")) return "Branch name cannot contain '@{'";
    // eslint-disable-next-line no-control-regex
    if (/[\x00-\x1f\x7f]/.test(name)) return "Branch name cannot contain control characters";

    // Duplicate check
    if (dialogState.options.branches.includes(name)) return `Branch '${name}' already exists`;

    return "";
  });

  let canCreate = $derived(branchName.trim().length > 0 && !validationError && !isCreating);

  async function handleCreate() {
    if (!canCreate) return;
    isCreating = true;
    serverError = "";
    try {
      const res = await GitService.createBranch(branchName.trim(), baseBranch, dialogState.options.repoPath || undefined);
      if (res.success) {
        window.dispatchEvent(new CustomEvent("branch-created"));
        closeCreateBranchDialog();
      } else {
        serverError = res.stderr || "Failed to create branch";
      }
    } catch (e: any) {
      serverError = String(e);
    } finally {
      isCreating = false;
    }
  }

  function handleCancel() {
    closeCreateBranchDialog();
  }

  function onKeydown(e: KeyboardEvent) {
    // Global Ctrl+B shortcut
    if (e.ctrlKey && e.key === "b") {
      e.preventDefault();
      e.stopPropagation();
      if (dialogState.isOpen) {
        // Toggle close
        closeCreateBranchDialog();
      } else {
        // Dispatch event for BranchExplorer to handle (it has the branch data)
        const target = e.target as HTMLElement;
        const tag = target?.tagName;
        if (tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "SELECT") {
          window.dispatchEvent(new CustomEvent("open-create-branch"));
        }
      }
      return;
    }

    if (!dialogState.isOpen) return;

    if (e.key === "Escape") {
      e.stopPropagation();
      handleCancel();
    }
    if (e.key === "Enter" && canCreate) {
      e.preventDefault();
      e.stopPropagation();
      handleCreate();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if dialogState.isOpen}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-[100] bg-black/60 backdrop-blur-sm flex items-start justify-center pt-10"
    role="presentation"
    onmousedown={(e) => { if (e.target === e.currentTarget) handleCancel(); }}
  >
    <!-- Modal Container -->
    <div
      class="bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl overflow-hidden animate-in fade-in slide-in-from-top-4 duration-200 flex flex-col"
      style="width: 480px; max-width: 90vw;"
      role="dialog"
      aria-label="Create Branch"
    >
      <!-- Header -->
      <div class="px-4 py-3 border-b border-[#30363d] bg-[#0d1117] flex justify-between items-center shrink-0">
        <h3 class="text-sm font-semibold text-white">Create Branch</h3>
        <button
          onclick={handleCancel}
          class="text-[#8b949e] hover:text-[#c9d1d9] p-1 rounded hover:bg-[#30363d] transition-colors"
          aria-label="Close"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <!-- Form Body -->
      <div class="p-4 bg-[#161b22] flex flex-col gap-4">
        <!-- Base Branch -->
        <div class="flex flex-col gap-1.5">
          <label for="base-branch" class="text-xs font-medium text-[#8b949e]">Base branch</label>
          <select
            id="base-branch"
            bind:value={baseBranch}
            class="w-full px-3 py-1.5 text-xs text-[#c9d1d9] bg-[#0d1117] border border-[#30363d] rounded-md focus:border-[#388bfd] focus:outline-none focus:ring-1 focus:ring-[#388bfd] transition-colors"
          >
            {#each dialogState.options.branches as branch}
              <option value={branch}>{branch}</option>
            {/each}
          </select>
        </div>

        <!-- New Branch Name -->
        <div class="flex flex-col gap-1.5">
          <label for="branch-name" class="text-xs font-medium text-[#8b949e]">New branch name</label>
          <input
            id="branch-name"
            type="text"
            bind:this={nameInput}
            bind:value={branchName}
            placeholder="feature/my-branch"
            class="w-full px-3 py-1.5 text-xs text-[#c9d1d9] bg-[#0d1117] rounded-md focus:outline-none focus:ring-1 transition-colors placeholder-[#484f58] {validationError
              ? 'border border-[#f85149] focus:border-[#f85149] focus:ring-[#f85149]'
              : 'border border-[#30363d] focus:border-[#388bfd] focus:ring-[#388bfd]'}"
          />
          {#if validationError}
            <span class="text-[10px] text-[#f85149]">{validationError}</span>
          {/if}
        </div>

        <!-- Server Error -->
        {#if serverError}
          <div class="px-3 py-2 text-xs text-[#f85149] bg-[#f85149]/10 border border-[#f85149]/30 rounded-md">
            {serverError}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="px-4 py-3 border-t border-[#30363d] bg-[#0d1117] flex justify-end gap-3">
        <button
          class="px-4 py-1.5 text-xs font-medium text-[#c9d1d9] hover:text-white bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded-md transition-colors"
          onclick={handleCancel}
        >
          Cancel
        </button>
        <button
          class="px-4 py-1.5 text-xs font-medium text-white bg-[#238636] hover:bg-[#2ea043] rounded-md border border-[rgba(240,246,252,0.1)] shadow-sm transition-colors focus:ring-2 focus:ring-[#238636] focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-[#238636]"
          onclick={handleCreate}
          disabled={!canCreate}
        >
          {#if isCreating}
            Creating...
          {:else}
            Create Branch
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}
