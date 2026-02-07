<script lang="ts">
  import type { GraphNode, GraphEdge } from "../lib/graph-layout";
  import { onMount, tick } from "svelte";

  interface Props {
    nodes?: GraphNode[];
    edges?: GraphEdge[];
  }

  let { nodes = [], edges = [] }: Props = $props();

  const ROW_HEIGHT = 28;
  const COL_WIDTH = 20; 
  const DOT_RADIUS = 4;
  const STROKE_WIDTH = 2;
  const PADDING_TOP = 8;
  const PADDING_LEFT = 10;
  
  // -- State -- 
  interface Column {
      id: string;
      label: string;
      width: number;
      visible: boolean;
      minWidth: number;
  }

  let columns = $state<Column[]>([
      { id: "graph", label: "Graph", width: 300, visible: true, minWidth: 100 },
      { id: "hash", label: "Hash", width: 80, visible: true, minWidth: 60 },
      { id: "message", label: "Message", width: 400, visible: true, minWidth: 200 }, // Using numeric width to serve as flex basis concept if we were using flex, but for grid we can treat it as pixels or '1fr' logic if we get fancy. For now, pixel based resizing is robust.
      { id: "author", label: "Author", width: 150, visible: true, minWidth: 80 },
      { id: "date", label: "Date", width: 140, visible: true, minWidth: 100 }
  ]);

  // Derived grid template
  let gridTemplate = $derived(columns.filter(c => c.visible).map(c => c.id === 'message' ? `minmax(${c.width}px, 1fr)` : `${c.width}px`).join(" "));
  let visibleColumns = $derived(columns.filter(c => c.visible));

  // Persistence
  onMount(() => {
      const saved = localStorage.getItem("gh_table_columns");
      if (saved) {
          try {
              const parsed = JSON.parse(saved);
              // Merge saved widths/visibility into default config (to handle potential schema changes or missing cols)
              columns = columns.map(def => {
                  const s = parsed.find((p: Column) => p.id === def.id);
                  return s ? { ...def, width: s.width, visible: s.visible } : def;
              });
          } catch(e) { console.error("Failed to load column settings", e); }
      }
  });

  $effect(() => {
      localStorage.setItem("gh_table_columns", JSON.stringify(columns));
  });


  // -- Resizing Logic --
  let resizingColId = $state<string | null>(null);
  let startX = 0;
  let startWidth = 0;

  function onMouseDown(e: MouseEvent, colId: string) {
      const col = columns.find(c => c.id === colId);
      if (!col) return;
      
      resizingColId = colId;
      startX = e.clientX;
      startWidth = col.width;
      
      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
  }

  function onMouseMove(e: MouseEvent) {
      if (!resizingColId) return;
      
      const idx = columns.findIndex(c => c.id === resizingColId);
      if (idx === -1) return;
      
      const diff = e.clientX - startX;
      let newWidth = Math.max(columns[idx].minWidth, startWidth + diff);
      
      // Mutate grid
      columns[idx].width = newWidth;
  }

  function onMouseUp() {
      resizingColId = null;
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
  }
  
  // -- Visibility Toggle --
  let showMenu = $state(false);

  // Helper to parse refs for badges
  function getBadges(refs: string[]) {
      const badges = [];
      for (const ref of refs) {
          if (ref.includes("HEAD ->")) {
              badges.push({ text: "HEAD", type: "head" });
              const branch = ref.split("HEAD ->")[1].trim();
              if (branch) badges.push({ text: branch, type: "branch" });
          } else if (ref.startsWith("tag:")) {
               badges.push({ text: ref.replace("tag:", "").trim(), type: "tag" });
          } else if (ref.includes("/")) {
               badges.push({ text: ref.trim(), type: "remote" });
          } else {
               badges.push({ text: ref.trim(), type: "branch" });
          }
      }
      return badges;
  }
</script>

<div class="w-full h-full overflow-hidden flex flex-col bg-[#0d1117] font-sans">
  
  <!-- Toolbar / Menu -->
  <div class="h-8 flex items-center px-2 bg-[#161b22] border-b border-[#30363d] relative">
      <button 
        onclick={() => showMenu = !showMenu}
        class="text-xs text-[#8b949e] hover:text-[#c9d1d9] px-2 py-1 rounded hover:bg-[#21262d] flex items-center gap-1 transition-colors"
      >
        <span>Columns ▾</span> 
      </button>

      {#if showMenu}
        <div class="absolute top-8 left-2 bg-[#1f2428] border border-[#30363d] rounded-md shadow-xl z-50 p-2 w-40 animate-in fade-in zoom-in-95 duration-100">
            {#each columns as col}
                <label class="flex items-center gap-2 p-1.5 hover:bg-[#282e33] rounded cursor-pointer text-xs text-[#c9d1d9]">
                    <input type="checkbox" bind:checked={col.visible} class="rounded border-[#30363d] bg-[#0d1117] text-[#238636] focus:ring-0">
                    {col.label}
                </label>
            {/each}
        </div>
        <!-- Backdrop to close -->
        <div class="fixed inset-0 z-40" onclick={() => showMenu = false} role="none"></div>
      {/if}
  </div>

  <!-- Header Row -->
  <!-- We use same grid for header and body -->
  <div 
    class="border-b border-[#30363d] bg-[#161b22] select-none text-xs font-semibold text-[#8b949e] uppercase tracking-wider relative"
    style="display: grid; grid-template-columns: {gridTemplate}; min-width: 100%;"
  >
    {#each visibleColumns as col (col.id)}
      <div 
        class="relative flex items-center px-4 h-8 group border-r border-[#30363d]/30"
      >
        {col.label}
        <!-- Resize Handle -->
        <div 
            role="none"
            class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-[#58a6ff] active:bg-[#58a6ff] z-10 opacity-0 group-hover:opacity-100 transition-opacity"
            onmousedown={(e) => onMouseDown(e, col.id)}
            ondblclick={() => /* reset? */ col.width = col.minWidth + 100 }
        ></div>
      </div>
    {/each}
  </div>

  <div class="flex-1 overflow-auto custom-scrollbar relative">
      <div class="relative min-w-full" style="height: {nodes.length * ROW_HEIGHT + PADDING_TOP}px;">
        
        <!-- Graph SVG Layer -->
        <!-- Locked to the width of the 'graph' column if visible -->
        {#if columns.find(c => c.id === 'graph')?.visible}
            <div class="absolute top-0 left-0 h-full pointer-events-none z-10 overflow-hidden" style="width: {columns.find(c => c.id === 'graph')?.width}px">
                <svg class="w-full h-full"> 
                  <g transform="translate({PADDING_LEFT}, {PADDING_TOP})">
                    <!-- Edges -->
                    {#each edges as edge}
                      {#if edge.type === 'straight'}
                         <line 
                           x1={edge.x1 * COL_WIDTH + COL_WIDTH/2} 
                           y1={edge.y1 * ROW_HEIGHT + ROW_HEIGHT/2} 
                           x2={edge.x2 * COL_WIDTH + COL_WIDTH/2} 
                           y2={edge.y2 * ROW_HEIGHT + ROW_HEIGHT/2}
                           stroke={edge.color}
                           stroke-width={STROKE_WIDTH}
                           stroke-linecap="round"
                         />
                      {:else}
                         <path
                           d="M {edge.x1 * COL_WIDTH + COL_WIDTH/2} {edge.y1 * ROW_HEIGHT + ROW_HEIGHT/2}
                              C {edge.x1 * COL_WIDTH + COL_WIDTH/2} {edge.y2 * ROW_HEIGHT},
                                {edge.x2 * COL_WIDTH + COL_WIDTH/2} {edge.y1 * ROW_HEIGHT + ROW_HEIGHT},
                                {edge.x2 * COL_WIDTH + COL_WIDTH/2} {edge.y2 * ROW_HEIGHT + ROW_HEIGHT/2}"
                           fill="none"
                           stroke={edge.color}
                           stroke-width={STROKE_WIDTH}
                           stroke-linecap="round"
                           opacity="0.8"
                         />
                      {/if}
                    {/each}
                    
                    <!-- Nodes -->
                    {#each nodes as node}
                      <circle 
                        cx={node.x * COL_WIDTH + COL_WIDTH/2} 
                        cy={node.y * ROW_HEIGHT + ROW_HEIGHT/2} 
                        r={DOT_RADIUS} 
                        fill={node.color} 
                        stroke="#0d1117"
                        stroke-width="2"
                      />
                    {/each}
                  </g>
                </svg>
            </div>
        {/if}

        <!-- Rows Container -->
        <div class="absolute top-0 left-0 w-full pt-[8px]">
           {#each nodes as node}
             <div 
                class="border-b border-[#30363d]/20 hover:bg-[#0d1b2a] transition-colors text-xs items-center group"
                style="display: grid; grid-template-columns: {gridTemplate}; height: {ROW_HEIGHT}px;"
             >
               {#each visibleColumns as col}
                 {#if col.id === 'graph'}
                     <div><!-- Placeholder for SVG overlay --></div>
                 {:else if col.id === 'hash'}
                     <div class="pl-4 font-mono text-[#8b949e] opacity-70 truncate">{node.hash}</div>
                 {:else if col.id === 'message'}
                     <div class="pl-4 flex items-center min-w-0 pr-4 overflow-hidden">
                          <!-- Badges inside Message column -->
                          {#each getBadges(node.refs) as badge}
                            <span 
                              class="px-1.5 py-0.5 rounded text-[10px] font-medium mr-2 border shrink-0
                                {badge.type === 'head' ? 'bg-sky-900/40 text-sky-300 border-sky-700/50' : 
                                 badge.type === 'tag' ? 'bg-yellow-900/40 text-yellow-300 border-yellow-700/50' : 
                                 badge.type === 'remote' ? 'bg-purple-900/40 text-purple-300 border-purple-700/50' :
                                 'bg-emerald-900/40 text-emerald-300 border-emerald-700/50'}"
                            >
                              {badge.text}
                            </span>
                          {/each}
                          <span class="truncate text-[#c9d1d9] group-hover:text-white" title={node.subject}>{node.subject}</span>
                     </div>
                 {:else if col.id === 'author'}
                     <div class="pl-4 truncate text-[#c9d1d9] opacity-80" title={node.author}>{node.author}</div>
                 {:else if col.id === 'date'}
                     <div class="pl-4 text-[#8b949e] opacity-70 font-mono truncate">
                         {new Date(node.date).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute:'2-digit' })}
                     </div>
                 {/if}
               {/each}
             </div>
           {/each}
        </div>

      </div>
  </div>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 10px;
    height: 10px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: #0d1117;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #30363d;
    border: 2px solid #0d1117;
    border-radius: 99px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #484f58;
  }
</style>
