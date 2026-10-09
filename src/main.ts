import { getCurrentWindow } from '@tauri-apps/api/window'
import { mount } from 'svelte'
import App from './App.svelte'
import './styles/tokens.css'

// One bundle for both windows; the window label picks the view.
const app = mount(App, {
  target: document.getElementById('app')!,
  props: { label: getCurrentWindow().label },
})

export default app
