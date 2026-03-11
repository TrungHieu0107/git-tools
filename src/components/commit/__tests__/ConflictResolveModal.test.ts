import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("../../lib/GitService", () => ({
  GitService: {
    getSettings: vi.fn(),
    getConflictFile: vi.fn(),
    getFileModifiedContent: vi.fn(),
    writeFile: vi.fn(),
    markResolved: vi.fn(),
  },
}));

vi.mock("../../lib/toast.svelte", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import { GitService } from "../../lib/GitService";
import ConflictResolveModal from "../ConflictResolveModal.svelte";

const MOCK_CONFLICT_FILE = {
  base: "BASE\n",
  ours: "OURS\n",
  theirs: "THEIRS\n",
};

const MOCK_OPERATION_STATE = {
  isMerging: true,
  isRebasing: false,
  isCherryPicking: false,
  isReverting: false,
  hasConflicts: true,
  conflictPaths: ["file.txt"],
  oursCommit: "111",
  theirsCommit: "222",
  oursBranch: "main",
  theirsBranch: "feature",
  rebaseCurrent: null,
  rebaseTotal: null,
  rebaseMessage: null,
};

describe("ConflictResolveModal", () => {
  const getSettings = vi.mocked(GitService.getSettings);
  const getConflictFile = vi.mocked(GitService.getConflictFile);
  const getFileModifiedContent = vi.mocked(GitService.getFileModifiedContent);
  const writeFile = vi.mocked(GitService.writeFile);
  const markResolved = vi.mocked(GitService.markResolved);

  beforeEach(() => {
    vi.clearAllMocks();
    getSettings.mockResolvedValue({ file_encodings: {}, repo_default_encodings: {} });
    getConflictFile.mockResolvedValue(MOCK_CONFLICT_FILE);
    getFileModifiedContent.mockResolvedValue(
      [
        "<<<<<<< LOCAL",
        "OURS content",
        "=======",
        "THEIRS content",
        ">>>>>>> ORIGIN",
        "",
      ].join("\n")
    );
    writeFile.mockResolvedValue();
    markResolved.mockResolvedValue();
  });

  it("renders conflict panes after loading content", async () => {
    render(ConflictResolveModal, {
      props: {
        repoPath: "/repo",
        filePath: "file.txt",
        operationState: MOCK_OPERATION_STATE,
        onClose: vi.fn(),
      },
    });

    expect(screen.getByText("Loading conflict content...")).toBeInTheDocument();

    await waitFor(() => expect(screen.queryByText("Loading conflict content...")).not.toBeInTheDocument());

    expect(screen.getByText("Resolve Conflict")).toBeInTheDocument();
    expect(screen.getByText("Local (Ours)")).toBeInTheDocument();
    expect(screen.getByText("Origin (Theirs)")).toBeInTheDocument();
    expect(screen.getByText(/block\(s\)/i)).toBeInTheDocument();
  });

  it("shows error state when getConflictFile fails", async () => {
    getConflictFile.mockRejectedValueOnce(new Error("fail"));

    render(ConflictResolveModal, {
      props: { repoPath: "/repo", filePath: "file.txt", operationState: MOCK_OPERATION_STATE, onClose: vi.fn() },
    });

    await waitFor(() => expect(screen.getByText("fail")).toBeInTheDocument());
  });

  it("saves resolved content and marks resolved", async () => {
    const onClose = vi.fn();
    render(ConflictResolveModal, {
      props: { repoPath: "/repo", filePath: "file.txt", operationState: MOCK_OPERATION_STATE, onClose },
    });

    await waitFor(() => expect(screen.queryByText("Loading conflict content...")).not.toBeInTheDocument());

    const saveButton = screen.getByRole("button", { name: /save & mark resolved/i });
    await fireEvent.click(saveButton);

    await waitFor(() => expect(writeFile).toHaveBeenCalled());
    expect(markResolved).toHaveBeenCalledWith("file.txt", "/repo");
    expect(onClose).toHaveBeenCalled();
  });
});
