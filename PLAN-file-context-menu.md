# Plan: File Context Menu - Changes Section

## Muc tieu

Implement day du context menu khi right-click vao file trong Changes section, tuong tu giao dien trong hinh:

```
┌─────────────────────────────┐
│ Stage                       │
│ Discard changes             │
│ Ignore                    ▸ │
│ Stash file                  │
├─────────────────────────────┤
│ File History                │
│ File Blame                  │
├─────────────────────────────┤
│ Open in external diff tool  │
│ Open in external editor     │
│ Open file in default program│
│ Show in folder              │
├─────────────────────────────┤
│ Copy file path              │
│ Create patch from changes   │
├─────────────────────────────┤
│ Edit file                   │
│ Delete file                 │
└─────────────────────────────┘
```

---

## Phan tich hien trang

### Context menu hien tai (CommitFileList.svelte, line 562-607)

Chi co 4 menu items:
1. **Copy file path** - DA CO
2. **Open file** - DA CO (goi `cmd_open_repo_file`)
3. **Stash this file** - DA CO (goi `cmd_git_stash_file`)
4. **Discard this file** - DA CO (goi `cmd_git_discard_changes`)

### Backend commands hien co

| Feature | Backend Command | Status |
|---------|----------------|--------|
| Stage | `cmd_git_add` | DA CO |
| Discard | `cmd_git_discard_changes` | DA CO |
| Stash file | `cmd_git_stash_file` | DA CO |
| File History | `cmd_get_file_history` | DA CO |
| Open file (default program) | `cmd_open_repo_file` | DA CO |
| Write file | `cmd_write_file` | DA CO |
| Ignore (.gitignore) | - | CHUA CO |
| File Blame | - | CHUA CO |
| Open in external diff tool | - | CHUA CO |
| Open in external editor | - | CHUA CO |
| Show in folder | - | CHUA CO |
| Create patch | - | CHUA CO |
| Delete file | - | CHUA CO |

---

## Tong quan cac steps

| # | Noi dung | Loai | Do kho |
|---|----------|------|--------|
| 1 | Them `Stage` vao context menu | Frontend only | De |
| 2 | Them `Discard changes` vao context menu (da co handler, chua hien thi dung label) | Frontend refactor | De |
| 3 | Them submenu `Ignore` (.gitignore) | Backend + Frontend | Trung binh |
| 4 | Them `Stash file` (da co) | Rename label | De |
| 5 | Them `File History` | Frontend wiring | De |
| 6 | Them `File Blame` | Backend + Frontend | Kho |
| 7 | Them `Open in external diff tool` | Backend + Frontend | Kho |
| 8 | Them `Open in external editor` | Backend + Frontend | Trung binh |
| 9 | Them `Open file in default program` (da co la "Open file") | Rename label | De |
| 10 | Them `Show in folder` | Backend + Frontend | Trung binh |
| 11 | Them `Copy file path` (da co) | Giu nguyen | De |
| 12 | Them `Create patch from file changes` | Backend + Frontend | Trung binh |
| 13 | Them `Edit file` | Frontend wiring | Trung binh |
| 14 | Them `Delete file` | Backend + Frontend | Trung binh |
| 15 | Them separators (visual dividers) giua cac nhom | Frontend styling | De |

---

## Chi tiet tung step

---

### Step 1: Them "Stage" / "Unstage" vao context menu

**Hien trang:** Context menu khong co option Stage/Unstage. Chi co action button (+/-) tren file row.

**Can lam:**
- Trong `CommitFileList.svelte`, them menu item "Stage" (hoac "Unstage" tuy thuoc vao section)
- Su dung `onAction` callback da co san (prop cua `CommitFileList`)
- Neu la unstaged file → hien thi "Stage"
- Neu la staged file → hien thi "Unstage"

**Files can sua:**
- `src/components/commit/CommitFileList.svelte` - Them menu item trong phan context menu HTML

**Code logic:**
```html
<button onclick={handleStageFromContextMenu}>
  {actionLabel}  <!-- "Stage" hoac "Unstage" tuy section -->
</button>
```

**Handler moi:**
```typescript
function handleStageFromContextMenu(): void {
    if (!fileContextMenu.file) return;
    const target = fileContextMenu.file;
    closeFileContextMenu();
    onAction(target); // onAction da la handleStage hoac handleUnstage
}
```

---

### Step 2: Them "Discard changes" vao context menu (cap nhat label)

**Hien trang:** Da co `onDiscard` handler va menu item "Discard this file".

**Can lam:**
- Doi label thanh "Discard changes" cho khop voi design
- Giu nguyen logic hien tai (`handleDiscardThisFile`)

**Files can sua:**
- `src/components/commit/CommitFileList.svelte` - Doi text tu "Discard this file" → "Discard changes"

---

### Step 3: Them submenu "Ignore" (gitignore)

**Hien trang:** Khong co tinh nang ignore.

**Can lam:**

#### 3a. Backend - Them Rust command

Tao command `cmd_git_ignore_file` trong `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub async fn cmd_git_ignore_file(
    state: State<'_, AppState>,
    pattern: String,     // Pattern de them vao .gitignore (vd: "file.txt", "*.log", "build/")
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let gitignore_path = Path::new(&r_path).join(".gitignore");

    // Doc noi dung hien tai cua .gitignore (neu co)
    let mut content = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    // Kiem tra pattern da ton tai chua
    let already_exists = content.lines().any(|line| line.trim() == pattern.trim());
    if already_exists {
        return Ok(());
    }

    // Them pattern vao cuoi file
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str(pattern.trim());
    content.push('\n');

    std::fs::write(&gitignore_path, content).map_err(|e| e.to_string())?;
    Ok(())
}
```

Dang ky command trong `main.rs`:
```rust
commands::cmd_git_ignore_file,
```

#### 3b. Frontend - Service layer

Them vao `src/lib/services/FileService.ts`:
```typescript
static async ignoreFile(pattern: string, repoPath?: string): Promise<void> {
    await executeCommand<void>(
        "cmd_git_ignore_file",
        { pattern, repoPath },
        `Added ${pattern} to .gitignore`,
        "Ignore file failed",
    );
}
```

Them vao `src/lib/GitService.ts`:
```typescript
static async ignoreFile(pattern: string, repoPath?: string): Promise<void> {
    return FileService.ignoreFile(pattern, repoPath);
}
```

#### 3c. Frontend - Context menu submenu

Them submenu "Ignore" voi cac options:
- **Ignore this file** → Them ten file chinh xac vao .gitignore (vd: `src/config.ts`)
- **Ignore by extension** → Them pattern `*.ext` (vd: `*.log`)
- **Ignore parent folder** → Them folder chua file (vd: `build/`)

**CommitFileList.svelte - Them submenu state:**
```typescript
let ignoreSubmenuOpen = $state(false);
```

**CommitFileList.svelte - Props moi:**
```typescript
// Trong interface Props, them:
onIgnore?: (pattern: string) => void;
```

**CommitPanel.svelte - Truyen callback:**
```typescript
async function handleIgnoreFile(pattern: string) {
    if (!repoPath) return;
    try {
        await GitService.ignoreFile(pattern, repoPath);
        await loadStatus(true);
    } catch (e) { /* toast handled */ }
}
```

---

### Step 4: Cap nhat label "Stash this file" → "Stash file"

**Can lam:** Doi label text.

**Files can sua:**
- `src/components/commit/CommitFileList.svelte` - Doi "Stash this file" → "Stash file"

---

### Step 5: Them "File History"

**Hien trang:** Da co `FileHistoryPanel.svelte` va `cmd_get_file_history`. Nhung chua co cach navigate truc tiep tu context menu.

**Can lam:**

**CommitFileList.svelte - Them prop:**
```typescript
onShowHistory?: (file: FileStatus) => void;
```

**CommitPanel.svelte - Truyen callback:**
Khi user click "File History", chuyen sang tab History voi file path da chon.
Cach implement phu thuoc vao architecture Workspace:
- Neu Workspace co tab system → emit event de switch sang History tab va set file path
- Hoac mo dialog/panel rieng hien thi history

**Option A: Emit event len Workspace**
```typescript
// CommitPanel.svelte
function handleShowFileHistory(file: FileStatus) {
    // Dispatch custom event de Workspace chuyen sang History tab
    window.dispatchEvent(new CustomEvent('show-file-history', {
        detail: { filePath: file.path }
    }));
}
```

**Option B: Truyen callback tu Workspace**
```typescript
// Workspace.svelte truyen onShowFileHistory prop xuong CommitPanel
// CommitPanel truyen xuong CommitFileList
```

**Files can sua:**
- `src/components/commit/CommitFileList.svelte` - Them prop + menu item
- `src/components/CommitPanel.svelte` - Them handler + truyen prop
- `src/components/Workspace.svelte` - Them logic chuyen tab + set filePath

---

### Step 6: Them "File Blame"

**Hien trang:** CHUA CO backend command, CHUA CO UI component.

**Can lam:**

#### 6a. Backend - Them Rust command

Tao `cmd_git_blame` trong `src-tauri/src/commands.rs` (hoac tao file moi `blame_commands.rs`):

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlameLine {
    pub commit_hash: String,
    pub author: String,
    pub date: String,
    pub line_number: u32,
    pub content: String,
}

#[tauri::command]
pub async fn cmd_git_blame(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<Vec<BlameLine>, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    // git blame --porcelain <file>
    let args = vec![
        "blame".to_string(),
        "--line-porcelain".to_string(),
        file_path,
    ];

    let resp = state.git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    // Parse porcelain blame output
    // Moi block bat dau bang commit hash, sau do la metadata, cuoi cung la "\t<content>"
    parse_blame_output(&resp.stdout)
}
```

Dang ky command trong `main.rs`.

#### 6b. Frontend - Service + Type

Them vao `src/lib/types.ts`:
```typescript
export interface BlameLine {
    commitHash: string;
    author: string;
    date: string;
    lineNumber: number;
    content: string;
}
```

Them vao `src/lib/services/CommitService.ts`:
```typescript
static async getBlame(filePath: string, repoPath?: string): Promise<BlameLine[]> {
    return invoke("cmd_git_blame", { filePath, repoPath });
}
```

#### 6c. Frontend - BlameView component

Tao `src/components/blame/BlameView.svelte`:
- Hien thi file content voi blame annotations (author, date, commit hash) moi dong
- Styling tuong tu diff view (font mono, line numbers)
- Click vao commit hash → mo commit detail

#### 6d. Ket noi vao context menu

Tuong tu Step 5, them prop `onShowBlame` va navigate den BlameView.

**Files can sua/tao moi:**
- `src-tauri/src/commands.rs` - Them command
- `src-tauri/src/main.rs` - Dang ky command
- `src/lib/types.ts` - Them BlameLine type
- `src/lib/services/CommitService.ts` - Them method
- `src/lib/GitService.ts` - Them method
- `src/components/blame/BlameView.svelte` - TAO MOI
- `src/components/commit/CommitFileList.svelte` - Them menu item
- `src/components/CommitPanel.svelte` - Them handler
- `src/components/Workspace.svelte` - Them tab/panel cho blame

---

### Step 7: Them "Open in external diff tool"

**Hien trang:** CHUA CO.

**Can lam:**

#### 7a. Backend - Them Rust command

```rust
#[tauri::command]
pub async fn cmd_open_in_diff_tool(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;

    // Su dung `git difftool --no-prompt` de mo diff tool da cau hinh trong git config
    let mut args = vec!["difftool".to_string(), "--no-prompt".to_string()];
    if staged {
        args.push("--cached".to_string());
    }
    args.push("--".to_string());
    args.push(file_path);

    state.git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

**Luu y:** Yeu cau user da cau hinh `diff.tool` trong git config. Nen check truoc:
```bash
git config diff.tool
```
Neu chua cau hinh → tra ve loi thong bao cho user.

#### 7b. Frontend - Service + Context menu

Them vao `FileService.ts`:
```typescript
static async openInDiffTool(filePath: string, staged: boolean, repoPath?: string): Promise<void> {
    await executeCommand<void>(
        "cmd_open_in_diff_tool",
        { filePath, staged, repoPath },
        "",
        "Open in diff tool failed",
    );
}
```

**Files can sua/tao moi:**
- `src-tauri/src/commands.rs` - Them command
- `src-tauri/src/main.rs` - Dang ky command
- `src/lib/services/FileService.ts` - Them method
- `src/lib/GitService.ts` - Them method
- `src/components/commit/CommitFileList.svelte` - Them prop + menu item
- `src/components/CommitPanel.svelte` - Them handler

---

### Step 8: Them "Open in external editor"

**Hien trang:** CHUA CO.

**Can lam:**

#### 8a. Backend - Them Rust command

```rust
#[tauri::command]
pub async fn cmd_open_in_editor(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let full_path = Path::new(&r_path).join(&file_path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()));
    }

    let path_str = full_path.to_string_lossy().to_string();

    // Thu mo bang editor da cau hinh, fallback sang VS Code, sau do la default
    // 1. Check git config core.editor
    // 2. Check VISUAL env var
    // 3. Check EDITOR env var
    // 4. Fallback: "code" (VS Code)

    let editor = get_configured_editor(&state, &r_path).await;

    let mut cmd = std::process::Command::new(&editor);
    cmd.arg(&path_str);
    hide_console_window(&mut cmd);
    cmd.spawn().map_err(|e| format!("Failed to open editor '{}': {}", editor, e))?;

    Ok(())
}

async fn get_configured_editor(state: &State<'_, AppState>, repo_path: &str) -> String {
    // Try git config core.editor
    if let Ok(resp) = state.git.run(
        Path::new(repo_path),
        &["config".to_string(), "core.editor".to_string()],
        TIMEOUT_QUICK,
    ).await {
        let editor = resp.stdout.trim().to_string();
        if !editor.is_empty() {
            return editor;
        }
    }

    // Try env vars
    if let Ok(editor) = std::env::var("VISUAL") {
        if !editor.is_empty() { return editor; }
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.is_empty() { return editor; }
    }

    // Fallback to VS Code
    "code".to_string()
}
```

#### 8b. Frontend

Tuong tu Step 7, them service method va context menu item.

**Files can sua/tao moi:**
- `src-tauri/src/commands.rs` - Them command + helper function
- `src-tauri/src/main.rs` - Dang ky command
- `src/lib/services/FileService.ts` - Them method
- `src/lib/GitService.ts` - Them method
- `src/components/commit/CommitFileList.svelte` - Them prop + menu item
- `src/components/CommitPanel.svelte` - Them handler

---

### Step 9: Doi label "Open file" → "Open file in default program"

**Can lam:** Doi label text va giu nguyen logic.

**Files can sua:**
- `src/components/commit/CommitFileList.svelte` - Doi text

---

### Step 10: Them "Show in folder"

**Hien trang:** CHUA CO. `cmd_open_repo_file` mo file, khong mo folder chua file.

**Can lam:**

#### 10a. Backend - Them Rust command

```rust
#[tauri::command]
pub async fn cmd_show_in_folder(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();

    let target_path = if let Some((_old, new_path)) = split_rename_path(raw_path) {
        new_path
    } else {
        raw_path.to_string()
    };

    let candidate = PathBuf::from(&target_path);
    let full_path = if candidate.is_absolute() {
        candidate
    } else {
        Path::new(&r_path).join(candidate)
    };

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()));
    }

    let path_str = full_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg("/select,").arg(&path_str);
        hide_console_window(&mut cmd);
        cmd.spawn().map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")      // Reveal in Finder
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Linux: open parent folder
        if let Some(parent) = full_path.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
```

#### 10b. Frontend

Them service method va context menu item.

**Files can sua/tao moi:**
- `src-tauri/src/commands.rs` - Them command
- `src-tauri/src/main.rs` - Dang ky command
- `src/lib/services/FileService.ts` - Them method
- `src/lib/GitService.ts` - Them method
- `src/components/commit/CommitFileList.svelte` - Them prop + menu item
- `src/components/CommitPanel.svelte` - Them handler

---

### Step 11: "Copy file path" (da co, giu nguyen)

Khong can thay doi logic. Chi can dam bao vi tri trong menu dung nhu design.

---

### Step 12: Them "Create patch from file changes"

**Hien trang:** CHUA CO.

**Can lam:**

#### 12a. Backend - Them Rust command

```rust
#[tauri::command]
pub async fn cmd_create_patch(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    repo_path: Option<String>,
) -> Result<String, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;

    // Tao patch content dung git diff
    let mut args = vec!["diff".to_string()];
    if staged {
        args.push("--cached".to_string());
    }
    args.push("--".to_string());
    args.push(file_path);

    let resp = state.git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.stdout)
}
```

#### 12b. Frontend - Save patch file

Co 2 options:
- **Option A:** Copy patch content vao clipboard
- **Option B:** Mo Save File dialog de luu ra file .patch

Recommend Option B dung Tauri dialog:
```typescript
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";

static async createPatch(filePath: string, staged: boolean, repoPath?: string): Promise<void> {
    const patchContent = await invoke<string>("cmd_create_patch", {
        filePath, staged, repoPath
    });

    const savePath = await save({
        defaultPath: `${getFileName(filePath)}.patch`,
        filters: [{ name: "Patch Files", extensions: ["patch", "diff"] }],
    });

    if (savePath) {
        await writeTextFile(savePath, patchContent);
        toast.success(`Saved patch to ${savePath}`);
    }
}
```

**Files can sua/tao moi:**
- `src-tauri/src/commands.rs` - Them command
- `src-tauri/src/main.rs` - Dang ky command
- `src/lib/services/FileService.ts` - Them method
- `src/lib/GitService.ts` - Them method
- `src/components/commit/CommitFileList.svelte` - Them prop + menu item
- `src/components/CommitPanel.svelte` - Them handler

---

### Step 13: Them "Edit file"

**Hien trang:** Chua co trong context menu.

**Can lam:**

"Edit file" co the co 2 cach hieu:
- **Option A:** Mo file trong external editor (giong Step 8)
- **Option B:** Mo file trong built-in editor cua app

**Recommend Option A** (giong VS Code behavior): "Edit file" = mo file trong editor da cau hinh.
Neu phan biet voi "Open in external editor" → "Edit file" dung editor nhe (vd: notepad, nano), "Open in external editor" dung IDE (vd: VS Code).

**Hoac:** "Edit file" navigate den 1 simple editor view built-in trong app (textarea + save button).

**De don gian, implement Option A truoc:**
- "Edit file" goi lai cung `cmd_open_in_editor` nhu Step 8
- Hoac tao built-in editor modal voi `cmd_write_file` de save

**Files can sua:**
- `src/components/commit/CommitFileList.svelte` - Them menu item
- `src/components/CommitPanel.svelte` - Them handler

---

### Step 14: Them "Delete file"

**Hien trang:** CHUA CO backend command.

**Can lam:**

#### 14a. Backend - Them Rust command

```rust
#[tauri::command]
pub async fn cmd_delete_file(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();

    let target_path = if let Some((_old, new_path)) = split_rename_path(raw_path) {
        new_path
    } else {
        raw_path.to_string()
    };

    let full_path = Path::new(&r_path).join(&target_path);

    if !full_path.starts_with(&r_path) {
        return Err("Invalid path: cannot delete outside of repository".to_string());
    }

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()));
    }

    std::fs::remove_file(&full_path)
        .map_err(|e| format!("Failed to delete {}: {}", target_path, e))?;

    Ok(())
}
```

#### 14b. Frontend

Them confirmation dialog truoc khi xoa:
```typescript
async function handleDeleteFile(file: FileStatus) {
    if (!repoPath) return;
    const confirmed = await confirm({
        title: "Delete File",
        message: `Delete "${file.path}" permanently?\nThis action cannot be undone.`,
        confirmLabel: "Delete",
        cancelLabel: "Cancel"
    });
    if (!confirmed) return;

    try {
        await GitService.deleteFile(file.path, repoPath);
        await loadStatus(true);
    } catch (e) { /* toast handled */ }
}
```

**Files can sua/tao moi:**
- `src-tauri/src/commands.rs` - Them command
- `src-tauri/src/main.rs` - Dang ky command
- `src/lib/services/FileService.ts` - Them method
- `src/lib/GitService.ts` - Them method
- `src/components/commit/CommitFileList.svelte` - Them prop + menu item
- `src/components/CommitPanel.svelte` - Them handler + confirm dialog

---

### Step 15: Them separators giua cac nhom trong context menu

**Can lam:**

Them `<div>` separator giua cac nhom menu items trong CommitFileList.svelte:

```html
<!-- Group 1: Git actions -->
Stage / Discard changes / Ignore ▸ / Stash file

<!-- Separator -->
<div class="border-t border-[#30363d] my-1"></div>

<!-- Group 2: History -->
File History / File Blame

<!-- Separator -->
<div class="border-t border-[#30363d] my-1"></div>

<!-- Group 3: Open actions -->
Open in external diff tool / Open in external editor / Open file in default program / Show in folder

<!-- Separator -->
<div class="border-t border-[#30363d] my-1"></div>

<!-- Group 4: Path -->
Copy file path / Create patch from file changes

<!-- Separator -->
<div class="border-t border-[#30363d] my-1"></div>

<!-- Group 5: Destructive -->
Edit file / Delete file
```

**Files can sua:**
- `src/components/commit/CommitFileList.svelte` - Them separator divs

---

## Cap nhat `CONTEXT_MENU_WIDTH` va `CONTEXT_MENU_ITEM_HEIGHT`

Menu moi nhieu items hon nen can:
- Tang `CONTEXT_MENU_WIDTH` tu 190 → 260 (de vua text dai hon nhu "Open in external diff tool")
- Cap nhat `getContextMenuHeight()` de tinh dung so luong items + separators

---

## Thu tu implement khuyen nghi

### Phase 1: Frontend only (khong can backend moi)
1. Step 1 - Stage/Unstage
2. Step 2 - Discard changes (doi label)
3. Step 4 - Stash file (doi label)
4. Step 9 - Open file in default program (doi label)
5. Step 11 - Copy file path (giu nguyen)
6. Step 15 - Separators
7. Step 5 - File History (wiring)

### Phase 2: Backend commands don gian
8. Step 10 - Show in folder
9. Step 3 - Ignore (.gitignore)
10. Step 14 - Delete file

### Phase 3: Backend commands phuc tap
11. Step 8 - Open in external editor
12. Step 7 - Open in external diff tool
13. Step 12 - Create patch
14. Step 13 - Edit file

### Phase 4: Phuc tap nhat
15. Step 6 - File Blame (can component moi + backend moi)

---

## Tong ket files bi anh huong

### Frontend

| File | Thay doi |
|------|----------|
| `src/components/commit/CommitFileList.svelte` | **THAY DOI LON**: Context menu HTML, them props, them handlers, them submenu Ignore, them separators, cap nhat constants |
| `src/components/CommitPanel.svelte` | Them handlers cho cac action moi, truyen props xuong CommitFileList |
| `src/components/Workspace.svelte` | Them logic navigate sang History/Blame tab |
| `src/lib/GitService.ts` | Them cac method moi: ignoreFile, showInFolder, openInEditor, openInDiffTool, createPatch, deleteFile, getBlame |
| `src/lib/services/FileService.ts` | Them cac method moi tuong ung |
| `src/lib/services/CommitService.ts` | Them getBlame method |
| `src/lib/types.ts` | Them BlameLine interface |
| `src/components/blame/BlameView.svelte` | **TAO MOI**: Component hien thi blame |

### Backend (Rust)

| File | Thay doi |
|------|----------|
| `src-tauri/src/commands.rs` | Them cac command moi: cmd_git_ignore_file, cmd_show_in_folder, cmd_open_in_editor, cmd_open_in_diff_tool, cmd_create_patch, cmd_delete_file, cmd_git_blame |
| `src-tauri/src/main.rs` | Dang ky tat ca commands moi trong invoke_handler |

### Khong can sua

| File | Ly do |
|------|-------|
| `src/lib/diff.ts` | Khong lien quan |
| `src/lib/diff-types.ts` | Khong lien quan |
| `src/components/diff/*` | Khong lien quan |
