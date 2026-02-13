<script lang="ts">
    /**
     * ResizablePanel - Horizontal resizer for main layout panels
     * Wraps content and provides a drag handle for width adjustment.
     */

    interface Props {
        initialSize?: number;
        minSize?: number;
        maxSize?: number;
        side?: 'left' | 'right';
        children?: import('svelte').Snippet;
    }

    let {
        initialSize = 288,
        minSize = 200,
        maxSize = 600,
        side = 'right',
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
            let newSize: number;

            if (side === 'right') {
                // Handle on right side: width = pointer position relative to container left
                newSize = e.clientX - containerRect.left;
            } else {
                // Handle on left side: width = container right - pointer position
                newSize = containerRect.right - e.clientX;
            }

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
    class="relative flex shrink-0 max-w-full"
    style="width: {size}px;"
>
    <!-- Content slot -->
    <div class="flex-1 min-w-0 overflow-hidden">
        {#if children}
            {@render children()}
        {/if}
    </div>

    <!-- Resize handle -->
    <div
        class="absolute top-0 bottom-0 w-1 cursor-col-resize z-20 group
               {side === 'right' ? 'right-0 translate-x-1/2' : 'left-0 -translate-x-1/2'}"
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        role="separator"
        aria-orientation="vertical"
        tabindex="-1"
    >
        <!-- Visible handle bar on hover/drag -->
        <div
            class="absolute inset-y-0 w-1 transition-colors duration-150
                   {isDragging ? 'bg-[#58a6ff]' : 'bg-transparent group-hover:bg-[#30363d]'}"
        ></div>
    </div>
</div>

<style>
    /* Prevent text selection during drag */
    :global(body.resizing) {
        user-select: none !important;
        cursor: col-resize !important;
    }
</style>
