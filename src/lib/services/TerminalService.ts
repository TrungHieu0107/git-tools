import { invoke } from "@tauri-apps/api/core";

export class TerminalService {
  static async startTerminal(repoPath: string): Promise<void> {
    return invoke("cmd_terminal_start", { repoPath });
  }

  static async writeTerminal(repoPath: string, input: string): Promise<void> {
    return invoke("cmd_terminal_write", { repoPath, input });
  }

  static async stopTerminal(repoPath: string): Promise<void> {
    return invoke("cmd_terminal_stop", { repoPath });
  }
}
