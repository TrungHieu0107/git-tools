<script lang="ts">
  import { rebaseStore, type RebaseTodoItem } from "../../lib/rebaseStore";
  import { toast } from "../../lib/toast.svelte";

  let items = $state<RebaseTodoItem[]>([]);
  let draggingIndex = $state<number | null>(null);

  // Sync with store
  $effect(() => {
    if ($rebaseStore.status === "editingTodo") {
      items = [...$rebaseStore.todoItems];
    }
  });

  const actions: RebaseTodoItem["action"][] = ["pick", "reword", "edit", "squash", "fixup", "drop"];

  function handleActionChange(index: number, action: RebaseTodoItem["action"]) {
    items[index].action = action;
    rebaseStore.updateTodo(items);
  }

  function moveUp(index: number) {
    if (index === 0) return;
    const newItems = [...items];
    [newItems[index - 1], newItems[index]] = [newItems[index], newItems[index - 1]];
    items = newItems;
    rebaseStore.updateTodo(items);
  }

  function moveDown(index: number) {
    if (index === items.length - 1) return;
    const newItems = [...items];
    [newItems[index + 1], newItems[index]] = [newItems[index], newItems[index + 1]];
    items = newItems;
    rebaseStore.updateTodo(items);
  }

  async function handleApply() {
    try {
      await rebaseStore.applyInteractive();
    } catch (e: any) {
      toast.error(`Failed to apply rebase: ${e}`);
    }
  }

  function handleCancel() {
    rebaseStore.cancelEditing();
  }

  function handleDragStart(index: number) {
    draggingIndex = index;
  }

  function handleDragOver(event: DragEvent, index: number) {
    event.preventDefault();
    if (draggingIndex === null || draggingIndex === index) return;
    
    const newItems = [...items];
    const item = newItems.splice(draggingIndex, 1)[0];
    newItems.splice(index, 0, item);
    items = newItems;
    draggingIndex = index;
  }

  function handleDragEnd() {
    draggingIndex = null;
    rebaseStore.updateTodo(items);
  }
</script>

{#if $rebaseStore.status === "editingTodo"}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-8">
    <div class="bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl w-full max-w-3xl flex flex-col max-h-[80vh]">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-[#30363d] flex items-center justify-between bg-[#1d2128] rounded-t-lg">
        <div>
          <h2 class="text-lg font-semibold text-[#f0f6fc]">Interactive Rebase</h2>
          <p class="text-xs text-[#8b949e]">Reorder or modify commits onto <span class="text-blue-400 font-mono">{$rebaseStore.baseCommit?.slice(0, 7)}</span></p>
        </div>
        <button 
          class="text-[#8b949e] hover:text-[#f0f6fc] p-1 transition-colors"
          onclick={handleCancel}
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Action Legend / Info -->
      <div class="px-6 py-2 bg-[#0d1117] border-b border-[#30363d] flex gap-4 overflow-x-auto text-[10px]">
        <div class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-blue-500"></span> pick: use commit</div>
        <div class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-yellow-500"></span> squash: meld into previous</div>
        <div class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-red-500"></span> drop: remove commit</div>
        <div class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-green-500"></span> reword: edit message</div>
      </div>

      <!-- Commit List -->
      <div class="flex-1 overflow-y-auto p-4 space-y-2 bg-[#0d1117]">
        {#each items as item, i (item.hash)}
          <div 
            class="group flex items-center gap-3 p-2 rounded-md border border-[#30363d] bg-[#161b22] hover:border-[#8b949e] transition-all cursor-move"
            class:opacity-50={draggingIndex === i}
            draggable="true"
            ondragstart={() => handleDragStart(i)}
            ondragover={(e) => handleDragOver(e, i)}
            ondragend={handleDragEnd}
          >
            <!-- Drag Handle -->
            <div class="text-[#484f58] group-hover:text-[#8b949e]">
              <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 16 16">
                <path d="M10 13a1 1 0 1 1 0-2 1 1 0 0 1 0 2zm0-4a1 1 0 1 1 0-2 1 1 0 0 1 0 2zm0-4a1 1 0 1 1 0-2 1 1 0 0 1 0 2zm-4 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2zm0-4a1 1 0 1 1 0-2 1 1 0 0 1 0 2zm0-4a1 1 0 1 1 0-2 1 1 0 0 1 0 2z" />
              </svg>
            </div>

            <!-- Action Selector -->
            <select 
              class="bg-[#0d1117] border border-[#30363d] text-[#c9d1d9] text-xs rounded px-2 py-1 outline-none focus:border-blue-500 transition-colors"
              value={item.action}
              onchange={(e) => handleActionChange(i, (e.target as HTMLSelectElement).value as any)}
            >
              {#each actions as action}
                <option value={action}>{action}</option>
              {/each}
            </select>

            <!-- Hash -->
            <span class="font-mono text-[10px] text-blue-400 w-16">{item.hash.slice(0, 8)}</span>

            <!-- Message -->
            <span class="flex-1 text-xs text-[#c9d1d9] truncate" title={item.message}>{item.message}</span>

            <!-- Arrows (Fallback if drag fails) -->
            <div class="hidden group-hover:flex items-center gap-1">
              <button 
                class="p-1 hover:bg-[#30363d] rounded text-[#8b949e]" 
                onclick={(e) => { e.stopPropagation(); moveUp(i); }}
                disabled={i === 0}
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path d="M5 15l7-7 7 7" /></svg>
              </button>
              <button 
                class="p-1 hover:bg-[#30363d] rounded text-[#8b949e]" 
                onclick={(e) => { e.stopPropagation(); moveDown(i); }}
                disabled={i === items.length - 1}
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path d="M19 9l-7 7-7-7" /></svg>
              </button>
            </div>
          </div>
        {/each}
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-[#30363d] bg-[#1d2128] rounded-b-lg flex justify-between items-center">
        <div class="text-[10px] text-[#8b949e]">
          {items.length} commits selected
        </div>
        <div class="flex gap-3">
          <button 
            class="px-4 py-1.5 text-xs text-[#c9d1d9] hover:text-[#f0f6fc] transition-colors"
            onclick={handleCancel}
          >
            Cancel
          </button>
          <button 
            class="px-6 py-1.5 text-xs text-white bg-[#238636] hover:bg-[#2ea043] rounded-md font-semibold transition-colors shadow-sm"
            onclick={handleApply}
          >
            Start Rebase
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .cursor-move {
    cursor: grab;
  }
  .cursor-move:active {
    cursor: grabbing;
  }
</style>
