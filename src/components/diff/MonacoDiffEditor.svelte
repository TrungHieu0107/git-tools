<script lang="ts">
  import { onMount } from "svelte";
  import * as Monaco from "monaco-editor/esm/vs/editor/editor.api";
  import "../../lib/monaco-setup";

  interface Props {
    originalContent: string;
    modifiedContent: string;
    language?: string;
    viewMode?: "side-by-side" | "hunk" | "inline";
    filePath?: string;
    currentHunkIndex?: number;
    totalHunks?: number;
  }

  let {
    originalContent = "",
    modifiedContent = "",
    language = "plaintext",
    viewMode = "side-by-side",
    filePath = "",
    currentHunkIndex = $bindable(0),
    totalHunks = $bindable(0),
  }: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state();
  let editor: Monaco.editor.IStandaloneDiffEditor | undefined = $state();
  let monacoModule: typeof Monaco | undefined = $state();

  // Expose navigation methods
  export function nextDiff() {
    if (editor) {
      editor.goToDiff("next");
      updateHunkIndex();
    }
  }

  export function previousDiff() {
    if (editor) {
      editor.goToDiff("previous");
      updateHunkIndex();
    }
  }

  function updateHunkIndex() {
    if (!editor) return;
    const changes = editor.getLineChanges();
    if (!changes) {
      totalHunks = 0;
      currentHunkIndex = 0;
      return;
    }
    totalHunks = changes.length;
    
    // Determine current hunk based on modified editor cursor position
    const modifiedEditor = editor.getModifiedEditor();
    const position = modifiedEditor.getPosition();
    if (!position) return;

    let index = changes.findIndex(change => 
      position.lineNumber >= change.modifiedStartLineNumber && 
      position.lineNumber <= (change.modifiedEndLineNumber || change.modifiedStartLineNumber)
    );
    
    if (index === -1) {
      // Find the closest previous change
      index = changes.findLastIndex(change => position.lineNumber > change.modifiedEndLineNumber);
      if (index === -1) index = 0;
    }
    
    currentHunkIndex = index;
  }

  // Guess language from file extension
  function guessLanguage(path: string): string {
    if (!path) return "plaintext";
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    const map: Record<string, string> = {
      ts: "typescript",
      tsx: "typescript",
      js: "javascript",
      jsx: "javascript",
      json: "json",
      html: "html",
      htm: "html",
      css: "css",
      scss: "scss",
      less: "less",
      md: "markdown",
      xml: "xml",
      yaml: "yaml",
      yml: "yaml",
      py: "python",
      rs: "rust",
      go: "go",
      java: "java",
      c: "c",
      cpp: "cpp",
      h: "c",
      hpp: "cpp",
      cs: "csharp",
      rb: "ruby",
      php: "php",
      sh: "shell",
      bash: "shell",
      sql: "sql",
      svelte: "html",
      vue: "html",
      toml: "ini",
      ini: "ini",
      conf: "ini",
      bat: "bat",
      ps1: "powershell",
      dockerfile: "dockerfile",
    };
    return map[ext] ?? "plaintext";
  }

  // Define custom dark theme matching app aesthetic
  function defineTheme(monaco: typeof Monaco) {
    monaco.editor.defineTheme("git-tools-dark", {
      base: "vs-dark",
      inherit: true,
      rules: [
        { token: "", foreground: "c9d1d9", background: "0d1117" },
        { token: "comment", foreground: "8b949e", fontStyle: "italic" },
        { token: "keyword", foreground: "ff7b72" },
        { token: "string", foreground: "a5d6ff" },
        { token: "number", foreground: "79c0ff" },
        { token: "type", foreground: "ffa657" },
        { token: "variable", foreground: "ffa657" },
      ],
      colors: {
        "editor.background": "#0d1117",
        "editor.foreground": "#c9d1d9",
        "editor.lineHighlightBackground": "#161b2200",
        "editorLineNumber.foreground": "#484f58",
        "editorLineNumber.activeForeground": "#c9d1d9",
        "editor.selectionBackground": "#1f6feb44",
        "editor.inactiveSelectionBackground": "#1f6feb22",
        "editorGutter.background": "#0d1117",
        "diffEditor.insertedTextBackground": "#2ea04326",
        "diffEditor.removedTextBackground": "#da363326",
        "diffEditor.insertedLineBackground": "#2ea04315",
        "diffEditor.removedLineBackground": "#da363315",
        "scrollbar.shadow": "#00000000",
        "scrollbarSlider.background": "#30363d80",
        "scrollbarSlider.hoverBackground": "#484f5880",
        "scrollbarSlider.activeBackground": "#6e768180",
      },
    });
  }

  onMount(() => {
    let disposed = false;

    if (disposed || !containerEl) return;

    monacoModule = Monaco;
    defineTheme(Monaco);

    const effectiveLang = language !== "plaintext" ? language : guessLanguage(filePath);

    const originalModel = Monaco.editor.createModel(originalContent, effectiveLang);
    const modifiedModel = Monaco.editor.createModel(modifiedContent, effectiveLang);

    editor = Monaco.editor.createDiffEditor(containerEl, {
        theme: "git-tools-dark",
        automaticLayout: true,
        readOnly: true,
        renderSideBySide: viewMode === "side-by-side",
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        fontSize: 12,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, 'Courier New', monospace",
        lineHeight: 20,
        renderWhitespace: "boundary",
        scrollbar: {
          verticalScrollbarSize: 10,
          horizontalScrollbarSize: 10,
          verticalSliderSize: 6,
          horizontalSliderSize: 6,
        },
        overviewRulerLanes: 2,
        glyphMargin: false,
        folding: false,
        lineNumbersMinChars: 4,
        renderOverviewRuler: true,
        diffWordWrap: "off",
        ignoreTrimWhitespace: false,
        renderIndicators: true,
        originalEditable: false,
        contextmenu: true,
        hideUnchangedRegions: { enabled: viewMode === "hunk" },
      });

      editor.setModel({
        original: originalModel,
        modified: modifiedModel,
      });

      // Listen for diff updates and cursor changes to track hunk index
      editor.onDidUpdateDiff(() => {
        updateHunkIndex();
      });

      editor.getModifiedEditor().onDidChangeCursorPosition(() => {
        updateHunkIndex();
      });
    // Removed closure end
    
    return () => {
      disposed = true;
      if (editor) {
        const model = editor.getModel();
        editor.dispose();
        model?.original?.dispose();
        model?.modified?.dispose();
        editor = undefined;
      }
    };
  });

  // React to content changes
  $effect(() => {
    if (!editor || !monacoModule) return;
    const effectiveLang = language !== "plaintext" ? language : guessLanguage(filePath);

    const model = editor.getModel();
    if (model) {
      const origValue = model.original.getValue();
      const modValue = model.modified.getValue();

      if (origValue !== originalContent || modValue !== modifiedContent) {
        // Dispose old models and create new ones
        model.original.dispose();
        model.modified.dispose();

        const newOriginal = monacoModule.editor.createModel(originalContent, effectiveLang);
        const newModified = monacoModule.editor.createModel(modifiedContent, effectiveLang);
        editor.setModel({ original: newOriginal, modified: newModified });
      } else {
        // Just update language if needed
        monacoModule.editor.setModelLanguage(model.original, effectiveLang);
        monacoModule.editor.setModelLanguage(model.modified, effectiveLang);
      }
    }
  });

  // React to view mode updates
  $effect(() => {
    if (!editor) return;
    editor.updateOptions({ 
      renderSideBySide: viewMode === "side-by-side",
      hideUnchangedRegions: { enabled: viewMode === "hunk" }
    });
  });
</script>

<div class="monaco-diff-container" bind:this={containerEl}></div>

<style>
  .monaco-diff-container {
    width: 100%;
    height: 100%;
    min-height: 200px;
  }
</style>
