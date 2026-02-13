import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, RepoEntry } from "../GitService";

export class RepositoryService {
  static async getSettings(): Promise<AppSettings> {
    return invoke("cmd_get_settings");
  }

  static async setExcludedFiles(exclusions: string[]): Promise<AppSettings> {
    return invoke("cmd_set_excluded_files", { exclusions });
  }

  static async setRepoFilter(repoId: string, filter: string): Promise<AppSettings> {
    return invoke("cmd_set_repo_filter", { repoId, filter });
  }

  static async setGeminiApiToken(token: string): Promise<AppSettings> {
    return invoke("cmd_set_gemini_api_token", { token });
  }

  static async setGeminiModel(model: string): Promise<AppSettings> {
    return invoke("cmd_set_gemini_model", { model });
  }

  static async getGeminiModels(token?: string): Promise<string[]> {
    const trimmed = token?.trim();
    return invoke("cmd_get_gemini_models", { token: trimmed ? trimmed : null });
  }

  static async addRepo(name: string, path: string): Promise<AppSettings> {
    return invoke("cmd_add_repo", { name, path });
  }

  static async removeRepo(id: string): Promise<AppSettings> {
    return invoke("cmd_remove_repo", { id });
  }

  static async setActiveRepo(id: string): Promise<AppSettings> {
    return invoke("cmd_set_active_repo", { id });
  }

  static async openRepo(id: string): Promise<AppSettings> {
    return invoke("cmd_open_repo", { id });
  }

  static async closeRepo(id: string): Promise<AppSettings> {
    return invoke("cmd_close_repo", { id });
  }

  static async getActiveRepo(): Promise<RepoEntry | null> {
    return invoke("cmd_get_active_repo");
  }
}
