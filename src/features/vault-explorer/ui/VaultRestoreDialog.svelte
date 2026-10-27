<script lang="ts">
  import { GitRestoreDialog } from '@shared/ui/git-restore-dialog';

  interface Props {
    open: boolean;
    items: string[];
    loading?: boolean;
    onConfirm: () => void | Promise<void>;
    onCancel: () => void;
  }

  let {
    open = $bindable(false),
    items = [],
    loading = false,
    onConfirm,
    onCancel,
  }: Props = $props();
</script>

<GitRestoreDialog
  bind:open
  {items}
  {loading}
  {onConfirm}
  {onCancel}
/>
