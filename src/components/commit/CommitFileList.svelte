<script lang="ts">
  import type { FileStatus } from "../../lib/GitService";
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
  const PATH_LABEL_MAX_LENGTH = 42;
  const PATH_COLLAPSE_TOKEN = "....";

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
      onDiscard?: (file: FileStatus) => void;
      onDiscardAll?: () => void;
      discardAllLabel?: string;
      showDiscardAll?: boolean;
      viewMode?: ViewMode;
  }
  let { title, files, selectedFile, onSelect, onAction, actionLabel, onActionAll, actionAllLabel, onDiscard, onDiscardAll, discardAllLabel, showDiscardAll, viewMode = "path" }: Props = $props();

  let collapsedDirectories = $state<Set<string>>(new Set());

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
          onSelect(file);
      }
  }

  function handleFileContextMenu(event: MouseEvent, file: FileStatus): void {
      if (!onDiscard) return;
      event.preventDefault();
      event.stopPropagation();
      onDiscard(file);
  }
</script>

<div class="flex flex-col flex-1 overflow-hidden min-h-0 border-b border-[#30363d] last:border-b-0">
    <div class="h-8 px-3 flex items-center bg-[#21262d] font-semibold text-xs uppercase tracking-wider text-[#8b949e] shrink-0 justify-between group/header">
        <span>{title} ({files.length})</span>
        <div class="flex items-center gap-1.5">
            {#if onDiscardAll && (showDiscardAll ?? files.length > 0)}
                <button
                    class="opacity-90 hover:opacity-100 transition-opacity px-2 py-1 rounded hover:bg-[#3b1f2c] text-[#f85149] hover:text-[#ff7b72] text-xs font-medium flex items-center gap-1.5"
                    onclick={(e) => { e.stopPropagation(); onDiscardAll(); }}
                    title={discardAllLabel ?? "Discard All Changes"}
                >
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="3 6 5 6 21 6"></polyline>
                        <path d="M19 6l-1 14H6L5 6m3 0V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2"></path>
                    </svg>
                    {discardAllLabel ?? "Discard All"}
                </button>
            {/if}
            {#if files.length > 0 && onActionAll}
                <button 
                    class="opacity-90 hover:opacity-100 transition-opacity px-2 py-1 rounded hover:bg-[#30363d] text-[#58a6ff] hover:text-[#79c0ff] text-xs font-medium flex items-center gap-1.5"
                    onclick={(e) => { e.stopPropagation(); onActionAll(); }}
                    title={actionAllLabel}
                >
                    {#if actionAllLabel?.toLowerCase().includes('unstage')}
                        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
                    {:else}
                        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14"/></svg>
                    {/if}
                    {actionAllLabel}
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
                        onclick={() => onSelect(row.file)}
                        oncontextmenu={(e) => handleFileContextMenu(e, row.file)}
                        role="button"
                        tabindex="0"
                        onkeydown={(e) => handleFileKeydown(e, row.file)}
                    >
                        <FileChangeStatusBadge status={row.file.status} compact={true} showCode={true} className="shrink-0" />
                        <span class="truncate flex-1" title={row.title}>{row.label}</span>
                        
                        <button 
                            class="opacity-0 group-hover:opacity-100 p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-white transition-opacity"
                            onclick={(e) => { e.stopPropagation(); onAction(row.file); }}
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
