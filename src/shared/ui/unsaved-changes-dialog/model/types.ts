export interface UnsavedChangesDialogProps {
  open?: boolean;
  title?: string;
  files: string[];
  isWindowClose?: boolean;
  isManualSave?: boolean;
  isRename?: boolean;
  loading?: boolean;
  onAbort: () => void;
  onDiscard: () => void;
  onForceSave: () => void | Promise<void>;
}
