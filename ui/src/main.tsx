import { render } from 'preact';
import { App } from './App';
import './styles.css';
import { initZoom } from './zoom';

initZoom();

render(<App />, document.getElementById('app')!);
