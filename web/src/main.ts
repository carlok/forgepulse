import { mount } from 'svelte';
import App from './App.svelte';

import '@fontsource/jetbrains-mono/400.css';
import '@fontsource/jetbrains-mono/500.css';
import '@fontsource/jetbrains-mono/700.css';
import '@fontsource/jetbrains-mono/800.css';
import './styles.css';

const target = document.getElementById('app');
if (!target) throw new Error('Missing application root');
mount(App, { target });

