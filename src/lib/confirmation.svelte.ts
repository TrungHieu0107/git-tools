export interface ConfirmationOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  isHtmlMessage?: boolean;
}

interface ConfirmationState {
    isOpen: boolean;
    options: ConfirmationOptions;
    resolve: (value: boolean) => void;
}

// Global state using Svelte 5 rune
let state = $state<ConfirmationState>({
    isOpen: false,
    options: { title: "", message: "" },
    resolve: () => {}
});

export function getConfirmationState() {
    return state;
}

export function confirm(options: ConfirmationOptions): Promise<boolean> {
    state.options = { 
        confirmLabel: "Confirm",
        cancelLabel: "Cancel",
        ...options 
    };
    state.isOpen = true;
    
    return new Promise((resolve) => {
        state.resolve = (value: boolean) => {
            state.isOpen = false;
            resolve(value);
        };
    });
}
