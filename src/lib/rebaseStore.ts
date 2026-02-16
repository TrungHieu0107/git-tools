import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { GitService, type GitCommandResult } from "./GitService";
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
  status: string; // The raw status from backend (camelCase)
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

  return {
    subscribe,
    startRebase: async (base: string, repoPath: string) => {
      update(s => ({ ...s, repoPath, status: "inProgress" }));
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_start", { base, repoPath });
        if (res.success) {
          startPolling();
        } else {
          update(s => ({ ...s, status: "idle" }));
          toast.error(`Rebase failed: ${res.stderr}`);
        }
        return res;
      } catch (e) {
        update(s => ({ ...s, status: "idle" }));
        throw e;
      }
    },
    prepareInteractive: async (baseCommit: string, repoPath: string) => {
      update(s => ({ ...s, repoPath, baseCommit, status: "editingTodo" }));
      try {
        const items: RebaseTodoItem[] = await invoke("cmd_rebase_interactive_prepare", { baseCommit, repoPath });
        update(s => ({ ...s, todoItems: items }));
        return items;
      } catch (e) {
        update(s => ({ ...s, status: "idle" }));
        throw e;
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
        
        if (res.success) {
          startPolling();
        } else {
          pollStatus();
          startPolling();
        }
        return res;
      } catch (e) {
        startPolling();
        throw e;
      }
    },
    continue: async (repoPath?: string) => {
      const state = get(rebaseStore);
      const path = repoPath || state.repoPath;
      if (!path) return;
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_continue", { repoPath: path });
        startPolling();
        return res;
      } catch (e) {
        throw e;
      }
    },
    abort: async (repoPath?: string) => {
      const state = get(rebaseStore);
      const path = repoPath || state.repoPath;
      if (!path) return;
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_abort", { repoPath: path });
        update(s => ({ ...s, status: "idle", step: null }));
        stopPolling();
        return res;
      } catch (e) {
        throw e;
      }
    },
    skip: async (repoPath?: string) => {
      const state = get(rebaseStore);
      const path = repoPath || state.repoPath;
      if (!path) return;
      try {
        const res: GitCommandResult = await invoke("cmd_rebase_skip", { repoPath: path });
        startPolling();
        return res;
      } catch (e) {
        throw e;
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
