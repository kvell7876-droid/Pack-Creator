import { useEffect, useState } from 'preact/hooks';
import { api } from './api';
import { Dialog } from './Dialogs';
import { messageOf, notify } from './notify';
import type { BlockbenchStatus } from './types';

/**
 * Blockbench, the editor the models, animations and textures are made in. It shows whether it is installed (version 5.1 or newer is needed for the Strut Toolkit),
 * downloads and starts its installer, and starts it with the toolkit ready. The Pack Creator keeps a profile of its own for it, so the person's usual Blockbench is not touched.
 */
export function BlockbenchDialog({ onClose }: { onClose: () => void }) {
  const [status, setStatus] = useState<BlockbenchStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = () => { void api.blockbenchStatus().then(setStatus).catch(error => notify(messageOf(error))); };

  useEffect(() => {
    refresh();

    const timer = window.setInterval(refresh, 2000);

    return () => window.clearInterval(timer);
  }, []);

  const install = async () => {
    try {
      await api.installBlockbench();
      refresh();
    } catch (error) {
      notify(messageOf(error));
    }
  };

  const prepare = async () => {
    setBusy(true);

    try {
      await api.prepareBlockbench();
      notify('Blockbench is open with the Strut Toolkit ready.');
    } catch (error) {
      notify(messageOf(error));
    } finally {
      setBusy(false);
      refresh();
    }
  };

  const installing = status?.installing ?? '';
  const failed = installing.startsWith('failed');
  const ready = !!status?.exe && status.versionOk;

  return (
    <Dialog title="Blockbench" onClose={onClose}>
      {!status && <p class="muted" style="margin:0">Looking for Blockbench...</p>}

      {status && (
        <div style="display:flex;flex-direction:column;gap:10px">
          <div>
            {status.exe
              ? <>Blockbench {status.version ?? ''} is installed. {status.versionOk ? <span class="muted">That version works with the Strut Toolkit.</span> : <b>The Strut Toolkit needs Blockbench 5.1 or newer; install the newest one below.</b>}</>
              : <>Blockbench is not installed on this computer.</>}
          </div>

          <div class="muted">
            Strut Toolkit {status.toolkitVersion} comes with the Pack Creator and is loaded into a Blockbench profile of its own the first time a model is opened, so your usual Blockbench is left as it is.
            It adds the Strut model and animation formats, and a <b>Save to pack</b> command in the File menu that writes the model straight back into the pack.
          </div>

          {installing && <div class={failed ? 'form-note' : 'chip'} style={failed ? '' : 'align-self:flex-start'}>{installing}</div>}

          <div class="foot" style="margin-top:6px">
            <button class="btn ghost" onClick={onClose}>Close</button>
            {!ready && <button class="btn primary" disabled={!!installing && !failed} onClick={() => void install()} title="Downloads Blockbench's own installer from github.com/JannisX11/blockbench (about 110 MiB) and runs it">{status.exe ? 'Install the newest Blockbench' : 'Install Blockbench'}</button>}
            {ready && <button class="btn primary" disabled={busy} onClick={() => void prepare()}>{busy ? 'Starting...' : status.running ? 'Show Blockbench' : 'Open Blockbench with the toolkit'}</button>}
          </div>
        </div>
      )}
    </Dialog>
  );
}
