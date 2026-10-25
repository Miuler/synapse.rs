<script lang="ts">
  import type { Snippet } from "svelte";
  import { EditorHeader } from "@widgets/editor-header";
  import { systemApi } from "@shared/api";
  import { Columns2 } from "lucide-svelte";
  import type { TabItem, MarkdownViewMode } from "@entities/vault-item";

  interface Props {
    paneId: "single" | "left" | "right";
    isSplit?: boolean;
    style?: string;
    isEditing: boolean;
    tabs: TabItem[];
    activeTabPath: string | null;
    canGoBack?: boolean;
    canGoForward?: boolean;
    isMarkdownFile?: boolean;
    showViewToggle?: boolean;
    markdownViewMode?: MarkdownViewMode;
    title?: string;
    showSaveButton?: boolean;
    showWindowControls?: boolean;
    isSplitActive?: boolean;
    containerRef?: HTMLDivElement | null;
    showSplitDropZoneOverlay?: boolean;

    // Callbacks
    onNavigateBack?: () => void;
    onNavigateForward?: () => void;
    onChangeMarkdownView?: (mode: MarkdownViewMode) => void;
    onSelectTab?: (path: string) => void;
    onCloseTab?: (path: string) => void;
    onCloseAllTabs?: () => void;
    onCloseOtherTabs?: (path: string) => void;
    onCloseTabsToRight?: (path: string) => void;
    onOpenInNewWindow?: (path: string) => void;
    onReorderTab?: (draggedPath: string, targetIndex: number) => void;
    onReceiveTab?: (path: string, insertIndex?: number, sourceWindowId?: string) => void;
    onNewTab?: () => void;
    onNewFile?: () => void;
    onOpenQuickOpen?: () => void;
    onSplitView?: () => void;
    onToggleView?: () => void;
    onSave?: () => void;
    onOpenCommandPalette?: () => void;
    onPointerDown?: () => void;
    onDragOver?: (e: DragEvent) => void;
    onDragLeave?: (e: DragEvent) => void;
    onDrop?: (e: DragEvent) => void;

    children?: Snippet;
  }

  let {
    paneId,
    isSplit = false,
    style = undefined,
    isEditing = $bindable(true),
    tabs,
    activeTabPath,
    canGoBack = false,
    canGoForward = false,
    isMarkdownFile = false,
    showViewToggle = false,
    markdownViewMode = "live",
    title = "",
    showSaveButton = false,
    showWindowControls = false,
    isSplitActive = false,
    containerRef = $bindable(null),
    showSplitDropZoneOverlay = false,
    onNavigateBack,
    onNavigateForward,
    onChangeMarkdownView,
    onSelectTab,
    onCloseTab,
    onCloseAllTabs,
    onCloseOtherTabs,
    onCloseTabsToRight,
    onOpenInNewWindow,
    onReorderTab,
    onReceiveTab,
    onNewTab,
    onNewFile,
    onOpenQuickOpen,
    onSplitView,
    onToggleView,
    onSave,
    onOpenCommandPalette,
    onPointerDown,
    onDragOver,
    onDragLeave,
    onDrop,
    children,
  }: Props = $props();

  function handleAction(actionId: string) {
    if (actionId === "nav-back") onNavigateBack?.();
    else if (actionId === "nav-forward") onNavigateForward?.();
    else if (actionId === "new-file") onNewFile?.();
    else if (actionId === "quick-open") onOpenQuickOpen?.();
  }
</script>

{#snippet paneContent()}
  <EditorHeader
    bind:isEditing
    {tabs}
    {activeTabPath}
    {canGoBack}
    {canGoForward}
    {onNavigateBack}
    {onNavigateForward}
    {isMarkdownFile}
    {showViewToggle}
    markdownViewMode={!isEditing ? "reading" : markdownViewMode}
    {onChangeMarkdownView}
    {title}
    {showSaveButton}
    {onSelectTab}
    {onCloseTab}
    {onCloseAllTabs}
    {onCloseOtherTabs}
    {onCloseTabsToRight}
    onOpenFileWithDefaultApp={(p) => systemApi.openFileWithDefaultApp(p)}
    onShowItemInFolder={(p) => systemApi.showItemInFolder(p)}
    {onOpenInNewWindow}
    {onReorderTab}
    onReceiveTabFromOtherWindow={onReceiveTab}
    {onNewTab}
    {onNewFile}
    {onOpenQuickOpen}
    {onSplitView}
    onAction={handleAction}
    {onToggleView}
    {onSave}
    {onOpenCommandPalette}
    {showWindowControls}
    {isSplitActive}
  />

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="editor-container"
    bind:this={containerRef}
    ondragover={onDragOver}
    ondragleave={onDragLeave}
    ondrop={onDrop}
    onpointerdown={onPointerDown}
  >
    {@render children?.()}

    {#if showSplitDropZoneOverlay}
      <div class="split-drop-zone-overlay">
        <div class="split-drop-zone-content">
          <Columns2 size={36} />
          <span>Dividir panel a la derecha</span>
        </div>
      </div>
    {/if}
  </div>
{/snippet}

{#if isSplit}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="split-pane-wrapper {paneId}-pane"
    {style}
    onpointerdown={onPointerDown}
  >
    {@render paneContent()}
  </div>
{:else}
  {@render paneContent()}
{/if}

<style>
  .split-pane-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    position: relative;
  }

  .editor-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  .split-drop-zone-overlay {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 50%;
    background-color: rgba(9, 105, 218, 0.12);
    border: 2px dashed var(--accent, #0969da);
    border-radius: 8px;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    animation: fadeIn 0.15s ease-out;
  }

  .split-drop-zone-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    color: var(--accent, #0969da);
    font-weight: 600;
    font-size: 14px;
    padding: 16px 24px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--accent, #0969da);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
