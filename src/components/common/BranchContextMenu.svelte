<script lang="ts">
  import type {
    BranchContextActionHandler,
    BranchContextMenuAction,
    BranchContextMenuState,
  } from "./branch-context-menu-types";

  interface Props {
    menu: BranchContextMenuState | null;
    onClose: () => void;
    onAction: BranchContextActionHandler;
  }

  let { menu = null, onClose, onAction }: Props = $props();

  type SubmenuKind = "reset";

  const MENU_ITEM_CLASS =
    "w-full text-left px-4 py-2 text-xs text-[#c9d1d9] hover:bg-[#1f6feb] hover:text-white transition-colors";
  const MENU_ITEM_FLEX_CLASS = `${MENU_ITEM_CLASS} flex items-center justify-between gap-3`;
  const MENU_ITEM_DANGER_CLASS =
    "w-full text-left px-4 py-2 text-xs text-[#f85149] hover:bg-[#3b1f2c] hover:text-[#ff7b72] transition-colors";
  const SUBMENU_GAP = 4;
  const RESET_SUBMENU_WIDTH = 240;
  const MENU_ITEM_HEIGHT = 32;
  const MENU_PADDING = 4;

  let menuEl: HTMLDivElement | null = $state(null);
  let menuX = $state(0);
  let menuY = $state(0);
  let openSubmenu = $state<SubmenuKind | null>(null);
  let submenuX = $state(0);
  let submenuY = $state(0);

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
    const height = MENU_ITEM_HEIGHT * 3 + MENU_PADDING * 2;

    submenuX = clamp(rect.right + SUBMENU_GAP, 8, window.innerWidth - RESET_SUBMENU_WIDTH - 8);
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

  async function trigger(action: BranchContextMenuAction) {
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
    <!-- Merge / Rebase -->
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "merge" })}
    >
      Merge {menu.branchName} into {menu.currentBranch || "current"}
    </button>
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "rebase" })}
    >
      Rebase {menu.currentBranch || "current"} onto {menu.branchName}
    </button>
    <div class="border-t border-[#30363d] my-1"></div>

    <!-- Checkout -->
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "checkout" })}
    >
      Checkout {menu.branchName}
    </button>
    <div class="border-t border-[#30363d] my-1"></div>

    <!-- Create branch / Cherry pick / Reset / Revert -->
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
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "cherry-pick" })}
    >
      Cherry pick commit
    </button>
    <button
      type="button"
      class={MENU_ITEM_FLEX_CLASS}
      onclick={(event) => handleOpenSubmenu(event, "reset")}
    >
      <span>{resetLabel}</span>
      <span class="text-[#8b949e]">{openSubmenu === "reset" ? "▾" : "▸"}</span>
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

    <!-- Delete -->
    <button
      type="button"
      class={MENU_ITEM_DANGER_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "delete" })}
    >
      Delete {menu.branchName}
    </button>
    <div class="border-t border-[#30363d] my-1"></div>

    <!-- Copy -->
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "copy-branch-name" })}
    >
      Copy branch name
    </button>
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onmouseenter={closeSubmenu}
      onclick={() => void trigger({ type: "copy-commit-sha" })}
    >
      Copy commit sha
    </button>
    <div class="border-t border-[#30363d] my-1"></div>

    <!-- Tags -->
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

  <!-- Reset Submenu -->
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
