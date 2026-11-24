import type { Snippet } from 'svelte';

export interface ConfirmDialogProps {
  open?: boolean;
  title: string;
  description?: string;
  confirmText?: string;
  loadingText?: string;
  cancelText?: string;
  variant?: 'danger' | 'default';
  icon?: any;
  loading?: boolean;
  onConfirm: () => void | Promise<void>;
  onCancel?: () => void;
  children?: Snippet;
}
