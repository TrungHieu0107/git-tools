<script lang="ts">
  import type {
    StashCommitContextActionHandler,
    StashCommitContextMenuAction,
    StashCommitContextMenuState,
  } from "./stash-commit-context-menu-types";

  interface Props {
    menu: StashCommitContextMenuState | null;
    onClose: () => void;
    onAction: StashCommitContextActionHandler;
  }

  let { menu = null, onClose, onAction }: Props = $props();

  let menuX = $state(0);
  let menuY = $state(0);
  let menuEl: HTMLDivElement | null = $state(null);

  const MENU_ITEM_CLASS =
    "w-full text-left px-4 py-2 text-xs text-[#c9d1d9] hover:bg-[#1f6feb] hover:text-white transition-colors";
  const MENU_ITEM_DANGER_CLASS =
    "w-full text-left px-4 py-2 text-xs text-[#f85149] hover:bg-[#3b1f2c] hover:text-[#ff7b72] transition-colors";

  $effect(() => {
    if (!menu) return;
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

  async function trigger(action: StashCommitContextMenuAction) {
    if (!menu) return;
    const snapshot = menu;
    onClose();
    await onAction(action, snapshot);
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if menu}
  <div
    class="fixed inset-0 z-[85] bg-transparent"
    onclick={onClose}
    oncontextmenu={(event) => {
      event.preventDefault();
      onClose();
    }}
    role="presentation"
  ></div>

  <div
    class="fixed z-[95] rounded-md border border-[#30363d] bg-[#161b22] shadow-2xl py-1 min-w-[220px]"
    style={`left: ${menuX}px; top: ${menuY}px;`}
    role="menu"
    tabindex="-1"
    bind:this={menuEl}
    oncontextmenu={stopContextMenu}
  >
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onclick={() => void trigger({ type: "apply-stash" })}
    >
      Apply Stash
    </button>
    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onclick={() => void trigger({ type: "pop-stash" })}
    >
      Pop Stash
    </button>
    <button
      type="button"
      class={MENU_ITEM_DANGER_CLASS}
      onclick={() => void trigger({ type: "delete-stash" })}
    >
      Delete Stash
    </button>

    <div class="border-t border-[#30363d] my-1"></div>

    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onclick={() => void trigger({ type: "edit-stash-message" })}
    >
      Edit stash message
    </button>

    <div class="border-t border-[#30363d] my-1"></div>

    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onclick={() => void trigger({ type: "share-stash-cloud-patch" })}
    >
      Share stash as Cloud Patch
    </button>

    <div class="border-t border-[#30363d] my-1"></div>

    <button
      type="button"
      class={MENU_ITEM_CLASS}
      onclick={() => void trigger({ type: "hide" })}
    >
      Hide
    </button>
  </div>
{/if}
