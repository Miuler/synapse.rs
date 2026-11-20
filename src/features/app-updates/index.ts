export { default as NewsTab } from './ui/NewsTab.svelte';
export { default as UpdateNotice } from './ui/UpdateNotice.svelte';
export { appUpdates, type UpdateInfo } from './model/app-updates.svelte';
export { newsMarkdown, newsToOpenOnStartup, parseChangelog, previousVersion } from './lib/changelog';
export { CHANGELOG_RAW } from './lib/changelog-source';

/** Pestañas en memoria: novedades de la versión instalada (`synapse://news[:desde]`) y de la nueva disponible. */
export const NEWS_TAB = 'synapse://news';
export const UPDATE_NOTES_TAB = 'synapse://update-notes';
export const LAST_SEEN_VERSION_KEY = 'synapse.lastSeenVersion';
