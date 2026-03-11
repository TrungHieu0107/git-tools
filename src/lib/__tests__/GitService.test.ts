import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { GitService } from "../GitService";
import type { AppSettings, FileStatus, GitOperationState } from "../GitService";

const mockInvoke = vi.mocked(invoke);

describe("GitService.getSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls cmd_get_settings with no args", async () => {
    const settings: AppSettings = {
      repos: [],
      active_repo_id: null,
      open_repo_ids: [],
      excluded_files: [],
      repo_filters: {},
    };
    mockInvoke.mockResolvedValue(settings);

    const result = await GitService.getSettings();

    expect(mockInvoke).toHaveBeenCalledWith("cmd_get_settings", undefined);
    expect(result).toEqual(settings);
  });

  it("throws when invoke rejects", async () => {
    mockInvoke.mockRejectedValue(new Error("boom"));
    await expect(GitService.getSettings()).rejects.toThrow("boom");
  });
});

describe("GitService.addRepo", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls cmd_add_repo with name and path", async () => {
    const updated = { repos: [], active_repo_id: null, open_repo_ids: [], excluded_files: [], repo_filters: {} };
    mockInvoke.mockResolvedValue(updated);

    const result = await GitService.addRepo("myRepo", "/path");

    expect(mockInvoke).toHaveBeenCalledWith("cmd_add_repo", { name: "myRepo", path: "/path" });
    expect(result).toEqual(updated);
  });
});

describe("GitService.setActiveRepo", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls cmd_set_active_repo with id", async () => {
    const updated = { repos: [], active_repo_id: "123", open_repo_ids: [], excluded_files: [], repo_filters: {} };
    mockInvoke.mockResolvedValue(updated);

    const result = await GitService.setActiveRepo("123");

    expect(mockInvoke).toHaveBeenCalledWith("cmd_set_active_repo", { id: "123" });
    expect(result).toEqual(updated);
  });
});

describe("GitService.commit", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls cmd_git_commit with message and repoPath", async () => {
    const res = { success: true, stdout: "ok", stderr: "" };
    mockInvoke.mockResolvedValue(res);

    const result = await GitService.commit("msg", "/repo");

    expect(mockInvoke).toHaveBeenCalledWith("cmd_git_commit", { message: "msg", repoPath: "/repo" });
    expect(result).toEqual(res);
  });

  it("passes camelCase args (not snake_case)", async () => {
    mockInvoke.mockResolvedValue({ success: true });
    await GitService.commit("hello", "/r");
    const call = mockInvoke.mock.calls[0]?.[1];
    expect(call).toEqual({ message: "hello", repoPath: "/r" });
    expect(call).not.toHaveProperty("repo_path");
  });

  it("returns success=false on git error", async () => {
    mockInvoke.mockResolvedValue({ success: false, stderr: "oops" });
    const result = await GitService.commit("msg", "/repo");
    expect(result.success).toBe(false);
    expect(result.stderr).toBe("oops");
  });
});

describe("GitService.getStatusFiles", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls cmd_get_status_files with repoPath", async () => {
    const files: FileStatus[] = [{ path: "a", status: "M", staged: false, ignoredByApp: false }];
    mockInvoke.mockResolvedValue(files);

    const result = await GitService.getStatusFiles("/repo");

    expect(mockInvoke).toHaveBeenCalledWith("cmd_get_status_files", { repoPath: "/repo" });
    expect(result).toEqual(files);
  });

  it("returns empty array when no changes", async () => {
    mockInvoke.mockResolvedValue([]);
    const result = await GitService.getStatusFiles("/repo");
    expect(result).toEqual([]);
  });
});

describe("GitService.getDiffFile", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls cmd_get_diff_file with path, staged, repoPath", async () => {
    mockInvoke.mockResolvedValue("diff");
    await GitService.getDiff("file.txt", true, "/repo");
    expect(mockInvoke).toHaveBeenCalledWith("cmd_get_diff_file", {
      path: "file.txt",
      staged: true,
      repoPath: "/repo",
    });
  });

  it("passes staged=false for unstaged files", async () => {
    mockInvoke.mockResolvedValue("diff");
    await GitService.getDiff("file.txt", false, "/repo");
    expect(mockInvoke).toHaveBeenCalledWith("cmd_get_diff_file", {
      path: "file.txt",
      staged: false,
      repoPath: "/repo",
    });
  });

  it("returns diff string", async () => {
    mockInvoke.mockResolvedValue("--- a\n+++ b\n");
    const diff = await GitService.getDiff("file.txt", false, "/repo");
    expect(diff).toBe("--- a\n+++ b\n");
  });
});

describe("GitService.getConflictFile", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls cmd_get_conflict_file with path and repoPath", async () => {
    const conflict = { base: "b", ours: "o", theirs: "t" };
    mockInvoke.mockResolvedValue(conflict);

    const result = await GitService.getConflictFile("file.txt", "/repo");

    expect(mockInvoke).toHaveBeenCalledWith("cmd_get_conflict_file", { path: "file.txt", repoPath: "/repo" });
    expect(result).toEqual(conflict);
  });

  it("throws when file not found", async () => {
    mockInvoke.mockRejectedValue(new Error("not found"));
    await expect(GitService.getConflictFile("missing", "/repo")).rejects.toThrow("not found");
  });
});

describe("GitService.checkConflictState", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns true when conflicts exist", async () => {
    mockInvoke.mockResolvedValue(true);
    await expect(GitService.checkConflictState("/repo")).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("cmd_check_conflict_state", { repoPath: "/repo" });
  });

  it("returns false when no conflicts", async () => {
    mockInvoke.mockResolvedValue(false);
    await expect(GitService.checkConflictState("/repo")).resolves.toBe(false);
  });
});

describe("GitService.getOperationState", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns GitOperationState with all fields", async () => {
    const state: GitOperationState = {
      isMerging: true,
      isRebasing: false,
      isCherryPicking: false,
      isReverting: false,
      hasConflicts: true,
      conflictPaths: ["a"],
      oursCommit: "1",
      theirsCommit: "2",
      oursBranch: "main",
      theirsBranch: "feature",
      rebaseCurrent: 1,
      rebaseTotal: 2,
      rebaseMessage: "msg",
    };
    mockInvoke.mockResolvedValue(state);

    const result = await GitService.getOperationState("/repo");

    expect(mockInvoke).toHaveBeenCalledWith("cmd_get_operation_state", { repoPath: "/repo" });
    expect(result).toEqual(state);
  });
});

describe("GitService.merge", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls cmd_git_merge with branch and repoPath", async () => {
    mockInvoke.mockResolvedValue({ success: true, stdout: "", stderr: "" });

    const result = await GitService.merge("feature", "/repo");

    expect(mockInvoke).toHaveBeenCalledWith("cmd_git_merge", { branch: "feature", repoPath: "/repo" });
    expect(result.success).toBe(true);
  });

  it("returns failure result (not throw) for conflict merge", async () => {
    mockInvoke.mockResolvedValue({ success: false, stdout: "", stderr: "conflict" });
    const result = await GitService.merge("feature", "/repo");
    expect(result.success).toBe(false);
    expect(result.stderr).toBe("conflict");
  });
});
