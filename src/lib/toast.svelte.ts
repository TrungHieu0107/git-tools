export interface ToastMessage {
  id: string;
  type: 'success' | 'error' | 'info';
  message: string;
  duration?: number;
}

class ToastStore {
  toasts = $state<ToastMessage[]>([]);

  add(type: 'success' | 'error' | 'info', message: string, duration = 3000) {
    const id = crypto.randomUUID();
    const toast: ToastMessage = { id, type, message, duration };
    this.toasts.push(toast);

    if (duration > 0) {
      setTimeout(() => {
        this.remove(id);
      }, duration);
    }
  }

  remove(id: string) {
    const index = this.toasts.findIndex(t => t.id === id);
    if (index !== -1) {
      this.toasts.splice(index, 1);
    }
  }

  success(message: string, duration = 3000) {
    this.add('success', message, duration);
  }

  error(message: string, duration = 5000) {
    this.add('error', message, duration);
  }

  info(message: string, duration = 3000) {
    this.add('info', message, duration);
  }
}

export const toast = new ToastStore();
