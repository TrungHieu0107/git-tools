export interface PromptOptions {
  title: string;
  message: string;
  placeholder?: string;
  defaultValue?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  isHtmlMessage?: boolean;
}

interface PromptState {
  isOpen: boolean;
  value: string;
  options: PromptOptions;
  resolve: (value: string | null) => void;
}

let state = $state<PromptState>({
  isOpen: false,
  value: "",
  options: {
    title: "",
    message: "",
    placeholder: "",
    defaultValue: "",
    confirmLabel: "Confirm",
    cancelLabel: "Cancel",
    isHtmlMessage: false,
  },
  resolve: () => {},
});

export function getPromptState() {
  return state;
}

export function prompt(options: PromptOptions): Promise<string | null> {
  state.options = {
    placeholder: "",
    defaultValue: "",
    confirmLabel: "Confirm",
    cancelLabel: "Cancel",
    isHtmlMessage: false,
    ...options,
  };
  state.value = options.defaultValue ?? "";
  state.isOpen = true;

  return new Promise((resolve) => {
    state.resolve = (value: string | null) => {
      state.isOpen = false;
      resolve(value);
    };
  });
}
