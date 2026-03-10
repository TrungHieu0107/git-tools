<script lang="ts">
  import { onMount } from "svelte";
  import type * as Monaco from "monaco-editor";

  interface Props {
    originalContent: string;
    modifiedContent: string;
    language?: string;
    inlineView?: boolean;
    filePath?: string;
  }

  let {
    originalContent = "",
    modifiedContent = "",
    language = "plaintext",
    inlineView = false,
    filePath = "",
  }: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state();
  let editor: Monaco.editor.IStandaloneDiffEditor | undefined = $state();
  let monacoModule: typeof Monaco | undefined = $state();

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

    // Dynamically import monaco to avoid SSR issues
    Promise.all([
      import("../../lib/monaco-setup"),
      import("monaco-editor"),
    ]).then(([_, monaco]) => {
      if (disposed || !containerEl) return;

      monacoModule = monaco;
      defineTheme(monaco);

      const effectiveLang = language !== "plaintext" ? language : guessLanguage(filePath);

      const originalModel = monaco.editor.createModel(originalContent, effectiveLang);
      const modifiedModel = monaco.editor.createModel(modifiedContent, effectiveLang);

      editor = monaco.editor.createDiffEditor(containerEl, {
        theme: "git-tools-dark",
        automaticLayout: true,
        readOnly: true,
        renderSideBySide: !inlineView,
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
      });

      editor.setModel({
        original: originalModel,
        modified: modifiedModel,
      });
    });

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

  // React to inline/side-by-side toggle
  $effect(() => {
    if (!editor) return;
    editor.updateOptions({ renderSideBySide: !inlineView });
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
