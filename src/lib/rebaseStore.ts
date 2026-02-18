import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { GitCommandResult } from "./GitService";
import { toast } from "./toast.svelte";

export type RebaseStatus = "idle" | "inProgress" | "conflicted" | "editingTodo" | "completed" | "aborted";

export interface RebaseStepInfo {
  current: number;
  total: number;
  commitHash: string;
  commitMessage: string;
}

export interface RebaseTodoItem {
  action: "pick" | "reword" | "edit" | "squash" | "fixup" | "drop";
  hash: string;
  message: string;
}

export interface FullRebaseStatus {
  status: string;
  step: RebaseStepInfo | null;
  ontoBranch: string | null;
  upstreamBranch: string | null;
}

export interface RebaseState {
  status: RebaseStatus;
  step: RebaseStepInfo | null;
  ontoBranch: string | null;
  upstreamBranch: string | null;
  todoItems: RebaseTodoItem[];
  baseCommit: string | null;
  repoPath: string | null;
  isPolling: boolean;
}

const initialState: RebaseState = {
  status: "idle",
  step: null,
  ontoBranch: null,
  upstreamBranch: null,
  todoItems: [],
  baseCommit: null,
  repoPath: null,
  isPolling: false,
};

function createRebaseStore() {
  const { subscribe, set, update } = writable<RebaseState>(initialState);

  let pollInterval: any = null;

  async function pollStatus() {
    const state = get(rebaseStore);
    if (!state.repoPath) return;

    try {
      const status: FullRebaseStatus = await invoke("cmd_get_rebase_status", { repoPath: state.repoPath });
      
      const mappedStatus = status.status as RebaseStatus;
      
      update(s => ({
        ...s,
        status: mappedStatus,
        step: status.step,
        ontoBranch: status.ontoBranch,
        upstreamBranch: status.upstreamBranch,
      }));

      if (mappedStatus === "idle" || mappedStatus === "completed" || mappedStatus === "aborted") {
        stopPolling();
      }
    } catch (e) {
      console.error("Failed to poll rebase status", e);
      stopPolling();
    }
  }

  function startPolling() {
    if (pollInterval) return;
    update(s => ({ ...s, isPolling: true }));
    pollStatus();
    pollInterval = setInterval(pollStatus, 2000);
  }

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
    update(s => ({ ...s, isPolling: false }));
  }

  /**
   * After a rebase command, check the actual repo state.
   * This handles all outcomes: clean completion, conflicts, and in-progress states.
   */
  async function checkRebaseStateAfterCommand(
    res: GitCommandResult,
    repoPath: string,
    operationName: string
  ) {
    if (res.success) {
      // Command succeeded — check if rebase is fully done or still going
      try {
        const status: FullRebaseStatus = await invoke("cmd_get_rebase_status", { repoPath });
        const mapped = status.status as RebaseStatus;
        
        if (mapped === "idle" || mapped === "completed") {
          toast.success(`${operationName} completed successfully`);
          update(s => ({ ...s, status: "idle", step: null }));
          stopPolling();
        } else {
          // Still in progress (multi-step rebase)
          update(s => ({
            ...s,
            status: mapped,
            step: status.step,
            ontoBranch: status.ontoBranch,
            upstreamBranch: status.upstreamBranch,
          }));
          startPolling();
        }
      } catch {
        // Couldn't check status — assume done
        toast.success(`${operationName} completed`);
        update(s => ({ ...s, status: "idle", step: null }));
        stopPolling();
      }
    } else {
      // Command failed (conflict or error) — always check real rebase state
      try {
        const status: FullRebaseStatus = await invoke("cmd_get_rebase_status", { repoPath });
        const mapped = status.status as RebaseStatus;

        if (mapped === "conflicted") {
          toast.error("Rebase encountered conflicts. Resolve them in the Commit Panel to continue.");
          update(s => ({
            ...s,
            status: "conflicted",
            step: status.step,
            ontoBranch: status.ontoBranch,
            upstreamBranch: status.upstreamBranch,
          }));
          startPolling();
        } else if (mapped === "inProgress") {
          // Rebase in progress (not conflicted) — might need user attention
          update(s => ({
            ...s,
            status: "inProgress",
            step: status.step,
            ontoBranch: status.ontoBranch,
            upstreamBranch: status.upstreamBranch,
          }));
          startPolling();
        } else if (mapped === "idle") {
          // Rebase is not in progress — the error was a real failure
          const errMsg = res.stderr || "Unknown error";
          toast.error(`${operationName} failed: ${errMsg}`);
          update(s => ({ ...s, status: "idle" }));
          stopPolling();
        } else {
          // Any other state
          update(s => ({ ...s, status: mapped }));
          startPolling();
        }
      } catch {
        // Couldn't check status — report the original error
        const errMsg = res.stderr || "Unknown error";
        toast.error(`${operationName} failed: ${errMsg}`);
        update(s => ({ ...s, status: "idle" }));
        stopPolling();
      }
    }
  }

  return {
    subscribe,
    startRebase: async (base: string, repoPath: string) => {
      update(s => ({ ...s, repoPath, status: "inProgress" }));
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_start", { base, repoPath });
        await checkRebaseStateAfterCommand(res, repoPath, "Rebase");
        return res;
      } catch (e: any) {
        // invoke itself threw — this means a truly fatal backend error (not a conflict)
        console.error("Rebase invoke threw:", e);
        const errMsg = typeof e === "string" ? e : e?.message || "Unknown error";
        toast.error(`Rebase failed: ${errMsg}`);
        update(s => ({ ...s, status: "idle" }));
        return undefined;
      }
    },
    prepareInteractive: async (baseCommit: string, repoPath: string) => {
      update(s => ({ ...s, repoPath, baseCommit, status: "editingTodo" }));
      try {
        const items: RebaseTodoItem[] = await invoke("cmd_rebase_interactive_prepare", { baseCommit, repoPath });
        if (items.length === 0) {
          toast.error("No commits to rebase — the target is already an ancestor of HEAD.");
          update(s => ({ ...s, status: "idle" }));
          return [];
        }
        update(s => ({ ...s, todoItems: items }));
        return items;
      } catch (e: any) {
        console.error("Interactive rebase prepare failed:", e);
        const errMsg = typeof e === "string" ? e : e?.message || "Unknown error";
        toast.error(`Interactive rebase failed: ${errMsg}`);
        update(s => ({ ...s, status: "idle" }));
        return [];
      }
    },
    applyInteractive: async (repoPath?: string) => {
      const state = get(rebaseStore);
      const path = repoPath || state.repoPath;
      if (!state.baseCommit || !path) return;

      update(s => ({ ...s, status: "inProgress" }));
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_interactive_apply", {
          baseCommit: state.baseCommit,
          todoItems: state.todoItems,
          repoPath: path
        });
        await checkRebaseStateAfterCommand(res, path, "Interactive rebase");
        return res;
      } catch (e: any) {
        console.error("Interactive rebase apply threw:", e);
        const errMsg = typeof e === "string" ? e : e?.message || "Unknown error";
        toast.error(`Interactive rebase failed: ${errMsg}`);
        // Still check if rebase is in progress
        try {
          const status: FullRebaseStatus = await invoke("cmd_get_rebase_status", { repoPath: path });
          const mapped = status.status as RebaseStatus;
          if (mapped !== "idle") {
            update(s => ({ ...s, status: mapped, step: status.step }));
            startPolling();
          } else {
            update(s => ({ ...s, status: "idle" }));
          }
        } catch {
          update(s => ({ ...s, status: "idle" }));
        }
        return undefined;
      }
    },
    continue: async (repoPath?: string) => {
      const state = get(rebaseStore);
      const path = repoPath || state.repoPath;
      if (!path) return;
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_continue", { repoPath: path });
        await checkRebaseStateAfterCommand(res, path, "Rebase continue");
        return res;
      } catch (e: any) {
        console.error("Rebase continue threw:", e);
        const errMsg = typeof e === "string" ? e : e?.message || "Unknown error";
        toast.error(`Rebase continue failed: ${errMsg}`);
        startPolling();
        return undefined;
      }
    },
    abort: async (repoPath?: string) => {
      const state = get(rebaseStore);
      const path = repoPath || state.repoPath;
      if (!path) return;
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_abort", { repoPath: path });
        if (res.success) {
          toast.success("Rebase aborted");
        } else {
          toast.error(`Rebase abort failed: ${res.stderr}`);
        }
        update(s => ({ ...s, status: "idle", step: null }));
        stopPolling();
        return res;
      } catch (e: any) {
        console.error("Rebase abort threw:", e);
        const errMsg = typeof e === "string" ? e : e?.message || "Unknown error";
        toast.error(`Rebase abort failed: ${errMsg}`);
        update(s => ({ ...s, status: "idle", step: null }));
        stopPolling();
        return undefined;
      }
    },
    skip: async (repoPath?: string) => {
      const state = get(rebaseStore);
      const path = repoPath || state.repoPath;
      if (!path) return;
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_skip", { repoPath: path });
        await checkRebaseStateAfterCommand(res, path, "Rebase skip");
        return res;
      } catch (e: any) {
        console.error("Rebase skip threw:", e);
        const errMsg = typeof e === "string" ? e : e?.message || "Unknown error";
        toast.error(`Rebase skip failed: ${errMsg}`);
        startPolling();
        return undefined;
      }
    },
    updateTodo: (items: RebaseTodoItem[]) => {
      update(s => ({ ...s, todoItems: items }));
    },
    cancelEditing: () => {
      update(s => ({ ...s, status: "idle", todoItems: [], baseCommit: null }));
    }
  };
}

export const rebaseStore = createRebaseStore();
