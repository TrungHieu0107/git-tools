<script lang="ts">
  import type {
    CommitContextActionHandler,
    CommitContextMenuAction,
    CommitContextMenuState,
  } from "./commit-context-menu-types";

  interface Props {
    menu: CommitContextMenuState | null;
    onClose: () => void;
    onAction: CommitContextActionHandler;
  }

  let { menu = null, onClose, onAction }: Props = $props();

  type SubmenuKind = "checkout" | "reset";

  const MENU_PADDING = 4;
  const MENU_ITEM_HEIGHT = 32;
  const MENU_SEPARATOR_HEIGHT = 8;
  const SUBMENU_GAP = 4;
  const CHECKOUT_SUBMENU_WIDTH = 270;
  const RESET_SUBMENU_WIDTH = 240;

  const MENU_ITEM_CLASS =
    "w-full text-left px-4 py-2 text-xs text-[#c9d1d9] hover:bg-[#1f6feb] hover:text-white transition-colors";
  const MENU_ITEM_FLEX_CLASS = `${MENU_ITEM_CLASS} flex items-center justify-between gap-3`;
  const MENU_ITEM_DANGER_CLASS =
    "w-full text-left px-4 py-2 text-xs text-[#f85149] hover:bg-[#3b1f2c] hover:text-[#ff7b72] transition-colors";

  let menuEl: HTMLDivElement | null = $state(null);
  let menuX = $state(0);
  let menuY = $state(0);
  let openSubmenu = $state<SubmenuKind | null>(null);
  let submenuX = $state(0);
  let submenuY = $state(0);

  let primaryLocalBranch = $derived.by(() => {
    if (!menu) return "";
    return menu.localBranches[0] ?? "";
  });

  let deletableLocalBranch = $derived.by(() => {
    if (!menu) return "";
    return menu.localBranches.find((branch) => branch !== menu.currentBranch) ?? "";
  });

  let preferredRemoteRef = $derived.by(() => {
    if (!menu || menu.remoteBranches.length === 0) return "";
    if (primaryLocalBranch) {
      const match = findRemoteForLocalBranch(primaryLocalBranch, menu.remoteBranches);
      if (match) return match;
    }
    return menu.remoteBranches[0] ?? "";
  });

  let localAndRemotePair = $derived.by(() => {
    if (!menu || !primaryLocalBranch) return null;
    const remoteRef = findRemoteForLocalBranch(primaryLocalBranch, menu.remoteBranches);
    if (!remoteRef) return null;
    return { branch: primaryLocalBranch, remoteRef };
  });

  let showPullPushGroup = $derived.by(() => !!menu?.isHead);
  let showCheckoutGroup = $derived.by(() => !!menu && !menu.isHead);
  let showSetUpstream = $derived.by(() => !!menu?.isHead && !!menu.currentBranch);
  let showBranchManagementGroup = $derived.by(
    () => !!primaryLocalBranch || !!deletableLocalBranch || !!preferredRemoteRef,
  );

  let resetLabel = $derived.by(() => {
    const branchName = menu?.currentBranch?.trim() || "HEAD";
    return `Reset ${branchName} to this commit`;
  });

  $effect(() => {
    if (!menu) {
      openSubmenu = null;
      return;
    }

    openSubmenu = null;
    menuX = menu.x;
    menuY = menu.y;

    requestAnimationFrame(() => {
      const rect = menuEl?.getBoundingClientRect();
      if (!rect) return;
      menuX = clamp(menu.x, 8, window.innerWidth - rect.width - 8);
      menuY = clamp(menu.y, 8, window.innerHeight - rect.height - 8);
    });
  });

  function clamp(value: number, min: number, max: number): number {
    if (max < min) return min;
    return Math.min(Math.max(value, min), max);
  }

  function getRemoteBranchName(remoteRef: string): string {
    const idx = remoteRef.indexOf("/");
    if (idx < 0 || idx >= remoteRef.length - 1) return remoteRef;
    return remoteRef.slice(idx + 1);
  }

  function findRemoteForLocalBranch(localBranch: string, remoteRefs: string[]): string | null {
    const normalized = localBranch.trim().toLowerCase();
    if (!normalized) return null;

    for (const remoteRef of remoteRefs) {
      const remoteBranch = getRemoteBranchName(remoteRef).trim().toLowerCase();
      if (remoteBranch === normalized) {
        return remoteRef;
      }
    }
    return null;
  }

  function getCheckoutSubmenuHeight(): number {
    if (!menu) return 0;

    const localCount = menu.localBranches.length;
    const remoteCount = menu.remoteBranches.length;
    const actionCount = localCount + remoteCount + 1; // Detached option
    let separatorCount = 0;

    if (localCount > 0 && remoteCount > 0) separatorCount += 1;
    if (localCount > 0 || remoteCount > 0) separatorCount += 1;

    return (
      actionCount * MENU_ITEM_HEIGHT +
      separatorCount * MENU_SEPARATOR_HEIGHT +
      MENU_PADDING * 2
    );
  }

  function getResetSubmenuHeight(): number {
    return MENU_ITEM_HEIGHT * 3 + MENU_PADDING * 2;
  }

  function closeSubmenu() {
    openSubmenu = null;
  }

  function handleOpenSubmenu(event: MouseEvent, kind: SubmenuKind) {
    event.preventDefault();
    event.stopPropagation();

    if (!menu) return;
    if (openSubmenu === kind) {
      openSubmenu = null;
      return;
    }

    const target = event.currentTarget as HTMLElement | null;
    if (!target) return;

    const rect = target.getBoundingClientRect();
    const width = kind === "checkout" ? CHECKOUT_SUBMENU_WIDTH : RESET_SUBMENU_WIDTH;
    const height = kind === "checkout" ? getCheckoutSubmenuHeight() : getResetSubmenuHeight();

    submenuX = clamp(rect.right + SUBMENU_GAP, 8, window.innerWidth - width - 8);
    submenuY = clamp(rect.top, 8, window.innerHeight - height - 8);
    openSubmenu = kind;
  }

  function stopContextMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (!menu) return;
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }

  async function trigger(action: CommitContextMenuAction) {
    if (!menu) return;
    const snapshot = menu;
    closeSubmenu();
    onClose();
    await onAction(action, snapshot);
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if menu}
  <div
    class="fixed inset-0 z-[70] bg-transparent"
    onclick={onClose}
    oncontextmenu={(event) => {
      event.preventDefault();
      onClose();
    }}
    role="presentation"
  ></div>

  <div
    class="fixed z-[80] rounded-md border border-[#30363d] bg-[#161b22] shadow-2xl py-1 min-w-[260px]"
    style={`left: ${menuX}px; top: ${menuY}px;`}
    role="menu"
    tabindex="-1"
    bind:this={menuEl}
    oncontextmenu={stopContextMenu}
  >
    {#if showPullPushGroup}
      <button
        type="button"
        class={MENU_ITEM_CLASS}
        onmouseenter={closeSubmenu}
        onclick={() => void trigger({ type: "pull" })}
      >
        Pull (fast-forward if possible)
      </button>
      <button
        type="button"
        class={MENU_ITEM_CLASS}
        onmouseenter={closeSubmenu}
        onclick={() => void trigger({ type: "push" })}
      >
        Push
      </button>
      <button
        type="button"
        class={MENU_ITEM_CLASS}
        onmouseenter={closeSubmenu}
        onclick={() => void trigger({ type: "fetch" })}
      >
        Fetch
      </button>
      {#if showSetUpstream}
        <button
          type="button"
          class={MENU_ITEM_CLASS}
          onmouseenter={closeSubmenu}
          onclick={() => void trigger({ type: "set-upstream" })}
        >
          Set Upstream
        </button>
      {/if}
      <div class="border-t border-[#30363d] my-1"></div>
    {/if}

    {#if showCheckoutGroup}
      <button
        type="button"
        class={MENU_ITEM_FLEX_CLASS}
        onclick={(event) => handleOpenSubmenu(event, "checkout")}
      >
        <span>Checkout</span>
        <span class="text-[#8b949e]">{openSubmenu === "checkout" ? "v" : ">"}</span>
      </button>
      <div class="border-t border-[#30363d] my-1"></div>
    {/if}

    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "create-branch-here" })}
    >
      Create branch here
    </button>
    <button
      type="button"
      class={MENU_ITEM_FLEX_CLASS}
      onclick={(event) => handleOpenSubmenu(event, "reset")}
    >
      <span>{resetLabel}</span>
      <span class="text-[#8b949e]">{openSubmenu === "reset" ? "v" : ">"}</span>
    </button>
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "rebase" })}
    >
      Rebase {menu.currentBranch || "current"} onto this commit
    </button>
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "interactive-rebase" })}
    >
      Interactive Rebase onto this commit
    </button>
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "revert" })}
    >
      Revert commit
    </button>
    <div class="border-t border-[#30363d] my-1"></div>

    {#if showBranchManagementGroup}
      {#if primaryLocalBranch}
        <button
          type="button"
          class={MENU_ITEM_CLASS}
          onmouseenter={closeSubmenu}
          onclick={() => void trigger({ type: "rename-branch", branch: primaryLocalBranch })}
        >
          Rename {primaryLocalBranch}
        </button>
      {/if}
      {#if deletableLocalBranch}
        <button
          type="button"
          class={MENU_ITEM_DANGER_CLASS}
          onmouseenter={closeSubmenu}
          onclick={() => void trigger({ type: "delete-local-branch", branch: deletableLocalBranch })}
        >
          Delete {deletableLocalBranch}
        </button>
      {/if}
      {#if preferredRemoteRef}
        <button
          type="button"
          class={MENU_ITEM_DANGER_CLASS}
          onmouseenter={closeSubmenu}
          onclick={() => void trigger({ type: "delete-remote-branch", remoteRef: preferredRemoteRef })}
        >
          Delete {preferredRemoteRef}
        </button>
      {/if}
      {#if localAndRemotePair}
        <button
          type="button"
          class={MENU_ITEM_DANGER_CLASS}
          onmouseenter={closeSubmenu}
          onclick={() =>
            void trigger({
              type: "delete-local-and-remote",
              branch: localAndRemotePair.branch,
              remoteRef: localAndRemotePair.remoteRef,
            })}
        >
          Delete {localAndRemotePair.branch} and {localAndRemotePair.remoteRef}
        </button>
      {/if}
      <div class="border-t border-[#30363d] my-1"></div>
    {/if}

    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "copy-commit-sha" })}
    >
      Copy commit sha
    </button>
    {#if primaryLocalBranch}
      <button
        type="button"
        class={MENU_ITEM_CLASS}
        onmouseenter={closeSubmenu}
        onclick={() => void trigger({ type: "copy-branch-name", branch: primaryLocalBranch })}
      >
        Copy branch name
      </button>
    {/if}
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "create-patch-from-commit" })}
    >
      Create patch from commit
    </button>
    <div class="border-t border-[#30363d] my-1"></div>

    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "create-tag", annotated: false })}
    >
      Create tag here
    </button>
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "create-tag", annotated: true })}
    >
      Create annotated tag here
    </button>
  </div>

  {#if openSubmenu === "checkout"}
    <div
      class="fixed z-[90] rounded-md border border-[#30363d] bg-[#161b22] shadow-2xl py-1"
      style={`left: ${submenuX}px; top: ${submenuY}px; width: ${CHECKOUT_SUBMENU_WIDTH}px;`}
      role="menu"
      tabindex="-1"
      oncontextmenu={stopContextMenu}
    >
      {#if menu.localBranches.length > 0}
        {#each menu.localBranches as branch}
          <button
            type="button"
            class={MENU_ITEM_CLASS}
            onclick={() => void trigger({ type: "checkout-local", branch })}
          >
            {branch}
          </button>
        {/each}
      {/if}

      {#if menu.localBranches.length > 0 && menu.remoteBranches.length > 0}
        <div class="border-t border-[#30363d] my-1"></div>
      {/if}

      {#if menu.remoteBranches.length > 0}
        {#each menu.remoteBranches as remoteRef}
          <button
            type="button"
            class={MENU_ITEM_CLASS}
            onclick={() => void trigger({ type: "checkout-remote", remoteRef })}
          >
            Track {remoteRef}
          </button>
        {/each}
      {/if}

      {#if menu.localBranches.length > 0 || menu.remoteBranches.length > 0}
        <div class="border-t border-[#30363d] my-1"></div>
      {/if}

      <button
        type="button"
        class={MENU_ITEM_CLASS}
        onclick={() => void trigger({ type: "checkout-detached" })}
      >
        Detached HEAD at {menu.node.hash.slice(0, 8)}
      </button>
    </div>
  {/if}

  {#if openSubmenu === "reset"}
    <div
      class="fixed z-[90] rounded-md border border-[#30363d] bg-[#161b22] shadow-2xl py-1"
      style={`left: ${submenuX}px; top: ${submenuY}px; width: ${RESET_SUBMENU_WIDTH}px;`}
      role="menu"
      tabindex="-1"
      oncontextmenu={stopContextMenu}
    >
      <button
        type="button"
        class={MENU_ITEM_CLASS}
        onclick={() => void trigger({ type: "reset", mode: "soft" })}
      >
        Soft (--soft)
      </button>
      <button
        type="button"
        class={MENU_ITEM_CLASS}
        onclick={() => void trigger({ type: "reset", mode: "mixed" })}
      >
        Mixed (--mixed)
      </button>
      <button
        type="button"
        class={MENU_ITEM_DANGER_CLASS}
        onclick={() => void trigger({ type: "reset", mode: "hard" })}
      >
        Hard (--hard)
      </button>
    </div>
  {/if}
{/if}
