<script lang="ts">
  import type { BranchContextMenuState } from "./branch-context-menu-types";

  interface Props {
    menu: BranchContextMenuState | null;
    onClose: () => void;
    onCheckout: (payload: unknown) => void | Promise<void>;
    onMerge: (payload: unknown) => void | Promise<void>;
    checkoutLabel?: string;
    mergeLabel?: string;
  }

  let {
    menu = null,
    onClose,
    onCheckout,
    onMerge,
    checkoutLabel = "Checkout",
    mergeLabel = "Merge into current"
  }: Props = $props();

  function stopEvent(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
  }

  async function handleCheckoutClick() {
    if (!menu || menu.disableCheckout) return;
    const payload = menu.payload;
    onClose();
    await onCheckout(payload);
  }

  async function handleMergeClick() {
    if (!menu || menu.disableMerge) return;
    const payload = menu.payload;
    onClose();
    await onMerge(payload);
  }
</script>

{#if menu}
  <div
    class="fixed inset-0 z-50 bg-transparent"
    onclick={onClose}
    oncontextmenu={(e) => {
      e.preventDefault();
      onClose();
    }}
    role="presentation"
  ></div>

  <div
    class="fixed z-[60] bg-[#1f2428] border border-[#30363d] rounded-md shadow-xl py-1 min-w-[170px]"
    style="top: {menu.y}px; left: {menu.x}px;"
    role="menu"
    tabindex="-1"
    oncontextmenu={stopEvent}
  >
    <button
      class="w-full text-left px-4 py-2 text-xs text-[#c9d1d9] hover:bg-[#1f6feb] hover:text-white flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
      disabled={menu.disableCheckout}
      onclick={handleCheckoutClick}
    >
      <span>{checkoutLabel}</span>
    </button>

    <button
      class="w-full text-left px-4 py-2 text-xs text-[#c9d1d9] hover:bg-[#1f6feb] hover:text-white flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
      disabled={menu.disableMerge}
      onclick={handleMergeClick}
    >
      <span>{mergeLabel}</span>
    </button>
  </div>
{/if}
