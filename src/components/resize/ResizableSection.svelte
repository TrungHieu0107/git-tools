<script lang="ts">
    /**
     * ResizableSection - Vertical resizer for panel sections
     * Wraps content and provides a drag handle for height adjustment.
     */

    interface Props {
        initialSize?: number;
        minSize?: number;
        maxSize?: number;
        children?: import('svelte').Snippet;
    }

    let {
        initialSize = 200,
        minSize = 80,
        maxSize = 800,
        children
    }: Props = $props();

    let size = $state(initialSize);
    let isDragging = $state(false);
    let containerEl = $state<HTMLDivElement | null>(null);

    function handlePointerDown(e: PointerEvent) {
        e.preventDefault();
        isDragging = true;
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
    }

    function handlePointerMove(e: PointerEvent) {
        if (!isDragging || !containerEl) return;

        // Use requestAnimationFrame to prevent layout thrashing
        requestAnimationFrame(() => {
            const containerRect = containerEl.getBoundingClientRect();
            // Height = pointer position relative to container top
            const newSize = e.clientY - containerRect.top;

            // Clamp to min/max
            size = Math.max(minSize, Math.min(maxSize, newSize));
        });
    }

    function handlePointerUp(e: PointerEvent) {
        isDragging = false;
        (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    }
</script>

<div
    bind:this={containerEl}
    class="relative flex flex-col shrink-0"
    style="height: {size}px;"
>
    <!-- Content slot -->
    <div class="flex-1 min-h-0 overflow-hidden">
        {#if children}
            {@render children()}
        {/if}
    </div>

    <!-- Resize handle at bottom -->
    <div
        class="absolute left-0 right-0 bottom-0 h-1 cursor-row-resize z-20 group translate-y-1/2"
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        role="separator"
        aria-orientation="horizontal"
        tabindex="-1"
    >
        <!-- Visible handle bar on hover/drag -->
        <div
            class="absolute inset-x-0 h-1 transition-colors duration-150
                   {isDragging ? 'bg-[#58a6ff]' : 'bg-transparent group-hover:bg-[#30363d]'}"
        ></div>
    </div>
</div>

<style>
    /* Prevent text selection during drag */
    :global(body.resizing-v) {
        user-select: none !important;
        cursor: row-resize !important;
    }
</style>
