<script lang="ts">
  import { Dialog } from 'bits-ui';
  import {
    GitBranch,
    Search,
    Check,
    Plus,
    Loader2,
    AlertCircle,
    ArrowUpRight,
    Calendar,
    MessageSquare,
    X,
    ExternalLink,
  } from 'lucide-svelte';
  import {
    vaultRepository,
    type GitBranchItem,
    type GitBranchesResult,
  } from '@shared/repositories';

  export interface GitBranchDialogProps {
    open?: boolean;
    onBranchChanged?: () => void | Promise<void>;
  }

  let {
    open = $bindable(false),
    onBranchChanged,
  }: GitBranchDialogProps = $props();

  // Estados reactivos internos con Runes
  let activeTab = $state<'local' | 'remote'>('local');
  let searchQuery = $state('');
  let branchesResult = $state<GitBranchesResult | null>(null);
  let isLoading = $state(false);
  let isActionLoading = $state(false);
  let errorMessage = $state<string | null>(null);
  let actionSuccessMessage = $state<string | null>(null);

  // Estado del formulario de creación de nueva rama
  let showCreateSection = $state(false);
  let newBranchName = $state('');
  let createFromBranch = $state('');
  let switchAfterCreate = $state(true);
  let createInputEl = $state<HTMLInputElement | null>(null);
  let searchInputEl = $state<HTMLInputElement | null>(null);

  // Cargar ramas al abrir el modal
  $effect(() => {
    if (open) {
      errorMessage = null;
      actionSuccessMessage = null;
      searchQuery = '';
      showCreateSection = false;
      newBranchName = '';
      fetchBranches();
      requestAnimationFrame(() => {
        searchInputEl?.focus();
      });
    }
  });

  async function fetchBranches() {
    isLoading = true;
    errorMessage = null;
    try {
      const result = await vaultRepository.getGitBranches();
      branchesResult = result;
      if (!createFromBranch && result.current_branch) {
        createFromBranch = result.current_branch;
      }
    } catch (err: unknown) {
      console.error('Error al obtener ramas Git:', err);
      const msg = err instanceof Error ? err.message : String(err);
      errorMessage = `Error al listar ramas: ${msg}`;
    } finally {
      isLoading = false;
    }
  }

  // Filtrado reactivo de ramas
  const filteredLocalBranches = $derived(
    (branchesResult?.local_branches || []).filter((b) =>
      b.name.toLowerCase().includes(searchQuery.trim().toLowerCase())
    )
  );

  const filteredRemoteBranches = $derived(
    (branchesResult?.remote_branches || []).filter((b) =>
      b.name.toLowerCase().includes(searchQuery.trim().toLowerCase())
    )
  );

  const currentBranchName = $derived(branchesResult?.current_branch || null);

  // Acciones
  async function handleCheckout(branch: GitBranchItem) {
    if (isActionLoading || branch.is_current) return;
    isActionLoading = true;
    errorMessage = null;
    actionSuccessMessage = null;

    try {
      const res = await vaultRepository.gitCheckout(branch.name);
      actionSuccessMessage = res;
      await fetchBranches();
      if (onBranchChanged) {
        await onBranchChanged();
      }
      setTimeout(() => {
        if (open) open = false;
      }, 700);
    } catch (err: unknown) {
      console.error('Error en checkout de rama:', err);
      const msg = err instanceof Error ? err.message : String(err);
      errorMessage = msg;
    } finally {
      isActionLoading = false;
    }
  }

  function startCreateFromBranch(branch: GitBranchItem) {
    createFromBranch = branch.name;
    newBranchName = '';
    showCreateSection = true;
    errorMessage = null;
    requestAnimationFrame(() => {
      createInputEl?.focus();
    });
  }

  async function handleCreateBranch() {
    const trimmed = newBranchName.trim();
    if (!trimmed || isActionLoading) return;

    isActionLoading = true;
    errorMessage = null;
    actionSuccessMessage = null;

    try {
      const res = await vaultRepository.gitCreateBranch({
        newBranch: trimmed,
        baseBranch: createFromBranch || undefined,
        checkout: switchAfterCreate,
      });

      actionSuccessMessage = res;
      showCreateSection = false;
      newBranchName = '';
      await fetchBranches();
      if (onBranchChanged) {
        await onBranchChanged();
      }
      if (switchAfterCreate) {
        setTimeout(() => {
          if (open) open = false;
        }, 700);
      }
    } catch (err: unknown) {
      console.error('Error al crear rama:', err);
      const msg = err instanceof Error ? err.message : String(err);
      errorMessage = msg;
    } finally {
      isActionLoading = false;
    }
  }

  function handleClose() {
    if (isActionLoading) return;
    open = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (showCreateSection) {
        showCreateSection = false;
      } else {
        handleClose();
      }
    }
  }
</script>

<Dialog.Root
  {open}
  onOpenChange={(isOpen) => {
    if (!isOpen && !isActionLoading) {
      handleClose();
    }
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="git-branch-dialog-overlay" />
    <Dialog.Content
      class="git-branch-dialog"
      onkeydown={handleKeyDown}
      onOpenAutoFocus={(e) => {
        e.preventDefault();
        requestAnimationFrame(() => searchInputEl?.focus());
      }}
    >
      <!-- Cabecera -->
      <div class="git-branch-dialog-header">
        <div class="git-branch-icon-wrap">
          <GitBranch size={20} class="git-branch-header-icon" />
        </div>
        <div class="git-branch-titles">
          <div class="git-branch-title-row">
            <Dialog.Title class="git-branch-dialog-title">
              Ramas Git
            </Dialog.Title>
            {#if currentBranchName}
              <span class="git-current-badge" title="Rama activa actual">
                <Check size={11} />
                <span>{currentBranchName}</span>
              </span>
            {/if}
          </div>
          <Dialog.Description class="git-branch-dialog-desc">
            Gestiona, cambia entre ramas locales y remotas, o crea una nueva rama.
          </Dialog.Description>
        </div>
        <button
          type="button"
          class="git-branch-close-btn"
          onclick={handleClose}
          aria-label="Cerrar diálogo"
          title="Cerrar"
        >
          <X size={16} />
        </button>
      </div>

      <!-- Barra de herramientas: Búsqueda y botón de nueva rama -->
      <div class="git-branch-toolbar">
        <div class="git-branch-search-box">
          <Search size={14} class="git-branch-search-icon" />
          <input
            bind:this={searchInputEl}
            bind:value={searchQuery}
            type="text"
            class="git-branch-search-input"
            placeholder="Filtrar ramas..."
          />
          {#if searchQuery}
            <button
              type="button"
              class="git-branch-clear-search"
              onclick={() => (searchQuery = '')}
            >
              <X size={12} />
            </button>
          {/if}
        </div>
        <button
          type="button"
          class="git-branch-new-btn"
          class:active={showCreateSection}
          onclick={() => {
            showCreateSection = !showCreateSection;
            if (showCreateSection) {
              if (!createFromBranch && currentBranchName) {
                createFromBranch = currentBranchName;
              }
              requestAnimationFrame(() => createInputEl?.focus());
            }
          }}
          title="Crear nueva rama"
        >
          <Plus size={14} />
          <span>Nueva rama</span>
        </button>
      </div>

      <!-- Formulario para crear rama (expandible) -->
      {#if showCreateSection}
        <div class="git-create-branch-panel">
          <div class="git-create-branch-header">
            <span class="git-create-branch-title">Crear nueva rama</span>
            <button
              type="button"
              class="git-create-branch-close"
              onclick={() => (showCreateSection = false)}
            >
              <X size={13} />
            </button>
          </div>

          <div class="git-create-branch-form">
            <div class="git-create-field">
              <label for="new-branch-name-input">Nombre de la nueva rama:</label>
              <input
                id="new-branch-name-input"
                bind:this={createInputEl}
                bind:value={newBranchName}
                type="text"
                class="git-create-input"
                placeholder="ej. feature/mi-nueva-caracteristica"
                disabled={isActionLoading}
                onkeydown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    handleCreateBranch();
                  }
                }}
              />
            </div>

            <div class="git-create-field">
              <label for="create-base-branch-select">Basada en la rama:</label>
              <select
                id="create-base-branch-select"
                bind:value={createFromBranch}
                class="git-create-select"
                disabled={isActionLoading}
              >
                {#if currentBranchName}
                  <option value={currentBranchName}>
                    {currentBranchName} (Actual)
                  </option>
                {/if}
                {#each branchesResult?.local_branches || [] as b}
                  {#if b.name !== currentBranchName}
                    <option value={b.name}>{b.name} (Local)</option>
                  {/if}
                {/each}
                {#each branchesResult?.remote_branches || [] as b}
                  <option value={b.name}>{b.name} (Remota)</option>
                {/each}
              </select>
            </div>

            <label class="git-create-checkbox-label">
              <input
                type="checkbox"
                bind:checked={switchAfterCreate}
                disabled={isActionLoading}
              />
              <span>Cambiar a la nueva rama inmediatamente (checkout)</span>
            </label>

            <div class="git-create-actions">
              <button
                type="button"
                class="git-btn secondary"
                onclick={() => (showCreateSection = false)}
                disabled={isActionLoading}
              >
                Cancelar
              </button>
              <button
                type="button"
                class="git-btn primary"
                onclick={handleCreateBranch}
                disabled={!newBranchName.trim() || isActionLoading}
              >
                {#if isActionLoading}
                  <Loader2 size={13} class="spin-icon" />
                  <span>Creando...</span>
                {:else}
                  <Plus size={13} />
                  <span>Crear rama</span>
                {/if}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Mensaje de éxito -->
      {#if actionSuccessMessage}
        <div class="git-alert success" role="status">
          <Check size={14} class="alert-icon" />
          <span>{actionSuccessMessage}</span>
        </div>
      {/if}

      <!-- Mensaje de error / conflicto -->
      {#if errorMessage}
        <div class="git-alert error" role="alert">
          <AlertCircle size={15} class="alert-icon" />
          <div class="git-alert-text">
            <strong>Error:</strong> {errorMessage}
          </div>
        </div>
      {/if}

      <!-- Pestañas Locales / Remotas -->
      <div class="git-branch-tabs">
        <button
          type="button"
          class="git-branch-tab"
          class:active={activeTab === 'local'}
          onclick={() => (activeTab = 'local')}
        >
          <span>Locales</span>
          <span class="git-branch-count">
            {branchesResult?.local_branches.length ?? 0}
          </span>
        </button>
        <button
          type="button"
          class="git-branch-tab"
          class:active={activeTab === 'remote'}
          onclick={() => (activeTab = 'remote')}
        >
          <span>Remotas</span>
          <span class="git-branch-count">
            {branchesResult?.remote_branches.length ?? 0}
          </span>
        </button>
      </div>

      <!-- Lista de ramas -->
      <div class="git-branch-list-container">
        {#if isLoading}
          <div class="git-branch-empty">
            <Loader2 size={20} class="spin-icon" />
            <span>Cargando ramas...</span>
          </div>
        {:else if activeTab === 'local'}
          {#if filteredLocalBranches.length === 0}
            <div class="git-branch-empty">
              <span>No se encontraron ramas locales</span>
            </div>
          {:else}
            <div class="git-branch-list">
              {#each filteredLocalBranches as branch (branch.ref_name)}
                <div
                  class="git-branch-row"
                  class:is-current={branch.is_current}
                >
                  <div class="git-branch-info">
                    <div class="git-branch-main-line">
                      <GitBranch size={14} class="branch-item-icon" />
                      <span class="branch-name" title={branch.name}>
                        {branch.name}
                      </span>
                      {#if branch.is_current}
                        <span class="current-tag">Actual</span>
                      {/if}
                      {#if branch.upstream}
                        <span class="upstream-tag" title="Rastrea {branch.upstream}">
                          <ArrowUpRight size={10} />
                          <span>{branch.upstream}</span>
                        </span>
                      {/if}
                    </div>

                    {#if branch.last_commit_message || branch.last_commit_date}
                      <div class="git-branch-sub-line">
                        {#if branch.last_commit_date}
                          <span class="branch-date" title="Fecha del último commit">
                            <Calendar size={11} />
                            <span>{branch.last_commit_date}</span>
                          </span>
                        {/if}
                        {#if branch.last_commit_message}
                          <span class="branch-msg" title={branch.last_commit_message}>
                            <MessageSquare size={11} />
                            <span>{branch.last_commit_message}</span>
                          </span>
                        {/if}
                      </div>
                    {/if}
                  </div>

                  <!-- Acciones de la rama -->
                  <div class="git-branch-actions">
                    {#if !branch.is_current}
                      <button
                        type="button"
                        class="git-row-btn checkout"
                        onclick={() => handleCheckout(branch)}
                        disabled={isActionLoading}
                        title="Cambiar a esta rama (git checkout)"
                      >
                        <Check size={12} />
                        <span>Checkout</span>
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="git-row-btn branch-from"
                      onclick={() => startCreateFromBranch(branch)}
                      disabled={isActionLoading}
                      title="Crear nueva rama a partir de esta"
                    >
                      <Plus size={12} />
                      <span>Nueva rama</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          {#if filteredRemoteBranches.length === 0}
            <div class="git-branch-empty">
              <span>No se encontraron ramas remotas</span>
            </div>
          {:else}
            <div class="git-branch-list">
              {#each filteredRemoteBranches as branch (branch.ref_name)}
                <div class="git-branch-row">
                  <div class="git-branch-info">
                    <div class="git-branch-main-line">
                      <ExternalLink size={14} class="branch-item-icon remote" />
                      <span class="branch-name" title={branch.name}>
                        {branch.name}
                      </span>
                    </div>

                    {#if branch.last_commit_message || branch.last_commit_date}
                      <div class="git-branch-sub-line">
                        {#if branch.last_commit_date}
                          <span class="branch-date">
                            <Calendar size={11} />
                            <span>{branch.last_commit_date}</span>
                          </span>
                        {/if}
                        {#if branch.last_commit_message}
                          <span class="branch-msg" title={branch.last_commit_message}>
                            <MessageSquare size={11} />
                            <span>{branch.last_commit_message}</span>
                          </span>
                        {/if}
                      </div>
                    {/if}
                  </div>

                  <!-- Acciones de la rama remota -->
                  <div class="git-branch-actions">
                    <button
                      type="button"
                      class="git-row-btn checkout"
                      onclick={() => handleCheckout(branch)}
                      disabled={isActionLoading}
                      title="Hacer checkout a esta rama remota"
                    >
                      <Check size={12} />
                      <span>Checkout</span>
                    </button>
                    <button
                      type="button"
                      class="git-row-btn branch-from"
                      onclick={() => startCreateFromBranch(branch)}
                      disabled={isActionLoading}
                      title="Crear rama local a partir de esta remota"
                    >
                      <Plus size={12} />
                      <span>Nueva rama</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </div>

      <!-- Footer -->
      <div class="git-branch-dialog-footer">
        <button
          type="button"
          class="git-btn secondary"
          onclick={handleClose}
          disabled={isActionLoading}
        >
          Cerrar
        </button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.git-branch-dialog-overlay) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10001;
    animation: overlay-fade-in 0.15s ease-out;
  }

  @keyframes overlay-fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  :global(.git-branch-dialog) {
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: var(--modal-shadow, 0 12px 32px rgba(0, 0, 0, 0.25));
    width: 90%;
    max-width: 580px;
    max-height: 85vh;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    animation: modal-scale-in 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 10002;
    outline: none;
    box-sizing: border-box;
  }

  @keyframes modal-scale-in {
    from {
      opacity: 0;
      transform: translate(-50%, -50%) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
    }
  }

  /* Header */
  .git-branch-dialog-header {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    position: relative;
  }

  .git-branch-icon-wrap {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(9, 105, 218, 0.1);
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .git-branch-titles {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }

  .git-branch-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  :global(.git-branch-dialog-title) {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin: 0;
  }

  .git-current-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 12px;
    background: rgba(46, 160, 67, 0.15);
    color: #2ea043;
    font-size: 11.5px;
    font-weight: 500;
  }

  :global(.git-branch-dialog-desc) {
    font-size: 12.5px;
    color: var(--text-secondary, #656d76);
    margin: 0;
  }

  .git-branch-close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s;
  }

  .git-branch-close-btn:hover {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
  }

  /* Toolbar */
  .git-branch-toolbar {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .git-branch-search-box {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 6px 10px;
  }

  :global(.git-branch-search-icon) {
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  .git-branch-search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-size: 13px;
    color: var(--text-primary, #1f2328);
  }

  .git-branch-clear-search {
    background: transparent;
    border: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 2px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .git-branch-new-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .git-branch-new-btn:hover,
  .git-branch-new-btn.active {
    background: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
    color: #ffffff;
  }

  /* Panel para crear nueva rama */
  .git-create-branch-panel {
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    animation: modal-scale-in 0.12s ease-out;
  }

  .git-create-branch-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .git-create-branch-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .git-create-branch-close {
    background: transparent;
    border: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 2px;
  }

  .git-create-branch-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .git-create-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .git-create-field label {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
  }

  .git-create-input,
  .git-create-select {
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 5px;
    padding: 6px 10px;
    font-size: 12.5px;
    color: var(--text-primary, #1f2328);
    outline: none;
  }

  .git-create-input:focus,
  .git-create-select:focus {
    border-color: var(--accent, #0969da);
    box-shadow: 0 0 0 2px rgba(9, 105, 218, 0.15);
  }

  .git-create-checkbox-label {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    cursor: pointer;
  }

  .git-create-checkbox-label input {
    cursor: pointer;
  }

  .git-create-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 2px;
  }

  /* Alertas */
  .git-alert {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
    line-height: 1.4;
  }

  :global(.alert-icon) {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .git-alert.success {
    background: rgba(46, 160, 67, 0.12);
    border: 1px solid rgba(46, 160, 67, 0.35);
    color: #2ea043;
  }

  .git-alert.error {
    background: rgba(207, 34, 46, 0.1);
    border: 1px solid rgba(207, 34, 46, 0.35);
    color: #cf222e;
  }

  .git-alert-text {
    word-break: break-word;
  }

  /* Pestañas */
  .git-branch-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    gap: 4px;
  }

  .git-branch-tab {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: -1px;
    transition: all 0.15s ease;
  }

  .git-branch-tab:hover {
    color: var(--text-primary, #1f2328);
  }

  .git-branch-tab.active {
    color: var(--accent, #0969da);
    border-bottom-color: var(--accent, #0969da);
  }

  .git-branch-count {
    font-size: 11px;
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 6px;
    border-radius: 10px;
    color: var(--text-secondary, #656d76);
  }

  /* Lista de ramas */
  .git-branch-list-container {
    flex: 1;
    overflow-y: auto;
    min-height: 180px;
    max-height: 340px;
    padding-right: 4px;
  }

  .git-branch-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 40px 10px;
    color: var(--text-secondary, #656d76);
    font-size: 13px;
  }

  .git-branch-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .git-branch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-secondary, #e1e4e8);
    transition: background 0.15s, border-color 0.15s;
    gap: 8px;
  }

  .git-branch-row:hover {
    background: var(--bg-secondary, #f6f8fa);
    border-color: var(--border-primary, #d0d7de);
  }

  .git-branch-row.is-current {
    border-color: rgba(46, 160, 67, 0.4);
    background: rgba(46, 160, 67, 0.04);
  }

  .git-branch-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }

  .git-branch-main-line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex-wrap: wrap;
  }

  :global(.branch-item-icon) {
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  :global(.branch-item-icon.remote) {
    color: var(--accent, #0969da);
  }

  .branch-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .current-tag {
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 10px;
    background: rgba(46, 160, 67, 0.15);
    color: #2ea043;
  }

  .upstream-tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-secondary, #656d76);
  }

  .git-branch-sub-line {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 11.5px;
    color: var(--text-secondary, #656d76);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .branch-date,
  .branch-msg {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .git-branch-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .git-row-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: 5px;
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
    transition: all 0.15s ease;
  }

  .git-row-btn:hover:not(:disabled) {
    background: var(--bg-secondary, #f6f8fa);
    border-color: var(--accent, #0969da);
    color: var(--accent, #0969da);
  }

  .git-row-btn.checkout:hover:not(:disabled) {
    background: rgba(46, 160, 67, 0.1);
    border-color: #2ea043;
    color: #2ea043;
  }

  .git-row-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Botones generales */
  .git-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .git-btn.secondary {
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
  }

  .git-btn.secondary:hover:not(:disabled) {
    background: var(--bg-tertiary, #eaeef2);
  }

  .git-btn.primary {
    background: var(--accent, #0969da);
    border: 1px solid var(--accent, #0969da);
    color: #ffffff;
  }

  .git-btn.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .git-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Footer */
  .git-branch-dialog-footer {
    display: flex;
    justify-content: flex-end;
    border-top: 1px solid var(--border-primary, #d0d7de);
    padding-top: 12px;
  }

  :global(.spin-icon) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
