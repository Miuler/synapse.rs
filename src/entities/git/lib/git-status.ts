export interface NormalizedGitStatus {
  index: string | null;
  worktree: string | null;
  is_stashed: boolean;
}

export function parseGitStatusInfo(raw: unknown): NormalizedGitStatus | null {
  if (!raw) return null;
  if (typeof raw === 'string') {
    if (raw === 'modified') return { index: null, worktree: 'M', is_stashed: false };
    if (raw === 'untracked') return { index: null, worktree: '?', is_stashed: false };
    if (raw === 'staged') return { index: 'M', worktree: null, is_stashed: false };
    if (raw === 'staged_modified' || raw === 'modified_staged') return { index: 'M', worktree: 'M', is_stashed: false };
    if (raw === 'stashed') return { index: null, worktree: null, is_stashed: true };
    if (raw === 'stashed_modified' || raw === 'modified_staged') return { index: null, worktree: 'M', is_stashed: true };
    return null;
  }
  if (typeof raw === 'object') {
    const s = raw as Record<string, unknown>;
    const index = typeof s.index === 'string' ? s.index : null;
    const worktree = typeof s.worktree === 'string' ? s.worktree : null;
    const is_stashed = Boolean(s.is_stashed);
    if (!index && !worktree && !is_stashed) return null;
    return { index, worktree, is_stashed };
  }
  return null;
}
