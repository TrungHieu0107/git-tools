# 📋 Kịch Bản Test Ứng Dụng Git GUI (Git Tools)

Tài liệu này mô tả toàn bộ kịch bản test cho ứng dụng desktop Git GUI, đảm bảo tất cả tính năng hoạt động chính xác, không có lỗi, và UI responsive.

---

## Mục Lục

1. [Khởi động ứng dụng](#1-khởi-động-ứng-dụng)
2. [Quản lý Repository](#2-quản-lý-repository)
3. [Tab System](#3-tab-system)
4. [Commit Graph](#4-commit-graph)
5. [Commit Panel (Stage/Unstage/Commit)](#5-commit-panel)
6. [Diff View](#6-diff-view)
7. [Branch Explorer](#7-branch-explorer)
8. [Merge](#8-merge)
9. [Rebase](#9-rebase)
10. [Interactive Rebase](#10-interactive-rebase)
11. [Cherry-pick](#11-cherry-pick)
12. [Conflict Resolution](#12-conflict-resolution)
13. [Stash](#13-stash)
14. [Tag](#14-tag)
15. [File History](#15-file-history)
16. [Git Blame](#16-git-blame)
17. [Git Command Center](#17-git-command-center)
18. [Terminal Panel](#18-terminal-panel)
19. [Settings](#19-settings)
20. [AI Commit Message](#20-ai-commit-message)
21. [Error Handling & Edge Cases](#21-error-handling--edge-cases)
22. [Performance](#22-performance)
23. [UI/UX](#23-uiux)

---

## 1. Khởi Động Ứng Dụng

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 1.1 | Khởi động khi chưa có Git installed | Gỡ Git khỏi PATH → Mở app | Hiển thị thông báo lỗi rõ ràng: "Git not found. Please install Git and ensure it is in your PATH" |
| 1.2 | Khởi động lần đầu (chưa có repo nào) | Mở app lần đầu | Hiển thị **Repo Manager**, không có repo trong danh sách, không crash |
| 1.3 | Khởi động khi đã có repos saved | Mở app với repos đã lưu trước đó | Load settings thành công, hiển thị tabs cho các repo đã mở, activate đúng repo cuối cùng active |
| 1.4 | Khởi động khi repo path đã bị xóa | Xóa thư mục repo trên disk → Mở app | Hiển thị lỗi khi chọn repo đã xóa, không crash ứng dụng |

---

## 2. Quản Lý Repository

### 2.1 Thêm Repo

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 2.1.1 | Thêm repo hợp lệ | Nhập name + path → "Add Repo" | Repo xuất hiện trong danh sách, tự động active |
| 2.1.2 | Thêm repo bằng Browse | Click 📂 → Chọn thư mục | Path được điền, name tự động lấy từ folder name |
| 2.1.3 | Thêm repo không phải Git repo | Nhập path đến thư mục không có `.git` | Hiển thị lỗi "not a valid git repository" |
| 2.1.4 | Thêm repo trùng path | Thêm repo với path đã tồn tại | Hiển thị lỗi hoặc bỏ qua, không tạo trùng lặp |
| 2.1.5 | Thêm repo với path không tồn tại | Nhập path không có trên disk | Hiển thị lỗi rõ ràng, không crash |
| 2.1.6 | Thêm repo không có name | Để trống name → click "Add Repo" | Nút "Add Repo" bị disabled |

### 2.2 Xóa Repo

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 2.2.1 | Xóa repo không active | Click 🗑 trên repo không active | Repo bị xóa khỏi danh sách, settings cập nhật |
| 2.2.2 | Xóa repo active | Click 🗑 trên repo đang active | Repo bị xóa, tự chuyển sang repo khác hoặc hiện Repo Manager nếu hết repo |

### 2.3 Open/Close Repo

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 2.3.1 | Open repo từ Repo Manager | Click "Open" trên repo trong danh sách | Tab mới xuất hiện, workspace load đúng repo |
| 2.3.2 | Close repo tab | Click close (✕) trên tab | Tab bị xóa, nếu là active thì chuyển sang tab khác |
| 2.3.3 | Close tất cả tabs | Close lần lượt tất cả tabs | Hiển thị Repo Manager khi không còn tab nào |

---

## 3. Tab System

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 3.1 | Chuyển đổi giữa các tabs | Click vào các tab khác nhau | Workspace chuyển đổi đúng, state giữ nguyên cho mỗi repo |
| 3.2 | Tab hiển thị đúng tên repo | Mở nhiều repos | Mỗi tab hiển thị đúng tên repo |
| 3.3 | Nút "+" thêm repo | Click "+" | Mở Repo Manager |
| 3.4 | Active tab có visual highlight | Click vào tab | Tab active có style khác biệt so với tabs khác |
| 3.5 | Giữ state khi chuyển tab | Chỉnh sửa trong tab A → chuyển sang tab B → quay lại tab A | State của tab A được giữ nguyên (selected commit, scroll position, etc.) |

---

## 4. Commit Graph

### 4.1 Loading & Rendering

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 4.1.1 | Load graph khi mở repo | Mở repo → Tab Graph | Graph tự động load, hiển thị commits với branch/tag labels |
| 4.1.2 | Graph với repo rỗng (chưa có commit) | Mở repo mới chưa có commit | Hiển thị headers đúng, thông báo "No commits" hoặc tương tự, không crash |
| 4.1.3 | Infinite scroll (load more) | Scroll xuống cuối danh sách commits | Tự động load thêm 200 commits, spinner hiển thị đúng |
| 4.1.4 | Reload graph | Click nút Reload | Graph reload hoàn toàn, hiển thị dữ liệu mới nhất |
| 4.1.5 | Graph với nhiều branches | Mở repo có nhiều branches | Hiển thị đúng lines/lanes, màu sắc khác nhau cho mỗi branch |
| 4.1.6 | Graph hiển thị merge commits | Repo có merge commits | Hiển thị đúng kết nối giữa các branch tại merge point |

### 4.2 Column Management

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 4.2.1 | Resize columns | Kéo border giữa các columns | Column resize đúng, content reflow |
| 4.2.2 | Hiển thị đầy đủ columns | Kiểm tra tất cả columns | Branch/Tag, Graph, Commit Message, Author, Date/Time hiện đầy đủ |
| 4.2.3 | Headers hiển thị khi data rỗng | Repo rỗng hoặc lỗi load | Headers vẫn hiển thị đúng |

### 4.3 Commit Selection

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 4.3.1 | Click chọn commit | Click vào một commit row | Highlight row, hiển thị commit details (changed files, hash, message, author, date) |
| 4.3.2 | Chọn WIP row | Click vào "Working Changes" row | Hiển thị working changes summary, staged/unstaged files |
| 4.3.3 | Focus commit by hash | Sử dụng chức năng search/navigate | Scroll đến commit đúng và highlight |
| 4.3.4 | Hiển thị avatar | Kiểm tra cột Author | Hiển thị Gravatar hoặc fallback avatar cho mỗi author |

### 4.4 Commit Context Menu

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 4.4.1 | Right-click commit | Right-click trên commit | Menu hiển thị: Checkout, Cherry-pick, Revert, Reset, Create Branch, Create Tag, Copy Hash, Rebase onto |
| 4.4.2 | Checkout từ context menu | Right-click → Checkout | Xác nhận → checkout thành công, graph reload |
| 4.4.3 | Copy commit hash | Right-click → Copy Hash | Hash được copy vào clipboard |
| 4.4.4 | Create branch from commit | Right-click → Create Branch | Dialog tạo branch xuất hiện với base commit đúng |
| 4.4.5 | Create tag from commit | Right-click → Create Tag | Prompt nhập tag name → tag được tạo |

### 4.5 Stash Commit Context Menu

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 4.5.1 | Right-click stash commit | Right-click trên stash entry trong graph | Menu hiển thị: Apply, Pop, Drop, Edit Message, Create Patch |
| 4.5.2 | Apply stash from graph | Right-click stash → Apply | Stash applied thành công, graph reload |
| 4.5.3 | Pop stash from graph | Right-click stash → Pop | Stash popped và bị xóa khỏi stash list |
| 4.5.4 | Drop stash from graph | Right-click stash → Drop | Xác nhận → stash bị xóa |

### 4.6 Branch Context Menu (trong Graph)

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 4.6.1 | Right-click branch label | Right-click trên branch label trong graph | Menu hiển thị: Checkout, Merge, Rebase, Rename, Delete, Set Upstream, Push |
| 4.6.2 | Merge từ branch label | Right-click branch → Merge | Xác nhận → merge thực hiện, graph reload |
| 4.6.3 | Delete branch | Right-click → Delete | Xác nhận → branch bị xóa |
| 4.6.4 | Rename branch | Right-click → Rename | Prompt nhập tên mới → branch được đổi tên |

### 4.7 Changed Files trong Commit Details

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 4.7.1 | Xem changed files của commit | Chọn commit → Xem file list | Danh sách files thay đổi hiển thị đúng với status icons (A/M/D/R) |
| 4.7.2 | Click file xem diff | Click file trong changed files | Diff view hiển thị đúng base vs modified content |
| 4.7.3 | Right-click changed file | Right-click file | Context menu: Open File, Show History, Show Blame, Open in Editor, Show in Folder |
| 4.7.4 | Tree view / Flat view toggle | Toggle giữa tree và flat view | Files hiển thị đúng ở cả hai mode |
| 4.7.5 | Collapse/Expand directories | Click vào folder icon | Folder collapse/expand đúng |

---

## 5. Commit Panel

### 5.1 File Status

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 5.1.1 | Hiển thị staged files | Stage files → Xem Commit Panel | Files xuất hiện trong section "Staged" |
| 5.1.2 | Hiển thị unstaged files | Thay đổi files → Xem Commit Panel | Files xuất hiện trong section "Unstaged/Modified" |
| 5.1.3 | Hiển thị untracked files | Tạo file mới | File xuất hiện với status "?" |
| 5.1.4 | Auto-refresh status | Thay đổi file bên ngoài app | Status tự cập nhật (qua git-event) |
| 5.1.5 | Status hỗ trợ renamed files | `git mv` file | Hiển thị arrow `→` với old → new path |

### 5.2 Stage/Unstage Operations

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 5.2.1 | Stage single file | Click stage icon trên file | File chuyển từ unstaged → staged |
| 5.2.2 | Unstage single file | Click unstage icon trên staged file | File chuyển từ staged → unstaged |
| 5.2.3 | Stage all files | Click "Stage All" | Tất cả unstaged files chuyển sang staged |
| 5.2.4 | Unstage all files | Click "Unstage All" | Tất cả staged files chuyển sang unstaged |
| 5.2.5 | Stage single line | Trong diff view, click stage line | Chỉ line được chọn được stage |
| 5.2.6 | Unstage single line | Trong diff view, click unstage line | Chỉ line được chọn được unstage |
| 5.2.7 | Discard changes | Right-click file → Discard | Xác nhận → file revert về trạng thái HEAD |

### 5.3 Commit

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 5.3.1 | Commit với message | Stage files → Nhập message → Commit | Commit thành công, staged files clear, graph reload |
| 5.3.2 | Commit không có message | Click Commit khi message rỗng | Nút Commit disabled hoặc hiện lỗi |
| 5.3.3 | Commit không có staged files | Message nhập đủ nhưng chưa stage | Hiện lỗi "nothing to commit" |
| 5.3.4 | Amend commit | Tick checkbox "Amend" → Commit | Commit trước đó được amend |

### 5.4 File Context Menu

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 5.4.1 | Stash single file | Right-click file → Stash | File được stash riêng |
| 5.4.2 | Stash all changes | Click "Stash All" | Tất cả changes được stash |
| 5.4.3 | Open file in editor | Right-click → Open in Editor | Mở file trong editor được cấu hình (git config core.editor) |
| 5.4.4 | Show in folder | Right-click → Show in Folder | Mở file explorer tại vị trí file |
| 5.4.5 | Add to .gitignore | Right-click → Ignore File | File được thêm vào .gitignore |
| 5.4.6 | Delete file | Right-click → Delete | Xác nhận → file bị xóa |
| 5.4.7 | Create patch | Right-click → Create Patch | Patch file được tạo |

---

## 6. Diff View

### 6.1 Diff Modes

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 6.1.1 | Inline diff | Chọn file → Mode "Inline" | Hiển thị unified diff với added (green) / removed (red) lines |
| 6.1.2 | Side-by-side diff | Chọn file → Mode "Side-by-Side" | Hiển thị 2 panels: base bên trái, modified bên phải |
| 6.1.3 | Hunk diff | Chọn file → Mode "Hunk" | Hiển thị từng hunk với ±3 lines context |
| 6.1.4 | Switch giữa các modes | Toggle qua lại giữa 3 modes | Content hiển thị đúng ở mỗi mode, không mất data |

### 6.2 Diff Content

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 6.2.1 | Diff cho staged files | Chọn staged file | Diff hiển thị HEAD → Index |
| 6.2.2 | Diff cho unstaged files | Chọn unstaged file | Diff hiển thị Index → Working Tree |
| 6.2.3 | Diff cho new file (untracked) | Chọn untracked file | Hiển thị toàn bộ content là additions |
| 6.2.4 | Diff cho deleted file | Chọn deleted file | Hiển thị toàn bộ content cũ là deletions |
| 6.2.5 | Diff cho binary file | Chọn binary file (.png, .jpg) | Hiển thị thông báo "Binary file" thay vì crash |
| 6.2.6 | Diff cho large file | File > 500KB | Hiển thị warning về large file, không freeze UI |
| 6.2.7 | Diff cho commit (từ Graph) | Chọn commit → click file | Diff hiển thị parent commit → selected commit |

### 6.3 Encoding

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 6.3.1 | Change encoding | Chọn encoding khác (UTF-8, Shift_JIS, etc.) | File re-render với encoding mới |
| 6.3.2 | Per-file encoding override | Set encoding cho file cụ thể | Encoding được lưu và áp dụng mỗi lần mở file |
| 6.3.3 | Default encoding cho repo | Set repo default encoding | Tất cả files mới mở sử dụng encoding này |

---

## 7. Branch Explorer

### 7.1 Hiển thị

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 7.1.1 | Load branch list | Mở Workspace với tab Graph | Branch tree hiển thị ở sidebar |
| 7.1.2 | Tree view structure | Repo có branches: feature/auth, feature/ui | Hiển thị cấu trúc cây: feature → auth, ui |
| 7.1.3 | Hiển thị remote branches | Repo có remote tracking branches | Hiển thị remotes/origin/* trong tree |
| 7.1.4 | Current branch highlight | Branch hiện tại active | Branch hiện tại có visual indicator (bold, icon, color) |
| 7.1.5 | Search/Filter branches | Nhập search query | Filter tree hiển thị chỉ branches matching, auto expand |

### 7.2 Actions

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 7.2.1 | Click checkout branch | Click vào branch name | Xác nhận → switch thành công, graph reload |
| 7.2.2 | Checkout remote branch | Click remote branch | Xác nhận → tạo local branch tracking remote |
| 7.2.3 | Create branch | Click nút "+" | Dialog: nhập name, chọn base → branch tạo thành công |
| 7.2.4 | Context menu (right-click) | Right-click branch | Menu: Checkout, Merge, Rebase, Interactive Rebase, Delete, Rename, Set Upstream |
| 7.2.5 | Delete branch | Right-click → Delete → Xác nhận | Branch bị xóa, tree refresh |
| 7.2.6 | Rename branch | Right-click → Rename → Nhập tên mới | Branch được đổi tên |
| 7.2.7 | Delete remote branch | Right-click remote branch → Delete | Remote branch bị xóa |

---

## 8. Merge

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 8.1 | Merge không conflict (fast-forward) | Branch Explorer → Right-click → Merge → Xác nhận | Merge thành công, graph reload hiện merge commit |
| 8.2 | Merge không conflict (no-ff) | Branch có diverge nhưng không conflict | Merge thành công với merge commit |
| 8.3 | Merge có conflict | Merge branch có conflicting changes | UI detect conflict → chuyển sang Commit Panel, hiện conflict files |
| 8.4 | Abort merge | Đang merge conflict → click "Abort Merge" | Merge bị hủy, repo quay về trạng thái trước merge |
| 8.5 | Merge commit message | Successfully merge → commit message | Message mặc định "Merge branch 'xxx'" |

---

## 9. Rebase

### 9.1 Standard Rebase

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 9.1.1 | Rebase thành công (no conflicts) | Branch Explorer → Right-click → Rebase → Xác nhận | Rebase hoàn tất, graph reload, toast thông báo thành công |
| 9.1.2 | Rebase có conflict | Rebase branch có conflicting changes | UI hiện conflict, overlay buttons: Continue/Skip/Abort |
| 9.1.3 | Continue rebase | Resolve conflicts → click "Continue" | Rebase tiếp tục với commit tiếp theo |
| 9.1.4 | Skip rebase commit | Không resolve → click "Skip" | Commit hiện tại bị bỏ qua, chuyển sang commit tiếp theo |
| 9.1.5 | Abort rebase | Click "Abort" | Rebase hủy hoàn toàn, repo quay về trạng thái trước |
| 9.1.6 | Rebase progress | Rebase đang xử lý nhiều commits | Hiển thị progress: "Step X/Y" với commit message |

### 9.2 Rebase State Recovery

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 9.2.1 | App restart during rebase | Đóng app khi đang rebase → Mở lại | Detect rebase in-progress, hiện đúng status/buttons |
| 9.2.2 | Switch tab during rebase | Đang rebase → switch tab → quay lại | Rebase state được giữ nguyên |

---

## 10. Interactive Rebase

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 10.1 | Mở interactive rebase editor | Right-click branch → Interactive Rebase | Editor hiển thị danh sách commits với actions |
| 10.2 | Change commit action | Click dropdown → chọn pick/reword/edit/squash/fixup/drop | Action thay đổi, UI cập nhật |
| 10.3 | Reorder commits | Drag and drop commits | Thứ tự commits thay đổi |
| 10.4 | Apply interactive rebase | Click "Apply" | Rebase thực hiện theo plan, graph reload |
| 10.5 | Cancel editing | Click "Cancel" | Quay về trạng thái trước, không thay đổi gì |
| 10.6 | Interactive rebase có conflict | Apply rebase → gặp conflict | UI hiện conflict, cho phép resolve + continue |

---

## 11. Cherry-pick

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 11.1 | Cherry-pick thành công | Right-click commit → Cherry-pick → Xác nhận | Commit được cherry-pick, graph reload |
| 11.2 | Cherry-pick có conflict | Cherry-pick commit có conflicting changes | Detect conflict, hiện conflict UI |
| 11.3 | Cherry-pick nhiều commits | Cherry-pick lần lượt | Mỗi commit applied đúng thứ tự |

---

## 12. Conflict Resolution

### 12.1 Detection

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 12.1.1 | Detect merge conflicts | Merge gây conflict | UI hiện **Resolve Conflicts** button, commit panel hiển thị conflicted files |
| 12.1.2 | Detect rebase conflicts | Rebase gây conflict | RebaseProgress overlay hiện, files hiện status conflict |
| 12.1.3 | Conflict UI ẩn khi không có conflict | Repo clean, không merge/rebase | Không hiện conflict UI hoặc buttons |
| 12.1.4 | Detect cherry-pick conflicts | Cherry-pick gây conflict | UI detect và hiện conflict resolution |

### 12.2 Resolution

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 12.2.1 | Accept Ours | Click "Accept Ours" trên conflict file | File resolved với content Ours, marked as resolved |
| 12.2.2 | Accept Theirs | Click "Accept Theirs" | File resolved với content Theirs |
| 12.2.3 | Manual resolution | Click "Edit Manually" → Sửa content → Save | File saved với manual content, marked resolved |
| 12.2.4 | Merge Both | Click "Merge Both" | Editor opened với combined ours/theirs, cho phép edit |
| 12.2.5 | Resolve tất cả files | Lần lượt resolve từng file | Sau resolve hết → có thể continue merge/rebase |
| 12.2.6 | ConflictResolveModal | Trong CommitPanel, click conflict file | Modal hiện với 3 panes: Base, Ours, Theirs |

### 12.3 Operation State

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 12.3.1 | Operation state indicator | Đang merge/rebase/cherry-pick/revert | Banner hiện rõ: "Merge in progress", "Rebase in progress (step X/Y)", etc. |
| 12.3.2 | Abort operation | Click "Abort" trên operation banner | Operation bị hủy, repo clean |
| 12.3.3 | Continue sau resolve | Resolve hết conflicts → Continue | Operation tiếp tục thành công |

---

## 13. Stash

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 13.1 | Stash file đơn lẻ | Right-click unstaged file → Stash | File được stash riêng |
| 13.2 | Stash all changes | Click "Stash All" | Tất cả working changes được stash |
| 13.3 | Apply stash | Right-click stash entry → Apply | Changes từ stash applied, stash vẫn còn |
| 13.4 | Pop stash | Right-click stash → Pop | Changes applied, stash bị xóa |
| 13.5 | Drop stash | Right-click stash → Drop → Xác nhận | Stash bị xóa vĩnh viễn |
| 13.6 | Edit stash message | Right-click stash → Edit Message | Prompt nhập message mới → message cập nhật |
| 13.7 | Create patch from stash | Right-click stash → Create Patch | Patch file được tạo từ stash content |
| 13.8 | Stash khi có conflict | Stash trong khi merge in progress | Hiển thị lỗi hoặc xử lý đúng |

---

## 14. Tag

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 14.1 | Create tag | Right-click commit → Create Tag → Nhập name | Tag được tạo, hiển thị trong graph |
| 14.2 | Create tag with message | Tạo tag annotated | Tag được tạo với message |
| 14.3 | Tag hiển thị trong graph | Có tags trong repo | Tags hiển thị cạnh branch labels trong graph |

---

## 15. File History

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 15.1 | Search file | Tab History → Nhập search query | Dropdown hiển thị matching files (debounced 300ms) |
| 15.2 | Select file | Chọn file từ search results | Load commit history cho file đó |
| 15.3 | Commit list | File selected → Xem danh sách commits | Hiển thị hash, author, date, message cho mỗi commit |
| 15.4 | View diff tại commit | Click commit trong history | Side-by-side diff hiển thị: file tại parent vs file tại commit |
| 15.5 | Diff modes trong history | Toggle Inline/Side-by-Side/Hunk | Các modes hoạt động đúng |
| 15.6 | Reload history | Click "Reload" | History refresh |
| 15.7 | Navigate to commit | Right-click → mở history từ CommitGraph | Tab chuyển sang History, file path và commit pre-selected |
| 15.8 | Empty history | File chưa có commit | Hiển thị thông báo "No history" thay vì crash |

---

## 16. Git Blame

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 16.1 | Open blame view | Tab Blame → search file → select | Blame data hiển thị: mỗi line kèm commit hash, author, date |
| 16.2 | Line-level blame info | Hover hoặc click line | Hiển thị commit detail cho line đó |
| 16.3 | Blame cho file mới (chưa commit) | Blame untracked file | Hiển thị lỗi rõ ràng hoặc "file not in git" |
| 16.4 | Navigate to commit từ blame | Click commit hash trong blame | Chuyển về graph tab, focus commit đó |

---

## 17. Git Command Center

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 17.1 | Chạy Git Status | Sidebar → Commands → Git Status → Run | Output hiện trạng thái working tree |
| 17.2 | Chạy Git Pull | Git Pull → Run | Pull thành công, hiện output |
| 17.3 | Chạy Git Push | Git Push → Run | Push thành công, hiện output |
| 17.4 | Chạy Git Fetch | Git Fetch → Run | Fetch from remote |
| 17.5 | Chạy Git Commit | Git Commit → Nhập message → Run | Commit với message |
| 17.6 | Chạy Git Add All | Git Add All → Run | Stage tất cả files |
| 17.7 | Chạy Git Checkout | Git Checkout → Nhập branch → Run | Switch branch |
| 17.8 | Chạy Git Branch List | Git Branch List → Run | Hiển thị danh sách branches |
| 17.9 | Chạy Git Log | Git Log → Run | Hiển thị 50 commits gần nhất |
| 17.10 | Lỗi command | Chạy command lỗi (e.g. push khi chưa có remote) | Hiển thị error output, authentication hint nếu cần |
| 17.11 | Không có repo active | Mở Command Center khi chưa chọn repo | Hiện "Please select a repository first" |
| 17.12 | Copy output | Click "📋 Copy" sau khi chạy command | Output copied vào clipboard |
| 17.13 | Clear output | Click "🧹 Clear" | Output area cleared |

---

## 18. Terminal Panel

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 18.1 | Start terminal session | Tab Terminal | Terminal session started, prompt hiện |
| 18.2 | Run command | Nhập `git status` → Enter | Output hiển thị trong terminal area |
| 18.3 | Auto-scroll | Nhiều output | Terminal auto-scroll xuống cuối |
| 18.4 | Session per repo | Switch giữa các repo tabs | Mỗi repo có terminal session riêng |
| 18.5 | Terminal disabled khi không running | Session chưa start hoặc lỗi | Input disabled, placeholder "Terminal not ready" |
| 18.6 | Empty command | Nhấn Enter khi input rỗng | Không gửi, không crash |

---

## 19. Settings

### 19.1 AI Provider Configuration

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 19.1.1 | Chọn Gemini provider | Radio button "Google Gemini" | Provider chuyển sang Gemini |
| 19.1.2 | Chọn OpenRouter provider | Radio button "OpenRouter" | Provider chuyển sang OpenRouter |
| 19.1.3 | Save Gemini API token | Nhập token → Save | Token được save, model list tự load |
| 19.1.4 | Save OpenRouter API token | Nhập token → Save | Token save, models load |
| 19.1.5 | Clear API token | Click Clear | Token xóa, models list cleared |
| 19.1.6 | Chọn model | Dropdown chọn model | Model saved |
| 19.1.7 | Invalid token | Nhập token sai → Save | Error hiển thị "Failed to load models" |
| 19.1.8 | Load model list | Token hợp lệ → Load models | Danh sách models hiển thị trong dropdown |

### 19.2 Commit Prompts

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 19.2.1 | Save global prompt | Nhập prompt → Save Global | Prompt saved, áp dụng cho tất cả repos |
| 19.2.2 | Save repo-specific prompt | Nhập prompt → Save Repo | Prompt saved, chỉ áp dụng cho repo hiện tại |
| 19.2.3 | Default prompt | Xem default prompt | Hiển thị expert prompt mặc định |

### 19.3 File Exclusions

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 19.3.1 | Add exclusion pattern | Nhập pattern (e.g. `node_modules`) → Add | Pattern thêm vào list |
| 19.3.2 | Remove exclusion | Click remove trên pattern | Pattern xóa khỏi list |
| 19.3.3 | Exclusion hoạt động | Thêm exclusion → kiểm tra file list | Files matching pattern bị ẩn khỏi status |

### 19.4 Repository Encoding

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 19.4.1 | Set default encoding | Chọn encoding cho repo | Encoding saved, áp dụng khi mở files |
| 19.4.2 | Per-file encoding | Đặt encoding riêng cho file | File hiển thị đúng với encoding đã set |

### 19.5 Repo Filter

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 19.5.1 | Set graph filter | Nhập filter cho repo | Filter saved, graph chỉ hiện commits matching |

---

## 20. AI Commit Message

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 20.1 | Generate message (Gemini) | Stage files → Click "Generate AI Message" (Gemini configured) | AI generate commit message dựa trên diff, message điền vào ô |
| 20.2 | Generate message (OpenRouter) | Stage files → Generate (OpenRouter configured) | AI generate message via OpenRouter |
| 20.3 | Generate khi chưa cấu hình AI | Click Generate khi chưa set token | Error: "No AI provider configured" hoặc tương tự |
| 20.4 | Generate khi chưa stage files | Click Generate khi chưa stage | Error: "No staged changes" |
| 20.5 | Large diff truncation | Stage many files → Generate | Diff được truncate đến limit (40K chars cho Gemini), message vẫn generate |
| 20.6 | Sanitize output | AI trả về message có markdown/extra formatting | Message được sanitize, bỏ markdown decorations |
| 20.7 | Network error | Generate khi không có internet | Error hiển thị rõ ràng: network/API error |

---

## 21. Error Handling & Edge Cases

### 21.1 Git Command Errors

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 21.1.1 | Push khi chưa có remote | Git Push trên repo chưa add remote | Error message rõ ràng |
| 21.1.2 | Push khi cần authentication | Push cần credentials | Hiện thông báo authentication required |
| 21.1.3 | Pull khi có uncommitted changes | Pull khi working tree dirty | Error hoặc prompt to stash |
| 21.1.4 | Checkout khi có uncommitted changes | Try checkout branch khác | Error khi có conflicts |
| 21.1.5 | Command timeout | Git command chạy quá lâu | Timeout handled gracefully, không freeze UI |

### 21.2 Edge Cases

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 21.2.1 | Repo bị xóa đang sử dụng | Xóa .git folder khi app đang mở repo | Hiện error, không crash |
| 21.2.2 | Nhiều app instances | Mở 2 instances cùng repo | Không conflict, mỗi instance hoạt động độc lập |
| 21.2.3 | File permissions | File read-only | Error hiển thị khi cố write |
| 21.2.4 | Special characters trong paths | Repo path/file name có unicode, spaces | Hoạt động đúng, không bị encoding issues |
| 21.2.5 | Rất nhiều files | Repo có >1000 changed files | Performance OK, không freeze, có thể lazy load |
| 21.2.6 | Deep nested directories | File paths rất dài | Hiển thị đúng, truncate có tooltip |
| 21.2.7 | Network disconnect | Mất mạng khi đang fetch/pull/push | Error handled, UI responsive |
| 21.2.8 | Concurrent operations | Trigger multiple git operations đồng thời | Xử lý đúng, không corrupt repo state |

### 21.3 State Management

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 21.3.1 | No active repository | Đóng tất cả repos | Graph không load, commands disabled, conflict UI ẩn |
| 21.3.2 | Switch repo | Chuyển từ repo A sang repo B | State reset đúng cho repo mới |
| 21.3.3 | Settings persistence | Thay đổi settings → Đóng app → Mở lại | Settings được giữ nguyên |

---

## 22. Performance

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 22.1 | Graph load lớn (>10k commits) | Mở repo lớn với 10k+ commits | Load 200 đầu nhanh (<3s), infinite scroll mượt |
| 22.2 | Status refresh speed | Thay đổi file → Kiểm tra status cập nhật | Cập nhật <1s |
| 22.3 | Diff render speed | Mở diff file lớn (>5000 lines) | Render <2s, không freeze |
| 22.4 | Branch list load | Repo có >100 branches | Load <1s |
| 22.5 | UI responsive | Thực hiện git operation → Interact UI | UI không bị freeze khi git command đang chạy |
| 22.6 | Memory usage | Mở app dài thời gian (>1hr) | Memory không tăng liên tục (no memory leaks) |
| 22.7 | Graph debounce | Git events liên tục (nhiều saves nhanh) | Graph chỉ reload khi debounce timer hết (150ms) |

---

## 23. UI/UX

### 23.1 Layout

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 23.1.1 | Resizable panels | Kéo resize border giữa sidebar ↔ main area | Panels resize mượt (min 220px, max 450px) |
| 23.1.2 | Responsive window sizes | Resize cửa sổ app | Layout responsive, không bị overlap |
| 23.1.3 | Dark theme | Giao diện mặc định | Consistent dark theme (#0d1117 background, #c9d1d9 text) |

### 23.2 Toast Notifications

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 23.2.1 | Success toast | Thực hiện operation thành công | Toast success hiện và tự biến mất |
| 23.2.2 | Error toast | Operation thất bại | Toast error hiện với message rõ ràng |
| 23.2.3 | Multiple toasts | Nhiều operations liên tiếp | Toasts stack đúng, không overlap |

### 23.3 Confirmation Dialogs

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 23.3.1 | Destructive action confirm | Delete branch, discard changes | Dialog confirm hiện trước khi thực hiện |
| 23.3.2 | Cancel confirm | Click Cancel trên dialog | Action bị hủy, không thay đổi gì |
| 23.3.3 | HTML message support | Confirm dialog với HTML content | HTML render đúng (branch names bold, etc.) |

### 23.4 Loading States

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 23.4.1 | Graph loading spinner | Load graph | Spinner/loading indicator hiện khi đang load |
| 23.4.2 | Diff loading | Load diff file | Loading indicator hiện |
| 23.4.3 | Command running | Run git command | "Running..." indicator hiện |
| 23.4.4 | Status loading | Refresh status | Loading indicator hiện |

### 23.5 Empty States

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 23.5.1 | No repos | Đóng tất cả repos | Repo Manager hiện, hướng dẫn thêm repo |
| 23.5.2 | No staged files | Commit panel khi chưa stage | Section staged trống, thông báo |
| 23.5.3 | No commits | Repo mới | Graph hiện headers, message "No commits" |
| 23.5.4 | No changes | Working tree clean | Message "Working tree clean" |

### 23.6 Keyboard Navigation

| # | Kịch bản | Bước thực hiện | Kết quả mong đợi |
|---|----------|----------------|-------------------|
| 23.6.1 | Enter để submit | Nhập commit message → Enter | Commit được thực hiện (nếu có staged files) |
| 23.6.2 | Escape đóng menu | Mở context menu → Escape | Menu đóng |
| 23.6.3 | Search input navigation | History search → Arrow keys | Navigate qua search results |

---

## Checklist Tổng Hợp

> [!IMPORTANT]
> Trước mỗi release, đảm bảo chạy qua **tất cả** các test case ở trên. Các test case đặc biệt quan trọng được đánh dấu dưới đây:

### 🔴 Critical (Phải pass)
- [ ] 1.1 - App khởi động khi không có Git
- [ ] 2.1.3 - Thêm repo không hợp lệ
- [ ] 4.1.2 - Graph với repo rỗng
- [ ] 5.3.1 - Commit flow cơ bản
- [ ] 8.3 - Merge có conflict
- [ ] 9.1.2 - Rebase có conflict
- [ ] 12.2.1-12.2.4 - Tất cả phương thức resolve conflict
- [ ] 21.1.5 - Command timeout không freeze UI
- [ ] 21.3.1 - No active repository → commands disabled
- [ ] 22.5 - UI responsive khi git chạy

### 🟡 Important (Nên pass)
- [ ] 3.5 - State preservation khi switch tabs
- [ ] 5.2.5-5.2.6 - Stage/Unstage single line
- [ ] 6.1.1-6.1.3 - Tất cả diff modes
- [ ] 9.2.1 - Rebase state recovery sau restart
- [ ] 10.1-10.6 - Interactive rebase flow đầy đủ
- [ ] 15.1-15.5 - File History flow
- [ ] 19.1.1-19.1.8 - AI configuration
- [ ] 20.1-20.2 - AI commit message generation

### 🟢 Nice to Have
- [ ] 22.6 - Memory leak check
- [ ] 22.1 - Large repo performance
- [ ] 23.6.1-23.6.3 - Keyboard navigation
