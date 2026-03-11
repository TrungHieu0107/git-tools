import { render, waitFor } from "@testing-library/svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("../../lib/GitService", () => ({
  GitService: {
    getStatusFiles: vi.fn(),
    getConflicts: vi.fn(),
    getOperationState: vi.fn(),
    getSettings: vi.fn(),
    abortOperation: vi.fn(),
  },
}));

vi.mock("../../lib/toast.svelte", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("../../lib/confirmation.svelte", () => ({
  confirm: vi.fn().mockResolvedValue(true),
}));

import { GitService } from "../../lib/GitService";
import GraphWipPanel from "../GraphWipPanel.svelte";

const defaultOperationState = {
  isMerging: true,
  isRebasing: false,
  isCherryPicking: false,
  isReverting: false,
  hasConflicts: true,
  conflictPaths: ["src/file1.ts"],
};

describe("GraphWipPanel conflict mode", () => {
  const getStatusFiles = vi.mocked(GitService.getStatusFiles);
  const getConflicts = vi.mocked(GitService.getConflicts);
  const getOperationState = vi.mocked(GitService.getOperationState);
  const getSettings = vi.mocked(GitService.getSettings);

  beforeEach(() => {
    vi.clearAllMocks();
    getStatusFiles.mockResolvedValue([{ path: "src/file1.ts", status: "U", staged: false, ignoredByApp: false }]);
    getConflicts.mockResolvedValue(["src/file1.ts"]);
    getOperationState.mockResolvedValue(defaultOperationState);
    getSettings.mockResolvedValue({ show_ignored_files: false, excluded_files: [], repo_filters: {}, repos: [], open_repo_ids: [], active_repo_id: null });
  });

  it("shows conflict layout when hasConflicts is true", async () => {
    const { getByText } = render(GraphWipPanel, { props: { repoPath: "/repo", onClose: vi.fn(), onCommitSuccess: vi.fn() } });

    await waitFor(() => expect(getByText(/Conflicted Files/i)).toBeInTheDocument());
    expect(getByText(/Resolved Files/)).toBeInTheDocument();
  });

  it("refresh() reloads status without calling onCommitSuccess", async () => {
    const onCommitSuccess = vi.fn();
    const renderResult = render(GraphWipPanel, { props: { repoPath: "/repo", onCommitSuccess } });

    await waitFor(() => expect(getStatusFiles).toHaveBeenCalledTimes(1));
    onCommitSuccess.mockClear();
    getStatusFiles.mockResolvedValueOnce([{ path: "src/file1.ts", status: "U", staged: false, ignoredByApp: false }]);

    renderResult.component.refresh();
    await waitFor(() => expect(getStatusFiles).toHaveBeenCalledTimes(2));
    expect(onCommitSuccess).not.toHaveBeenCalled();
  });
});
