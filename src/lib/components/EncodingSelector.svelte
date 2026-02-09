<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let selectedEncoding: string | undefined = undefined;

  const dispatch = createEventDispatcher<{ change: string }>();

  const encodingGroups = [
    {
      group: "Unicode",
      items: [
        { label: "UTF-8", value: "UTF-8" },
        { label: "UTF-16LE", value: "UTF-16LE" },
        { label: "UTF-16BE", value: "UTF-16BE" },
      ]
    },
    {
      group: "Western",
      items: [
        { label: "Windows-1252 (Western)", value: "windows-1252" },
        { label: "ISO-8859-1 (Western)", value: "ISO-8859-1" },
      ]
    },
    {
      group: "Asian",
      items: [
        { label: "Shift_JIS (Japanese)", value: "Shift_JIS" },
        { label: "GBK (Chinese)", value: "GBK" },
        { label: "EUC-KR (Korean)", value: "EUC-KR" },
      ]
    }
  ];

  function handleChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    const value = select.value;
    if (value && value !== selectedEncoding) {
      dispatch("change", value);
    }
  }
</script>

<div class="encoding-selector" title="Reload file and diff using selected text encoding">
    <label for="encoding-select" class="sr-only">Encoding</label>
    <!-- Visual label wrapper to add "Encoding:" prefix style -->
    <div class="select-wrapper">
        <span class="prefix">Encoding:</span>
        <select 
            id="encoding-select"
            value={selectedEncoding || "UTF-8"} 
            on:change={handleChange}
        >
            {#each encodingGroups as group}
                <optgroup label={group.group}>
                    {#each group.items as enc}
                        <option value={enc.value}>{enc.label}</option>
                    {/each}
                </optgroup>
            {/each}
        </select>
        <!-- Custom chevron arrow -->
        <svg class="chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9"></polyline>
        </svg>
    </div>
</div>

<style>
  .encoding-selector {
    display: inline-flex;
    align-items: center;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  .select-wrapper {
      position: relative;
      display: flex;
      align-items: center;
      background-color: #1e293b; /* bg-slate-800 */
      border: 1px solid #475569; /* border-slate-600 */
      border-radius: 4px;
      padding: 0 8px 0 8px;
      height: 24px;
      transition: all 0.2s ease;
  }

  .select-wrapper:hover {
      background-color: #334155; /* bg-slate-700 */
      border-color: #64748b; /* border-slate-500 */
  }

  /* Focus within wrapper to enable custom ring */
  .select-wrapper:focus-within {
      outline: 2px solid #3b82f6; /* ring-blue-500 */
      outline-offset: 1px;
      border-color: #3b82f6;
  }

  .prefix {
      font-size: 11px;
      color: #94a3b8; /* text-slate-400 */
      margin-right: 4px;
      pointer-events: none;
      font-weight: 500;
  }

  select {
      appearance: none;
      background: transparent;
      border: none;
      color: #f1f5f9; /* text-slate-100 */
      font-size: 11px;
      font-family: inherit;
      cursor: pointer;
      outline: none;
      padding-right: 16px; /* Space for chevron */
      margin: 0;
      width: 100%;
  }

  /* Dropdown options styling (browser dependent, but we provide hints) */
  option {
      background-color: #0f172a; /* bg-slate-900 */
      color: #f1f5f9; /* text-slate-100 */
  }
  
  option:checked {
      background-color: #2563eb; /* bg-blue-600 */
      color: #ffffff;
      font-weight: 600;
  }

  optgroup {
      background-color: #0f172a; /* bg-slate-900 */
      color: #94a3b8; /* text-slate-400 */
      font-weight: 600;
  }

  .chevron {
      position: absolute;
      right: 6px;
      top: 50%;
      transform: translateY(-50%);
      pointer-events: none;
      color: #94a3b8; /* text-slate-400 */
      width: 10px;
      height: 10px;
  }
  
  .select-wrapper:hover .chevron {
      color: #f1f5f9; /* text-slate-100 */
  }
</style>
