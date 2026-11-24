//! Import hooks on archiver.dll. The game opens a file in one of two ways; both
//! hand the file to `translate_file` and serve the result from memory.

use super::{classify, normalize, translate_file, FileTexts};
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    sync::{Mutex, MutexGuard, OnceLock},
};

// Archiver.dll imports of Patrician3.exe: RVAs of the IAT slots, named after the
// import table. All are cdecl; a file handle is an opaque pointer.
/// `arc_Seek(handle, position) -> position`, absolute position.
const IAT_SEEK: usize = 0x26a018;
/// `arc_FileLength(handle) -> length`.
const IAT_FILE_LENGTH: usize = 0x26a01c;
/// `arc_GetPos(handle) -> position`.
const IAT_GET_POS: usize = 0x26a020;
/// `arc_ReleaseFile(handle)`.
const IAT_RELEASE_FILE: usize = 0x26a028;
/// `arc_CreateFile(container, name) -> handle`, null if the file is missing.
/// Missions and ini files.
const IAT_CREATE_FILE: usize = 0x26a02c;
/// `arc_ReleaseMemMapped(container, handle)`.
const IAT_RELEASE_MAPPED: usize = 0x26a030;
/// `arc_CreateMemMapped(container, name, *data, *size) -> handle`, null if the file
/// is missing. Maps and name lists.
const IAT_CREATE_MAPPED: usize = 0x26a04c;
/// `arc_Read(handle, buffer, length) -> bytes read`.
const IAT_READ: usize = 0x26a050;

type CreateFileFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
type ReadFn = unsafe extern "C" fn(*mut c_void, *mut u8, i32) -> i32;
type SeekFn = unsafe extern "C" fn(*mut c_void, i32) -> i32;
type HandleFn = unsafe extern "C" fn(*mut c_void) -> i32;
type CreateMappedFn =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut *const u8, *mut u32) -> *mut c_void;
type ReleaseMappedFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;

struct Originals {
    create_file: CreateFileFn,
    read: ReadFn,
    seek: SeekFn,
    get_pos: HandleFn,
    file_length: HandleFn,
    release_file: HandleFn,
    create_mapped: CreateMappedFn,
    release_mapped: ReleaseMappedFn,
}

/// A file opened with `arc_CreateFile`, served from memory.
struct VirtualFile {
    data: Vec<u8>,
    position: usize,
}

static ORIGINALS: OnceLock<Originals> = OnceLock::new();
static TEXTS: OnceLock<FileTexts> = OnceLock::new();
/// Our handle (the address of the boxed file) -> file.
static OPEN: OnceLock<Mutex<HashMap<usize, Box<VirtualFile>>>> = OnceLock::new();
/// Archiver handle of a mapped file -> the translated bytes handed out instead.
static MAPPED: OnceLock<Mutex<HashMap<usize, Vec<u8>>>> = OnceLock::new();

fn lock<T: Default>(cell: &'static OnceLock<Mutex<T>>) -> MutexGuard<'static, T> {
    cell.get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn originals() -> &'static Originals {
    ORIGINALS.get().expect("File hooks not initialized")
}

/// The campaign list asks for a mission whose name differs from the file in the archive
/// (`Entfuehrung` against `Entf\u{fc}hrung`, CP1252 `0xFC`). Returns the archive's spelling.
fn archive_alias(name: &[u8]) -> Option<Vec<u8>> {
    const ASKED: &[u8] = b"Kampagne3_Entfuehrung.p2m";
    const STORED: &[u8] = b"Kampagne3_Entf\xfchrung.p2m";
    let start = name.len().checked_sub(ASKED.len())?;
    if !name[start..].eq_ignore_ascii_case(ASKED) {
        return None;
    }
    let mut alias = name[..start].to_vec();
    alias.extend_from_slice(STORED);
    Some(alias)
}

unsafe fn read_whole(handle: *mut c_void) -> Option<Vec<u8>> {
    let original = originals();
    let length = (original.file_length)(handle);
    if !(0..=16 << 20).contains(&length) {
        return None;
    }
    let mut data = vec![0u8; length as usize];
    let mut filled = 0usize;
    while filled < data.len() {
        let read = (original.read)(
            handle,
            data.as_mut_ptr().add(filled),
            (data.len() - filled) as i32,
        );
        if read <= 0 {
            return None;
        }
        filled += read as usize;
    }
    Some(data)
}

unsafe extern "C" fn create_file(container: *mut c_void, name: *const c_char) -> *mut c_void {
    let original = originals();
    let mut handle = (original.create_file)(container, name);
    if name.is_null() {
        return handle;
    }
    let file_name = CStr::from_ptr(name).to_bytes();
    if handle.is_null() {
        // Only a missing mission is retried, under the name the archive really holds.
        let Some(alias) =
            archive_alias(file_name).and_then(|alias| std::ffi::CString::new(alias).ok())
        else {
            return handle;
        };
        handle = (original.create_file)(container, alias.as_ptr());
        if handle.is_null() {
            return handle;
        }
    }
    let Some(texts) = TEXTS.get() else {
        return handle;
    };
    if classify(&normalize(file_name)).is_none() {
        return handle;
    }
    let Some(data) = read_whole(handle) else {
        // The original handle may have moved; reopen it for the game.
        (original.release_file)(handle);
        return (original.create_file)(container, name);
    };
    let translated = translate_file(file_name, &data, texts);
    (original.release_file)(handle);
    let file = Box::new(VirtualFile {
        data: translated.unwrap_or(data),
        position: 0,
    });
    let key = &*file as *const VirtualFile as usize;
    lock(&OPEN).insert(key, file);
    key as *mut c_void
}

unsafe extern "C" fn read(handle: *mut c_void, buffer: *mut u8, length: i32) -> i32 {
    let mut files = lock(&OPEN);
    let Some(file) = files.get_mut(&(handle as usize)) else {
        drop(files);
        return (originals().read)(handle, buffer, length);
    };
    if length < 0 || buffer.is_null() {
        return -1;
    }
    let count = (length as usize).min(file.data.len().saturating_sub(file.position));
    std::ptr::copy_nonoverlapping(file.data.as_ptr().add(file.position), buffer, count);
    file.position += count;
    count as i32
}

unsafe extern "C" fn seek(handle: *mut c_void, position: i32) -> i32 {
    let mut files = lock(&OPEN);
    let Some(file) = files.get_mut(&(handle as usize)) else {
        drop(files);
        return (originals().seek)(handle, position);
    };
    if position < 0 || position as usize > file.data.len() {
        return -1;
    }
    file.position = position as usize;
    position
}

unsafe extern "C" fn get_pos(handle: *mut c_void) -> i32 {
    let files = lock(&OPEN);
    match files.get(&(handle as usize)) {
        Some(file) => file.position as i32,
        None => {
            drop(files);
            (originals().get_pos)(handle)
        }
    }
}

unsafe extern "C" fn file_length(handle: *mut c_void) -> i32 {
    let files = lock(&OPEN);
    match files.get(&(handle as usize)) {
        Some(file) => file.data.len() as i32,
        None => {
            drop(files);
            (originals().file_length)(handle)
        }
    }
}

unsafe extern "C" fn release_file(handle: *mut c_void) -> i32 {
    if lock(&OPEN).remove(&(handle as usize)).is_some() {
        return 0;
    }
    (originals().release_file)(handle)
}

/// Keeps the archiver's handle, so the archiver still releases its own mapping, and
/// hands the game the translated bytes in place of the mapped ones.
unsafe extern "C" fn create_mapped(
    container: *mut c_void,
    name: *const c_char,
    data: *mut *const u8,
    size: *mut u32,
) -> *mut c_void {
    let handle = (originals().create_mapped)(container, name, data, size);
    if handle.is_null() || name.is_null() || data.is_null() || size.is_null() || (*data).is_null() {
        return handle;
    }
    let Some(texts) = TEXTS.get() else {
        return handle;
    };
    let file_name = CStr::from_ptr(name).to_bytes();
    if classify(&normalize(file_name)).is_none() || *size > 16 << 20 {
        return handle;
    }
    let original = std::slice::from_raw_parts(*data, *size as usize);
    if let Some(translated) = translate_file(file_name, original, texts) {
        *data = translated.as_ptr();
        *size = translated.len() as u32;
        // Moving the Vec into the map keeps its heap buffer where it is.
        lock(&MAPPED).insert(handle as usize, translated);
    }
    handle
}

unsafe extern "C" fn release_mapped(container: *mut c_void, handle: *mut c_void) -> i32 {
    let result = (originals().release_mapped)(container, handle);
    lock(&MAPPED).remove(&(handle as usize));
    result
}

/// IAT patches for the archiver imports; no patches if the language has no file texts.
pub(crate) unsafe fn hook_patches(
    base: usize,
    texts: FileTexts,
) -> Result<Vec<crate::patch::Patch>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let slots: [(usize, usize); 8] = [
        (IAT_CREATE_FILE, create_file as *const () as usize),
        (IAT_READ, read as *const () as usize),
        (IAT_SEEK, seek as *const () as usize),
        (IAT_GET_POS, get_pos as *const () as usize),
        (IAT_FILE_LENGTH, file_length as *const () as usize),
        (IAT_RELEASE_FILE, release_file as *const () as usize),
        (IAT_CREATE_MAPPED, create_mapped as *const () as usize),
        (IAT_RELEASE_MAPPED, release_mapped as *const () as usize),
    ];
    let mut current = [0usize; 8];
    let mut patches = Vec::new();
    for (index, (rva, hook)) in slots.iter().enumerate() {
        let before = crate::read_process_bytes(base + rva, 4)?;
        current[index] = u32::from_le_bytes(before.as_slice().try_into().unwrap()) as usize;
        if current[index] == 0 {
            return Err("Archiver import is not initialized".into());
        }
        patches.push(crate::patch::Patch {
            address: base + rva,
            before,
            after: (*hook as u32).to_le_bytes().to_vec(),
        });
    }
    ORIGINALS
        .set(Originals {
            create_file: std::mem::transmute::<usize, CreateFileFn>(current[0]),
            read: std::mem::transmute::<usize, ReadFn>(current[1]),
            seek: std::mem::transmute::<usize, SeekFn>(current[2]),
            get_pos: std::mem::transmute::<usize, HandleFn>(current[3]),
            file_length: std::mem::transmute::<usize, HandleFn>(current[4]),
            release_file: std::mem::transmute::<usize, HandleFn>(current[5]),
            create_mapped: std::mem::transmute::<usize, CreateMappedFn>(current[6]),
            release_mapped: std::mem::transmute::<usize, ReleaseMappedFn>(current[7]),
        })
        .map_err(|_| "File hooks already initialized")?;
    TEXTS
        .set(texts)
        .map_err(|_| "File texts already initialized")?;
    Ok(patches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misspelled_campaign_mission_name_maps_to_the_archive_file() {
        assert_eq!(
            archive_alias(b"missions_addon\\kampagne3_entfuehrung.P2M").unwrap(),
            b"missions_addon\\Kampagne3_Entf\xfchrung.p2m"
        );
        assert!(archive_alias(b"missions_addon\\Eskorte.p2m").is_none());
        assert!(archive_alias(b"").is_none());
    }
}
