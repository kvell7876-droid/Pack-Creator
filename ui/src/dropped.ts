/** Files that came from outside the Pack Creator (dragged from Explorer or another program, or pasted from the clipboard), read and ready to be copied into the pack. */

export interface Incoming {
  /** Where it goes inside the folder it is put in, with `/` between folders: a dropped folder keeps its own folders. */
  rel: string;
  /** The bytes, as base64. */
  data: string;
}

/** The most one file may be, in bytes: the bytes cross to the program as text, so a very large file is better copied by hand. */
const LARGEST = 200 * 1024 * 1024;

interface Found {
  rel: string;
  file: File;
}

/** Whether a drag or a paste carries files (and not only text, or an entry of the tree being moved). */
export function hasFiles(transfer: DataTransfer | null | undefined): boolean {
  return !!transfer && Array.from(transfer.types).includes('Files');
}

const readEntries = (reader: FileSystemDirectoryReader) => new Promise<FileSystemEntry[]>((resolve, reject) => reader.readEntries(resolve, reject));
const fileOf = (entry: FileSystemFileEntry) => new Promise<File>((resolve, reject) => entry.file(resolve, reject));

/** A folder entry and everything under it. A reader hands back its entries a batch at a time, so it is read until it is empty. */
async function walk(entry: FileSystemEntry, prefix: string, found: Found[]): Promise<void> {
  if (entry.isFile) {
    found.push({ rel: prefix + entry.name, file: await fileOf(entry as FileSystemFileEntry) });
    return;
  }

  const reader = (entry as FileSystemDirectoryEntry).createReader();

  for (;;) {
    const batch = await readEntries(reader);

    if (batch.length === 0) break;

    for (const child of batch) await walk(child, prefix + entry.name + '/', found);
  }
}

const base64Of = (file: File) =>
  new Promise<string>((resolve, reject) => {
    const reader = new FileReader();

    reader.onerror = () => reject(reader.error ?? new Error('The file could not be read.'));
    reader.onload = () => resolve(String(reader.result).replace(/^data:[^,]*,/, ''));
    reader.readAsDataURL(file);
  });

/**
 * Reads what a drop or a paste carries. The entries are collected first and straight away, because a browser empties a transfer as soon as the event handler returns, and only then
 * are the folders walked and the files read.
 *
 * @returns what could be read, and the names of the files that were left out (too big, or could not be read)
 */
export async function collect(transfer: DataTransfer): Promise<{ files: Incoming[]; skipped: string[] }> {
  const entries: FileSystemEntry[] = [];
  const loose: File[] = [];

  for (const item of Array.from(transfer.items ?? [])) {
    if (item.kind !== 'file') continue;

    const entry = item.webkitGetAsEntry?.();

    if (entry) entries.push(entry);
    else {
      const file = item.getAsFile();

      if (file) loose.push(file);
    }
  }

  /* Some sources give only a list of files and no entries. */
  if (entries.length === 0 && loose.length === 0) loose.push(...Array.from(transfer.files ?? []));

  const found: Found[] = loose.map(file => ({ rel: file.name, file }));
  const skipped: string[] = [];

  for (const entry of entries) {
    try {
      await walk(entry, '', found);
    } catch {
      skipped.push(entry.name);
    }
  }

  const files: Incoming[] = [];

  for (const item of found) {
    if (item.file.size > LARGEST) { skipped.push(item.rel); continue; }

    try {
      files.push({ rel: item.rel, data: await base64Of(item.file) });
    } catch {
      skipped.push(item.rel);
    }
  }

  return { files, skipped };
}
