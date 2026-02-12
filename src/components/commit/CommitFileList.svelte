<script lang="ts">
  import { onMount } from "svelte";
  import type { FileStatus } from "../../lib/GitService";
  import { toast } from "../../lib/toast.svelte";
  import FileChangeStatusBadge from "../common/FileChangeStatusBadge.svelte";

  type ViewMode = "tree" | "path";

  type TreeDirectory = {
      name: string;
      path: string;
      children: Map<string, TreeDirectory>;
      files: FileStatus[];
  };

  type DirectoryRow = {
      kind: "directory";
      key: string;
      depth: number;
      path: string;
      name: string;
      fileCount: number;
      collapsed: boolean;
  };

  type FileRow = {
      kind: "file";
      key: string;
      depth: number;
      file: FileStatus;
      label: string;
      title: string;
  };

  type ListRow = DirectoryRow | FileRow;
  type FileContextMenu = {
      visible: boolean;
      x: number;
      y: number;
      file: FileStatus | null;
  };

  const PATH_LABEL_MAX_LENGTH = 42;
  const PATH_COLLAPSE_TOKEN = "....";
  const CONTEXT_MENU_WIDTH = 190;
  const CONTEXT_MENU_ITEM_HEIGHT = 32;
  const CONTEXT_MENU_PADDING_Y = 4;
  const MIN_HEADER_TITLE_WIDTH = 96;

  interface Props {
      title: string;
      files: FileStatus[];
      selectedFile: FileStatus | null;
      onSelect: (file: FileStatus) => void;
      onAction: (file: FileStatus) => void; // Stage or Unstage action
      actionLabel: string; // "Stage" or "Unstage" text for tooltip/aria (or implied by context)
      actionIcon?: string; // Optional custom icon?
      onActionAll?: () => void;
      actionAllLabel?: string;
      onOpenFile?: (file: FileStatus) => void;
      onDiscard?: (file: FileStatus) => void;
      onStash?: (file: FileStatus) => void;
      onStashAll?: () => void;
      stashAllLabel?: string;
      showStashAll?: boolean;
      onDiscardAll?: () => void;
      discardAllLabel?: string;
      showDiscardAll?: boolean;
      viewMode?: ViewMode;
  }
  let {
      title,
      files,
      selectedFile,
      onSelect,
      onAction,
      actionLabel,
      onActionAll,
      actionAllLabel,
      onOpenFile,
      onDiscard,
      onStash,
      onStashAll,
      stashAllLabel,
      showStashAll,
      onDiscardAll,
      discardAllLabel,
      showDiscardAll,
      viewMode = "path"
  }: Props = $props();

  let collapsedDirectories = $state<Set<string>>(new Set());
  let fileContextMenu = $state<FileContextMenu>({
      visible: false,
      x: 0,
      y: 0,
      file: null
  });
  let headerEl = $state<HTMLDivElement | null>(null);
  let actionsEl = $state<HTMLDivElement | null>(null);
  let iconOnlyActions = $state(false);

  let headerResizeObserver: ResizeObserver | null = null;
  let responsiveUpdateRafId = 0;

  function getTreePath(filePath: string): string {
      const normalized = filePath.replaceAll("\\", "/");
      const renameParts = normalized.split(" -> ");
      return (renameParts[renameParts.length - 1] ?? normalized).trim();
  }

  function getBaseName(filePath: string): string {
      const path = getTreePath(filePath);
      const segments = path.split("/").filter(Boolean);
      return segments.length > 0 ? segments[segments.length - 1] : path;
  }

  function collapseSinglePath(path: string, maxLength: number): string {
      const normalized = path.replaceAll("\\", "/").trim();
      if (normalized.length <= maxLength) return normalized;

      const segments = normalized.split("/").filter(Boolean);
      if (segments.length <= 1) {
          return `...${normalized.slice(-Math.max(1, maxLength - 3))}`;
      }

      let first = segments[0];
      let last = segments[segments.length - 1];
      let candidate = `${first}/${PATH_COLLAPSE_TOKEN}/${last}`;

      if (candidate.length > maxLength) {
          const lastBudget = Math.max(10, maxLength - (first.length + PATH_COLLAPSE_TOKEN.length + 5));
          if (last.length > lastBudget) {
              last = `...${last.slice(-Math.max(1, lastBudget - 3))}`;
          }
          candidate = `${first}/${PATH_COLLAPSE_TOKEN}/${last}`;
      }

      if (candidate.length > maxLength) {
          const firstBudget = Math.max(3, maxLength - (PATH_COLLAPSE_TOKEN.length + last.length + 5));
          if (first.length > firstBudget) {
              first = `${first.slice(0, Math.max(1, firstBudget - 3))}...`;
          }
          candidate = `${first}/${PATH_COLLAPSE_TOKEN}/${last}`;
      }

      if (candidate.length <= maxLength) return candidate;
      return `...${normalized.slice(-Math.max(1, maxLength - 3))}`;
  }

  function formatPathLabel(path: string): string {
      const normalized = path.replaceAll("\\", "/").trim();
      const renameParts = normalized.split(" -> ").map((part) => part.trim()).filter(Boolean);

      if (renameParts.length === 2) {
          const leftBudget = Math.max(16, Math.floor((PATH_LABEL_MAX_LENGTH - 4) / 2));
          const rightBudget = Math.max(16, PATH_LABEL_MAX_LENGTH - 4 - leftBudget);
          const left = collapseSinglePath(renameParts[0], leftBudget);
          const right = collapseSinglePath(renameParts[1], rightBudget);
          return `${left} -> ${right}`;
      }

      return collapseSinglePath(normalized, PATH_LABEL_MAX_LENGTH);
  }

  function isSelected(file: FileStatus): boolean {
      return !!selectedFile && selectedFile.path === file.path && selectedFile.staged === file.staged;
  }

  function buildTree(items: FileStatus[]): TreeDirectory {
      const root: TreeDirectory = {
          name: "",
          path: "",
          children: new Map(),
          files: []
      };

      for (const file of items) {
          const treePath = getTreePath(file.path);
          const parts = treePath.split("/").filter(Boolean);

          if (parts.length <= 1) {
              root.files.push(file);
              continue;
          }

          parts.pop();
          let current = root;

          for (const part of parts) {
              let child = current.children.get(part);
              if (!child) {
                  const childPath = current.path ? `${current.path}/${part}` : part;
                  child = {
                      name: part,
                      path: childPath,
                      children: new Map(),
                      files: []
                  };
                  current.children.set(part, child);
              }
              current = child;
          }

          current.files.push(file);
      }

      return root;
  }

  function countFiles(directory: TreeDirectory): number {
      let count = directory.files.length;
      for (const child of directory.children.values()) {
          count += countFiles(child);
      }
      return count;
  }

  function flattenTree(directory: TreeDirectory, depth: number): ListRow[] {
      const rows: ListRow[] = [];

      const directories = [...directory.children.values()].sort((a, b) => a.name.localeCompare(b.name));
      for (const child of directories) {
          const collapsed = collapsedDirectories.has(child.path);
          rows.push({
              kind: "directory",
              key: `dir:${child.path}`,
              depth,
              path: child.path,
              name: child.name,
              fileCount: countFiles(child),
              collapsed
          });

          if (!collapsed) {
              rows.push(...flattenTree(child, depth + 1));
          }
      }

      const directoryFiles = [...directory.files].sort((a, b) => {
          const byName = getBaseName(a.path).localeCompare(getBaseName(b.path));
          if (byName !== 0) return byName;
          return getTreePath(a.path).localeCompare(getTreePath(b.path));
      });

      for (const file of directoryFiles) {
          rows.push({
              kind: "file",
              key: `file:${file.path}:${file.staged}`,
              depth,
              file,
              label: getBaseName(file.path),
              title: file.path
          });
      }

      return rows;
  }

  let rows = $derived.by<ListRow[]>(() => {
      if (viewMode === "path") {
          return [...files]
              .sort((a, b) => a.path.localeCompare(b.path))
              .map((file) => ({
                  kind: "file" as const,
                  key: `file:${file.path}:${file.staged}`,
                  depth: 0,
                  file,
                  label: formatPathLabel(file.path),
                  title: file.path
              }));
      }

      const tree = buildTree(files);
      return flattenTree(tree, 0);
  });

  function toggleDirectory(path: string): void {
      closeFileContextMenu();
      const next = new Set(collapsedDirectories);
      if (next.has(path)) {
          next.delete(path);
      } else {
          next.add(path);
      }
      collapsedDirectories = next;
  }

  function handleFileKeydown(event: KeyboardEvent, file: FileStatus): void {
      if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          closeFileContextMenu();
          onSelect(file);
      }
  }

  function closeFileContextMenu(): void {
      fileContextMenu = {
          visible: false,
          x: 0,
          y: 0,
          file: null
      };
  }

  function getContextMenuHeight(): number {
      let actionCount = 1; // Copy file path is always available.
      if (onOpenFile) actionCount += 1;
      if (onStash) actionCount += 1;
      if (onDiscard) actionCount += 1;
      return actionCount * CONTEXT_MENU_ITEM_HEIGHT + CONTEXT_MENU_PADDING_Y * 2;
  }

  function getContextMenuPosition(clientX: number, clientY: number): { x: number; y: number } {
      const menuHeight = getContextMenuHeight();
      const maxX = Math.max(8, window.innerWidth - CONTEXT_MENU_WIDTH - 8);
      const maxY = Math.max(8, window.innerHeight - menuHeight - 8);
      return {
          x: Math.min(Math.max(8, clientX), maxX),
          y: Math.min(Math.max(8, clientY), maxY)
      };
  }

  function handleFileContextMenu(event: MouseEvent, file: FileStatus): void {
      event.preventDefault();
      event.stopPropagation();
      const pos = getContextMenuPosition(event.clientX, event.clientY);
      fileContextMenu = {
          visible: true,
          x: pos.x,
          y: pos.y,
          file
      };
  }

  function handleOpenThisFile(): void {
      if (!onOpenFile || !fileContextMenu.file) return;
      const target = fileContextMenu.file;
      closeFileContextMenu();
      onOpenFile(target);
  }

  async function handleCopyFilePath(): Promise<void> {
      if (!fileContextMenu.file) return;
      const targetPath = fileContextMenu.file.path;
      closeFileContextMenu();

      try {
          await navigator.clipboard.writeText(targetPath);
          toast.success(`Copied path: ${targetPath}`);
      } catch (e) {
          console.error("Copy file path failed", e);
          toast.error("Copy file path failed");
      }
  }

  function handleStashThisFile(): void {
      if (!onStash || !fileContextMenu.file) return;
      const target = fileContextMenu.file;
      closeFileContextMenu();
      onStash(target);
  }

  function handleDiscardThisFile(): void {
      if (!onDiscard || !fileContextMenu.file) return;
      const target = fileContextMenu.file;
      closeFileContextMenu();
      onDiscard(target);
  }

  function handleWindowMouseDown(event: MouseEvent): void {
      if (!fileContextMenu.visible) return;
      const target = event.target as Element | null;
      if (target?.closest(".file-context-menu")) return;
      closeFileContextMenu();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
      if (!fileContextMenu.visible) return;
      if (event.key === "Escape") {
          event.preventDefault();
          closeFileContextMenu();
      }
  }

  function scheduleResponsiveModeUpdate(): void {
      if (typeof window === "undefined") return;
      if (responsiveUpdateRafId !== 0) {
          window.cancelAnimationFrame(responsiveUpdateRafId);
      }
      responsiveUpdateRafId = window.requestAnimationFrame(() => {
          responsiveUpdateRafId = 0;
          updateResponsiveMode();
      });
  }

  function measureActionWidthWithLabels(): number {
      if (!headerEl || !actionsEl) return 0;

      const clone = actionsEl.cloneNode(true) as HTMLElement;
      clone.classList.remove("commit-file-list-actions-icon-only");
      clone.style.position = "absolute";
      clone.style.left = "-9999px";
      clone.style.top = "0";
      clone.style.visibility = "hidden";
      clone.style.pointerEvents = "none";
      clone.style.width = "max-content";
      clone.setAttribute("aria-hidden", "true");

      headerEl.appendChild(clone);
      const width = Math.ceil(clone.getBoundingClientRect().width);
      clone.remove();

      return width;
  }

  function updateResponsiveMode(): void {
      if (!headerEl || !actionsEl) return;
      const headerWidth = Math.ceil(headerEl.getBoundingClientRect().width);
      const actionsWidthWithLabels = measureActionWidthWithLabels();
      iconOnlyActions = headerWidth < actionsWidthWithLabels + MIN_HEADER_TITLE_WIDTH;
  }

  onMount(() => {
      if (typeof ResizeObserver !== "undefined" && headerEl) {
          headerResizeObserver = new ResizeObserver(() => {
              scheduleResponsiveModeUpdate();
          });
          headerResizeObserver.observe(headerEl);
      }

      scheduleResponsiveModeUpdate();

      return () => {
          if (headerResizeObserver) {
              headerResizeObserver.disconnect();
              headerResizeObserver = null;
          }
          if (responsiveUpdateRafId !== 0) {
              window.cancelAnimationFrame(responsiveUpdateRafId);
              responsiveUpdateRafId = 0;
          }
      };
  });

  $effect(() => {
      files.length;
      showStashAll;
      showDiscardAll;
      onStashAll;
      onDiscardAll;
      onActionAll;
      stashAllLabel;
      discardAllLabel;
      actionAllLabel;
      scheduleResponsiveModeUpdate();
  });
</script>

<svelte:window onmousedown={handleWindowMouseDown} onkeydown={handleWindowKeydown} />

<div class="flex flex-col flex-1 overflow-hidden min-h-0 border-b border-[#30363d] last:border-b-0">
    <div bind:this={headerEl} class="commit-file-list-header h-8 px-3 flex items-center bg-[#21262d] font-semibold text-xs uppercase tracking-wider text-[#8b949e] shrink-0 justify-between group/header">
        <span class="min-w-0 truncate pr-2">{title} ({files.length})</span>
        <div bind:this={actionsEl} class="commit-file-list-actions flex items-center gap-1.5 shrink-0" class:commit-file-list-actions-icon-only={iconOnlyActions}>
            {#if onStashAll && (showStashAll ?? files.length > 0)}
                <button
                    class="commit-file-list-action-btn opacity-90 hover:opacity-100 transition-opacity px-2 py-1 rounded hover:bg-[#1f2f45] text-[#58a6ff] hover:text-[#79c0ff] text-xs font-medium flex items-center gap-1.5"
                    onclick={(e) => { e.stopPropagation(); onStashAll(); }}
                    title={stashAllLabel ?? "Stash All Changes"}
                >
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="3 7 12 13 21 7"></polyline>
                        <path d="M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V7"></path>
                    </svg>
                    <span class="commit-file-list-action-label">{stashAllLabel ?? "Stash All"}</span>
                </button>
            {/if}
            {#if onDiscardAll && (showDiscardAll ?? files.length > 0)}
                <button
                    class="commit-file-list-action-btn opacity-90 hover:opacity-100 transition-opacity px-2 py-1 rounded hover:bg-[#3b1f2c] text-[#f85149] hover:text-[#ff7b72] text-xs font-medium flex items-center gap-1.5"
                    onclick={(e) => { e.stopPropagation(); onDiscardAll(); }}
                    title={discardAllLabel ?? "Discard All Changes"}
                >
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="3 6 5 6 21 6"></polyline>
                        <path d="M19 6l-1 14H6L5 6m3 0V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2"></path>
                    </svg>
                    <span class="commit-file-list-action-label">{discardAllLabel ?? "Discard All"}</span>
                </button>
            {/if}
            {#if files.length > 0 && onActionAll}
                <button 
                    class="commit-file-list-action-btn opacity-90 hover:opacity-100 transition-opacity px-2 py-1 rounded hover:bg-[#30363d] text-[#58a6ff] hover:text-[#79c0ff] text-xs font-medium flex items-center gap-1.5"
                    onclick={(e) => { e.stopPropagation(); onActionAll(); }}
                    title={actionAllLabel}
                >
                    {#if actionAllLabel?.toLowerCase().includes('unstage')}
                        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
                    {:else}
                        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14"/></svg>
                    {/if}
                    <span class="commit-file-list-action-label">{actionAllLabel}</span>
                </button>
            {/if}
        </div>
    </div>
    <div class="flex-1 overflow-y-auto custom-scrollbar p-1">
        {#if files.length === 0}
            <div class="text-[#8b949e] text-xs text-center mt-4 italic opacity-60">Empty</div>
        {:else}
            {#each rows as row (row.key)}
                {#if row.kind === "directory"}
                    <button
                        type="button"
                        class="w-full flex items-center gap-1.5 px-2 py-1.5 text-xs rounded text-[#8b949e] hover:bg-[#21262d] transition-colors"
                        style={`padding-left: ${8 + row.depth * 14}px;`}
                        onclick={() => toggleDirectory(row.path)}
                        title={row.path}
                    >
                        <svg class={`w-3 h-3 shrink-0 transition-transform ${row.collapsed ? "" : "rotate-90"}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <polyline points="9 6 15 12 9 18"></polyline>
                        </svg>
                        <svg class="w-3.5 h-3.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z"></path>
                        </svg>
                        <span class="truncate text-left">{row.name}</span>
                        <span class="ml-auto text-[10px] text-[#6e7681]">{row.fileCount}</span>
                    </button>
                {:else}
                    <div 
                        class="group flex items-center gap-2 px-2 py-1.5 text-xs rounded cursor-pointer transition-colors relative
                               {isSelected(row.file) ? 'bg-[#30363d] text-white' : 'hover:bg-[#21262d] text-[#c9d1d9]'}"
                        style={`padding-left: ${8 + row.depth * 14}px;`}
                        onclick={() => { closeFileContextMenu(); onSelect(row.file); }}
                        oncontextmenu={(e) => handleFileContextMenu(e, row.file)}
                        role="button"
                        tabindex="0"
                        onkeydown={(e) => handleFileKeydown(e, row.file)}
                    >
                        <FileChangeStatusBadge status={row.file.status} compact={true} showCode={true} className="shrink-0" />
                        <span class="truncate flex-1" title={row.title}>{row.label}</span>
                        
                        <button 
                            class="opacity-0 group-hover:opacity-100 p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-white transition-opacity"
                            onclick={(e) => { e.stopPropagation(); closeFileContextMenu(); onAction(row.file); }}
                            title={actionLabel}
                        >
                            {#if actionLabel === 'Stage'}
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                            {:else}
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                            {/if}
                        </button>
                    </div>
                {/if}
            {/each}
        {/if}
    </div>
</div>

{#if fileContextMenu.visible}
    <div
        class="file-context-menu fixed z-[120] min-w-[180px] rounded-md border border-[#30363d] bg-[#161b22] shadow-2xl overflow-hidden"
        style={`left: ${fileContextMenu.x}px; top: ${fileContextMenu.y}px;`}
        role="menu"
    >
        <button
            type="button"
            class="w-full text-left px-3 py-2 text-xs text-[#c9d1d9] hover:bg-[#21262d] hover:text-white transition-colors"
            onclick={() => void handleCopyFilePath()}
            role="menuitem"
        >
            Copy file path
        </button>
        {#if onOpenFile}
            <button
                type="button"
                class="w-full text-left px-3 py-2 text-xs text-[#c9d1d9] hover:bg-[#21262d] hover:text-white transition-colors"
                onclick={handleOpenThisFile}
                role="menuitem"
            >
                Open file
            </button>
        {/if}
        {#if onStash}
            <button
                type="button"
                class="w-full text-left px-3 py-2 text-xs text-[#58a6ff] hover:bg-[#1f2f45] hover:text-[#79c0ff] transition-colors"
                onclick={handleStashThisFile}
                role="menuitem"
            >
                Stash this file
            </button>
        {/if}
        {#if onDiscard}
            <button
                type="button"
                class="w-full text-left px-3 py-2 text-xs text-[#f85149] hover:bg-[#3b1f2c] hover:text-[#ff7b72] transition-colors"
                onclick={handleDiscardThisFile}
                role="menuitem"
            >
                Discard this file
            </button>
        {/if}
    </div>
{/if}

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

  .commit-file-list-action-label {
    white-space: nowrap;
  }

  .commit-file-list-actions-icon-only .commit-file-list-action-label {
    display: none;
  }

  .commit-file-list-actions-icon-only .commit-file-list-action-btn {
    padding-left: 0.375rem;
    padding-right: 0.375rem;
    gap: 0;
  }
</style>
