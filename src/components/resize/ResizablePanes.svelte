<script lang="ts">
    /**
     * ResizablePanes - Horizontal split pane with draggable divider
     * For side-by-side layouts like diff viewer (Base | Modified)
     */

    interface Props {
        initialLeftPercent?: number;
        minLeftPercent?: number;
        maxLeftPercent?: number;
        leftContent?: import('svelte').Snippet;
        rightContent?: import('svelte').Snippet;
    }

    let {
        initialLeftPercent = 50,
        minLeftPercent = 20,
        maxLeftPercent = 80,
        leftContent,
        rightContent
    }: Props = $props();

    let leftPercent = $state(initialLeftPercent);
    let isDragging = $state(false);
    let containerEl = $state<HTMLDivElement | null>(null);

    function handlePointerDown(e: PointerEvent) {
        e.preventDefault();
        isDragging = true;
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
    }

    function handlePointerMove(e: PointerEvent) {
        if (!isDragging || !containerEl) return;

        requestAnimationFrame(() => {
            const rect = containerEl!.getBoundingClientRect();
            const relativeX = e.clientX - rect.left;
            const percent = (relativeX / rect.width) * 100;
            
            // Clamp to min/max
            leftPercent = Math.max(minLeftPercent, Math.min(maxLeftPercent, percent));
        });
    }

    function handlePointerUp(e: PointerEvent) {
        isDragging = false;
        (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    }
</script>

<div
    bind:this={containerEl}
    class="flex flex-1 overflow-hidden min-h-0 relative"
>
    <!-- Left pane -->
    <div 
        class="overflow-hidden"
        style="width: {leftPercent}%;"
    >
        {#if leftContent}
            {@render leftContent()}
        {/if}
    </div>

    <!-- Resize handle -->
    <div
        class="w-1 cursor-col-resize z-20 relative group shrink-0 bg-[#30363d]"
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        role="separator"
        aria-orientation="vertical"
        tabindex="-1"
    >
        <!-- Visible handle highlight on hover/drag -->
        <div
            class="absolute inset-y-0 w-1 transition-colors duration-150
                   {isDragging ? 'bg-[#58a6ff]' : 'bg-[#30363d] group-hover:bg-[#484f58]'}"
        ></div>
    </div>

    <!-- Right pane -->
    <div 
        class="flex-1 overflow-hidden"
    >
        {#if rightContent}
            {@render rightContent()}
        {/if}
    </div>
</div>
