// A tiny driver for the Pack Creator's real window, through WebView2's debugging port. Start the app with
//   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 target/release/pack-creator.exe
// then: eval, shot, key ("ctrl+.", Enter), click ("x,y"), text ("typed text"). It needs Node 22 or later (the built in WebSocket).
// node cdp.mjs eval "<js>"      runs js in the page (await allowed) and prints the result
// node cdp.mjs shot <file.png>  saves a screenshot of the window
import fs from 'node:fs';

const [command, argument] = process.argv.slice(2);
const targets = await (await fetch('http://127.0.0.1:9222/json')).json();
const page = targets.find(t => t.type === 'page');

if (!page) { console.error('no page'); process.exit(1); }

const socket = new WebSocket(page.webSocketDebuggerUrl);
let id = 0;
const waiting = new Map();

socket.onmessage = event => {
  const message = JSON.parse(event.data);

  if (message.id && waiting.has(message.id)) { waiting.get(message.id)(message); waiting.delete(message.id); }
};

await new Promise(resolve => (socket.onopen = resolve));

const send = (method, params = {}) => new Promise(resolve => { const n = ++id; waiting.set(n, resolve); socket.send(JSON.stringify({ id: n, method, params })); });

if (command === 'eval') {
  const reply = await send('Runtime.evaluate', { expression: argument, awaitPromise: true, returnByValue: true });

  console.log(JSON.stringify(reply.result?.exceptionDetails ? reply.result.exceptionDetails.exception : (reply.result?.result?.value ?? reply.result), null, 2));
} else if (command === 'shot') {
  const reply = await send('Page.captureScreenshot', { format: 'png' });

  fs.writeFileSync(argument, Buffer.from(reply.result.data, 'base64'));
  console.log('saved', argument);
}

else if (command === 'click') {
  const [x, y] = argument.split(',').map(Number);

  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
  console.log('clicked');
} else if (command === 'text') {
  await send('Input.insertText', { text: argument });
  console.log('typed');
} else if (command === 'key') {
  // node cdp.mjs key "ctrl+." or "Enter"
  const parts = argument.split('+');
  const name = parts.pop();
  const modifiers = (parts.includes('ctrl') ? 2 : 0) | (parts.includes('shift') ? 8 : 0) | (parts.includes('alt') ? 1 : 0);
  const table = { '.': ['.', 'Period', 190], Enter: ['Enter', 'Enter', 13], Escape: ['Escape', 'Escape', 27], Space: [' ', 'Space', 32], G: ['g', 'KeyG', 71], ArrowDown: ['ArrowDown', 'ArrowDown', 40] };
  const [key, code, vk] = table[name] ?? [name, 'Key' + name.toUpperCase(), name.toUpperCase().charCodeAt(0)];

  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', modifiers, key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', modifiers, key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  console.log('sent', argument);
}

socket.close();
process.exit(0);
