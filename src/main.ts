import { mount } from 'svelte'
import '@app/styles/app.css'
import App from '@app/App.svelte'
import { startMainThreadProbe } from '@shared/lib/perf-probe'

if (import.meta.env.DEV || import.meta.env.VITE_SYNAPSE_IPC_LOG === '1') startMainThreadProbe()

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
