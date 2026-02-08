export interface CreateBranchDialogOptions {
    branches: string[];      // Local branch names for duplicate checking + dropdown
    currentBranch: string;   // Default base branch selection
    repoPath: string;
}

interface CreateBranchDialogState {
    isOpen: boolean;
    options: CreateBranchDialogOptions;
}

let state = $state<CreateBranchDialogState>({
    isOpen: false,
    options: { branches: [], currentBranch: '', repoPath: '' }
});

export function getCreateBranchDialogState() {
    return state;
}

export function openCreateBranchDialog(options: CreateBranchDialogOptions) {
    state.options = options;
    state.isOpen = true;
}

export function closeCreateBranchDialog() {
    state.isOpen = false;
}
