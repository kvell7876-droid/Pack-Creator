//! Puts files of the pack on the Windows clipboard, so Explorer (or any program) can paste them: a copy leaves the files where they are, and a cut asks the program that pastes to move
//! them. It is the same thing Explorer itself puts there: a file list (`CF_HDROP`) and the "Preferred DropEffect" the pasting program reads to know whether it is a copy or a move.

use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT};
use windows::Win32::System::Ole::CF_HDROP;

/// The header of a file list on the clipboard (`DROPFILES`): where the names start, a point, and two flags, the second of which says the names are UTF-16.
const DROPFILES_BYTES: usize = 20;

const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_MOVE: u32 = 2;

/// Memory the clipboard can own, filled with these bytes.
fn global_with(bytes: &[u8]) -> Result<HGLOBAL, String> {
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len()).map_err(|e| format!("the clipboard could not be given memory: {e}"))?;
        let place = GlobalLock(memory);

        if place.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err("the clipboard memory could not be locked".into());
        }

        std::ptr::copy_nonoverlapping(bytes.as_ptr(), place as *mut u8, bytes.len());
        let _ = GlobalUnlock(memory);

        Ok(memory)
    }
}

/// Replaces what is on the clipboard with these files, as a copy or as a cut.
pub fn put_files(paths: &[PathBuf], cut: bool) -> Result<(), String> {
    if paths.is_empty() {
        return Err("there is nothing to put on the clipboard".into());
    }

    /* The file list: the header, then each full path in UTF-16 ending in a zero, and one more zero at the end. */
    let mut list = vec![0u8; DROPFILES_BYTES];

    list[0..4].copy_from_slice(&(DROPFILES_BYTES as u32).to_le_bytes());
    list[16..20].copy_from_slice(&1u32.to_le_bytes());

    for path in paths {
        let text = path.to_string_lossy().replace('/', "\\");

        for unit in text.encode_utf16().chain(std::iter::once(0)) {
            list.extend_from_slice(&unit.to_le_bytes());
        }
    }

    list.extend_from_slice(&[0, 0]);

    let effect = if cut { DROPEFFECT_MOVE } else { DROPEFFECT_COPY };

    unsafe {
        let files = global_with(&list)?;
        let preferred = global_with(&effect.to_le_bytes())?;
        let name: Vec<u16> = "Preferred DropEffect".encode_utf16().chain(std::iter::once(0)).collect();
        let format = RegisterClipboardFormatW(PCWSTR(name.as_ptr()));

        /* Another program may be holding the clipboard for a moment, so it is tried a few times. */
        let mut opened = false;

        for _ in 0..10 {
            if OpenClipboard(None).is_ok() {
                opened = true;
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(30));
        }

        if !opened {
            let _ = GlobalFree(Some(files));
            let _ = GlobalFree(Some(preferred));
            return Err("the clipboard is in use by another program".into());
        }

        let _ = EmptyClipboard();

        /* On success the clipboard owns the memory; on failure it is still ours to free. */
        let placed = SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(files.0)));

        if placed.is_err() {
            let _ = GlobalFree(Some(files));
        }

        if format != 0 && SetClipboardData(format, Some(HANDLE(preferred.0))).is_err() {
            let _ = GlobalFree(Some(preferred));
        }

        let _ = CloseClipboard();

        placed.map(|_| ()).map_err(|e| format!("the files could not be put on the clipboard: {e}"))
    }
}
