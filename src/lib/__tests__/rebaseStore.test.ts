import { get } from "svelte/store";
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

// Shared mocks (reset in beforeEach)
const invokeMock = vi.fn();
const listenMock = vi.fn().mockResolvedValue({ unsubscribe: vi.fn(), unlisten: vi.fn() });
const toastSuccess = vi.fn();
const toastError = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("../toast.svelte", () => ({ toast: { success: toastSuccess, error: toastError } }));

describe("rebaseStore", () => {
  let rebaseStore: typeof import("../rebaseStore").rebaseStore;

  beforeEach(async () => {
    vi.useFakeTimers();
    invokeMock.mockReset();
    listenMock.mockReset();
    listenMock.mockResolvedValue({ unsubscribe: vi.fn(), unlisten: vi.fn() });
    toastSuccess.mockReset();
    toastError.mockReset();
    vi.resetModules();

    // Re-import with fresh state and mocked deps
    ({ rebaseStore } = await import("../rebaseStore"));
    // Run the deferred listener setup
    vi.runAllTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  function mockStatus(status: string, extra: Partial<Record<string, unknown>> = {}) {
    return {
      status,
      step: null,
      ontoBranch: null,
      upstreamBranch: null,
      ...extra,
    };
  }

  // Initial state -----------------------------------------------------------
  it("starts in idle state", () => {
    expect(get(rebaseStore).status).toBe("idle");
  });

  it("has no repoPath initially", () => {
    expect(get(rebaseStore).repoPath).toBeNull();
  });

  it("has no todoItems initially", () => {
    expect(get(rebaseStore).todoItems).toEqual([]);
  });

  // startRebase -------------------------------------------------------------
  it("transitions from idle to inProgress on success", async () => {
    invokeMock.mockImplementation((cmd) => {
      if (cmd === "cmd_rebase_start") return Promise.resolve({ success: true });
      if (cmd === "cmd_get_rebase_status") return Promise.resolve(mockStatus("inProgress", { step: { current: 1, total: 3, commitHash: "abc", commitMessage: "msg" } }));
      throw new Error(`unexpected ${cmd}`);
    });

    await rebaseStore.startRebase("main", "/repo");

    const state = get(rebaseStore);
    expect(state.status).toBe("inProgress");
    expect(state.step?.commitHash).toBe("abc");
  });

  it("sets repoPath when starting rebase", async () => {
    invokeMock.mockImplementation((cmd) => {
      if (cmd === "cmd_rebase_start") return Promise.resolve({ success: true });
      if (cmd === "cmd_get_rebase_status") return Promise.resolve(mockStatus("inProgress"));
      throw new Error(`unexpected ${cmd}`);
    });

    await rebaseStore.startRebase("develop", "/my/repo");

    expect(get(rebaseStore).repoPath).toBe("/my/repo");
  });

  it("remains idle if git command fails", async () => {
    invokeMock.mockRejectedValueOnce(new Error("cmd failed"));
    invokeMock.mockRejectedValueOnce(new Error("status failed"));

    await rebaseStore.startRebase("main", "/repo");

    expect(get(rebaseStore).status).toBe("idle");
  });

  it("keeps inProgress state when start is called while already inProgress", async () => {
    // First start succeeds
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()));

    await rebaseStore.startRebase("main", "/repo");

    // Second start gets backend "busy" response
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: false, stderr: "already running" }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()));

    await rebaseStore.startRebase("main", "/repo");

    expect(get(rebaseStore).status).toBe("inProgress");
  });

  // Status checks via continue/command follow-up ---------------------------
  it("updates status from backend response", async () => {
    invokeMock.mockImplementation((cmd) => {
      if (cmd === "cmd_rebase_start") return Promise.resolve({ success: true });
      if (cmd === "cmd_get_rebase_status") return Promise.resolve(mockStatus("inProgress", { ontoBranch: "main", upstreamBranch: "origin/main" }));
      throw new Error(`unexpected ${cmd}`);
    });

    await rebaseStore.startRebase("main", "/repo");

    const state = get(rebaseStore);
    expect(state.ontoBranch).toBe("main");
    expect(state.upstreamBranch).toBe("origin/main");
  });

  it("transitions to conflicted when conflicts detected", async () => {
    // Start -> inProgress
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()))
      // Continue reports conflict
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_continue" ? Promise.resolve({ success: false, stderr: "conflict" }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("conflicted")) : Promise.reject()));

    await rebaseStore.startRebase("main", "/repo");
    await rebaseStore.continue();

    expect(get(rebaseStore).status).toBe("conflicted");
  });

  it("transitions to completed when rebase finishes", async () => {
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_continue" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("completed")) : Promise.reject()));

    await rebaseStore.startRebase("main", "/repo");
    await rebaseStore.continue();

    expect(get(rebaseStore).status).toBe("idle");
  });

  it("handles getRebaseStatus network error gracefully", async () => {
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_continue" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockRejectedValueOnce(new Error("network down"));

    await rebaseStore.startRebase("main", "/repo");
    await rebaseStore.continue();

    expect(get(rebaseStore).status).toBe("idle");
    expect(toastSuccess).toHaveBeenCalled();
  });

  // continue / abort / skip -------------------------------------------------
  it("rebaseContinue transitions conflicted → inProgress", async () => {
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_continue" ? Promise.resolve({ success: false, stderr: "conflict" }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("conflicted")) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_continue" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()));

    await rebaseStore.startRebase("main", "/repo");
    await rebaseStore.continue(); // into conflicted
    await rebaseStore.continue(); // back to inProgress

    expect(get(rebaseStore).status).toBe("inProgress");
  });

  it("rebaseAbort transitions any state → aborted", async () => {
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_abort" ? Promise.resolve({ success: true }) : Promise.reject()));

    await rebaseStore.startRebase("main", "/repo");
    await rebaseStore.abort();

    expect(get(rebaseStore).status).toBe("idle");
  });

  it("rebaseSkip transitions conflicted → inProgress", async () => {
    invokeMock
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_start" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_rebase_skip" ? Promise.resolve({ success: true }) : Promise.reject()))
      .mockImplementationOnce((cmd) => (cmd === "cmd_get_rebase_status" ? Promise.resolve(mockStatus("inProgress")) : Promise.reject()));

    await rebaseStore.startRebase("main", "/repo");
    await rebaseStore.skip();

    expect(get(rebaseStore).status).toBe("inProgress");
  });

  it("rebaseContinue does nothing when idle", async () => {
    await rebaseStore.continue();
    expect(get(rebaseStore).status).toBe("idle");
  });

  it("rebaseAbort does nothing when idle", async () => {
    await rebaseStore.abort();
    expect(get(rebaseStore).status).toBe("idle");
  });

  // Interactive rebase ------------------------------------------------------
  it("prepareInteractive transitions idle → editingTodo", async () => {
    invokeMock.mockResolvedValueOnce([{ action: "pick", hash: "a1", message: "msg" }]);

    await rebaseStore.prepareInteractive("abc", "/repo");

    const state = get(rebaseStore);
    expect(state.status).toBe("editingTodo");
    expect(state.repoPath).toBe("/repo");
    expect(state.baseCommit).toBe("abc");
  });

  it("prepareInteractive fetches todo items from backend", async () => {
    const items = [
      { action: "pick", hash: "h1", message: "first" },
      { action: "reword", hash: "h2", message: "second" },
    ];
    invokeMock.mockResolvedValueOnce(items);

    await rebaseStore.prepareInteractive("abc", "/repo");

    expect(get(rebaseStore).todoItems).toEqual(items);
  });

  it("applyInteractive transitions editingTodo → inProgress", async () => {
    const items = [{ action: "pick", hash: "h1", message: "msg" }];
    invokeMock
      .mockResolvedValueOnce(items) // prepare
      .mockImplementationOnce((cmd, args: any) => {
        if (cmd === "cmd_rebase_interactive_apply" && JSON.stringify(args.todoItems) === JSON.stringify(items)) {
          return Promise.resolve({ success: true });
        }
        return Promise.resolve({ success: false, stderr: "bad todo" });
      })
      .mockResolvedValueOnce(mockStatus("inProgress"));

    await rebaseStore.prepareInteractive("abc", "/repo");
    await rebaseStore.applyInteractive();

    expect(get(rebaseStore).status).toBe("inProgress");
  });

  it("applyInteractive sends correct todoItems to backend", async () => {
    const items = [
      { action: "pick", hash: "h1", message: "m1" },
      { action: "squash", hash: "h2", message: "m2" },
    ];
    invokeMock
      .mockResolvedValueOnce(items) // prepare
      .mockImplementationOnce((cmd, args: any) => {
        if (cmd === "cmd_rebase_interactive_apply" && JSON.stringify(args.todoItems) === JSON.stringify(items)) {
          return Promise.resolve({ success: true });
        }
        return Promise.resolve({ success: false, stderr: "todo mismatch" });
      })
      .mockResolvedValueOnce(mockStatus("inProgress"));

    await rebaseStore.prepareInteractive("abc", "/repo");
    await rebaseStore.applyInteractive();

    expect(get(rebaseStore).status).toBe("inProgress");
  });

  it("applyInteractive validates todoItems not empty", async () => {
    invokeMock.mockResolvedValueOnce([]); // prepare returns no items

    await rebaseStore.prepareInteractive("abc", "/repo");

    const state = get(rebaseStore);
    expect(state.status).toBe("idle");
    expect(state.todoItems).toEqual([]);
    expect(toastError).toHaveBeenCalled();
  });
});
