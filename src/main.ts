import { mount } from 'svelte'
import '@app/styles/app.css'
import App from '@app/App.svelte'

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
