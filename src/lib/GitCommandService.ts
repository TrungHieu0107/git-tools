import { invoke } from "@tauri-apps/api/core";
import { triggerGraphReload } from "./stores/git-events";
import type { GitCommandResult } from "./types";

// Helper to handle GitCommandResult in this service
async function runGitCmd(cmd: string, args?: Record<string, any>): Promise<string> {
   const res = await invoke<GitCommandResult>(cmd, args);
   if (res.success) {
       triggerGraphReload();
   }
   // Return combined output for the console
   let out = res.stdout;
   if (res.stderr) {
       out += "\n" + res.stderr;
   }
   return out || (res.success ? "Success" : "Failed");
}

export interface GitCommand {
  id: string;
  name: string;
  description: string;
  needsInput: boolean;
  inputPlaceholder?: string;
  run: (input?: string) => Promise<string | string[]>;
}

export const GIT_COMMANDS: GitCommand[] = [
  {
    id: "status",
    name: "Git Status",
    description: "Show working tree status",
    needsInput: false,
    run: async () => invoke("cmd_git_status"),
  },
  {
    id: "pull",
    name: "Git Pull",
    description: "Fetch from and integrate with another repository or a local branch",
    needsInput: false,
    run: async () => runGitCmd("cmd_git_pull"),
  },
  {
    id: "push",
    name: "Git Push",
    description: "Update remote refs along with associated objects",
    needsInput: false,
    run: async () => runGitCmd("cmd_git_push"),
  },
  {
    id: "fetch",
    name: "Git Fetch",
    description: "Download objects and refs from another repository",
    needsInput: false,
    run: async () => runGitCmd("cmd_git_fetch"),
  },
  {
    id: "commit",
    name: "Git Commit",
    description: "Record changes to the repository",
    needsInput: true,
    inputPlaceholder: "Commit message...",
    run: async (input) => {
      if (!input) throw new Error("Commit message required");
      return runGitCmd("cmd_git_commit", { message: input });
    },
  },
  {
    id: "add_all",
    name: "Git Add All",
    description: "Add file contents to the index (git add .)",
    needsInput: false,
    run: async () => invoke("cmd_git_add_all"),
  },
  {
    id: "checkout",
    name: "Git Checkout",
    description: "Switch branches or restore working tree files",
    needsInput: true,
    inputPlaceholder: "Branch name...",
    run: async (input) => {
      if (!input) throw new Error("Branch name required");
      return runGitCmd("cmd_git_checkout", { branch: input });
    },
  },
  {
    id: "branch_list",
    name: "Git Branch List",
    description: "List branches",
    needsInput: false,
    run: async () => invoke("cmd_git_branch_list"),
  },
  {
    id: "log",
    name: "Git Log",
    description: "Show commit logs (last 50)",
    needsInput: false, // Could be true for custom limit/args, but simplified as per request for now
    run: async () => invoke("cmd_git_log", { limit: 50 }),
  },
];
