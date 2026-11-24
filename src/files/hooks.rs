//! Hooks in archiver.dll. Every reader of the archive (the game, the renderer
//! `ddraw_Dll.dll`, other mods) goes through the exports hooked here, so each file
//! is handed to `translate_file` once, whoever opens it.
//!
//! Each hooked export starts with `push esi; mov esi, [esp+N]` (5 bytes, two whole
//! instructions). The entry is checked, replaced by a jump to the hook, and copied
//! into a trampoline that runs the original. A file the hook replaces is served from
//! memory under a handle of our own; every export that takes a file handle is
//! hooked, so the archiver never sees such a handle.

use super::{classify, normalize, translate_file, FileTexts};
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr, CString},
    sync::{Mutex, MutexGuard, OnceLock},
};
use windows::core::PCSTR;
use windows::Win32::System::{
    LibraryLoader::{GetModuleHandleA, GetProcAddress},
    Memory::{VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
};

/// Entry of the exports whose first argument sits at `[esp+8]` after `push esi`.
const ENTRY: [u8; 5] = [0x56, 0x8b, 0x74, 0x24, 0x08];
/// Entry of `cpr_OpenFile(name, container)`, which loads its second argument.
const ENTRY_SECOND: [u8; 5] = [0x56, 0x8b, 0x74, 0x24, 0x0c];
const JMP: u8 = 0xe9;

type CreateFileFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
type OpenFileFn = unsafe extern "C" fn(*const c_char, *mut c_void) -> *mut c_void;
type CreateMappedFn =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut *const u8, *mut u32) -> *mut c_void;
type ReleaseMappedFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
type ReadFn = unsafe extern "C" fn(*mut c_void, *mut u8, i32) -> i32;
type SeekFn = unsafe extern "C" fn(*mut c_void, i32) -> i32;
type SeekFileFn = unsafe extern "C" fn(*mut c_void, i32, i32) -> i32;
type HandleFn = unsafe extern "C" fn(*mut c_void) -> i32;
type NameFn = unsafe extern "C" fn(*mut c_void) -> *const c_char;

/// The original exports, reached through their trampolines.
struct Originals {
    create_file: CreateFileFn,
    open_file: OpenFileFn,
    create_mapped: CreateMappedFn,
    release_mapped: ReleaseMappedFn,
    read: ReadFn,
    seek: SeekFn,
    seek_file: SeekFileFn,
    get_pos: HandleFn,
    file_length: HandleFn,
    file_name: NameFn,
    release_file: HandleFn,
}

/// A file served from memory.
struct VirtualFile {
    name: CString,
    data: Vec<u8>,
    position: usize,
}

static ORIGINALS: OnceLock<Originals> = OnceLock::new();
static TEXTS: OnceLock<FileTexts> = OnceLock::new();
/// Our handle (the address of the boxed file) -> file.
static OPEN: OnceLock<Mutex<HashMap<usize, Box<VirtualFile>>>> = OnceLock::new();
/// Archiver handle of a mapped file -> the bytes handed out instead.
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

/// Opens `name` with `call` (one of the archiver's open functions) and serves a
/// listed file from memory.
unsafe fn open(name: *const c_char, call: impl Fn(*const c_char) -> *mut c_void) -> *mut c_void {
    let mut handle = call(name);
    if name.is_null() {
        return handle;
    }
    let file_name = CStr::from_ptr(name).to_bytes();
    // The name the archive knows the file by: the asked one or its alias.
    let mut opened = CStr::from_ptr(name).to_owned();
    if handle.is_null() {
        // Only a missing mission is retried, under the name the archive really holds.
        let Some(alias) = archive_alias(file_name).and_then(|alias| CString::new(alias).ok())
        else {
            return handle;
        };
        handle = call(alias.as_ptr());
        if handle.is_null() {
            return handle;
        }
        opened = alias;
    }
    let Some(texts) = TEXTS.get() else {
        return handle;
    };
    if classify(&normalize(file_name)).is_none() {
        return handle;
    }
    let original = originals();
    let Some(data) = read_whole(handle) else {
        // The original handle may have moved; reopen it for the caller.
        (original.release_file)(handle);
        return call(opened.as_ptr());
    };
    let Some(translated) = translate_file(file_name, &data, texts) else {
        (original.release_file)(handle);
        return call(opened.as_ptr());
    };
    (original.release_file)(handle);
    let file = Box::new(VirtualFile {
        name: opened,
        data: translated,
        position: 0,
    });
    let key = &*file as *const VirtualFile as usize;
    lock(&OPEN).insert(key, file);
    key as *mut c_void
}

unsafe extern "C" fn create_file(container: *mut c_void, name: *const c_char) -> *mut c_void {
    open(name, |name| (originals().create_file)(container, name))
}

unsafe extern "C" fn open_file(name: *const c_char, container: *mut c_void) -> *mut c_void {
    open(name, |name| (originals().open_file)(name, container))
}

/// Runs `virtual_call` for one of our handles, `original` for every other.
unsafe fn with_file<R>(
    handle: *mut c_void,
    virtual_call: impl FnOnce(&mut VirtualFile) -> R,
    original: impl FnOnce() -> R,
) -> R {
    let mut files = lock(&OPEN);
    match files.get_mut(&(handle as usize)) {
        Some(file) => virtual_call(file),
        None => {
            drop(files);
            original()
        }
    }
}

/// Moves the position of a virtual file; `-1` outside the file, like the archiver.
fn set_position(file: &mut VirtualFile, position: i64) -> i32 {
    if !(0..=file.data.len() as i64).contains(&position) {
        return -1;
    }
    file.position = position as usize;
    position as i32
}

unsafe extern "C" fn read(handle: *mut c_void, buffer: *mut u8, length: i32) -> i32 {
    with_file(
        handle,
        |file| {
            if length < 0 || buffer.is_null() {
                return -1;
            }
            let count = (length as usize).min(file.data.len().saturating_sub(file.position));
            std::ptr::copy_nonoverlapping(file.data.as_ptr().add(file.position), buffer, count);
            file.position += count;
            count as i32
        },
        || (originals().read)(handle, buffer, length),
    )
}

unsafe extern "C" fn seek(handle: *mut c_void, position: i32) -> i32 {
    with_file(
        handle,
        |file| set_position(file, position as i64),
        || (originals().seek)(handle, position),
    )
}

/// `cpr_SeekFile(handle, offset, origin)`: origin 0 = start, 1 = current, 2 = end;
/// returns the new position.
unsafe extern "C" fn seek_file(handle: *mut c_void, offset: i32, origin: i32) -> i32 {
    with_file(
        handle,
        |file| {
            let base = match origin {
                1 => file.position as i64,
                2 => file.data.len() as i64,
                _ => 0,
            };
            set_position(file, base + offset as i64)
        },
        || (originals().seek_file)(handle, offset, origin),
    )
}

unsafe extern "C" fn get_pos(handle: *mut c_void) -> i32 {
    with_file(
        handle,
        |file| file.position as i32,
        || (originals().get_pos)(handle),
    )
}

unsafe extern "C" fn file_length(handle: *mut c_void) -> i32 {
    with_file(
        handle,
        |file| file.data.len() as i32,
        || (originals().file_length)(handle),
    )
}

unsafe extern "C" fn file_name(handle: *mut c_void) -> *const c_char {
    with_file(
        handle,
        |file| file.name.as_ptr(),
        || (originals().file_name)(handle),
    )
}

unsafe extern "C" fn release_file(handle: *mut c_void) -> i32 {
    if lock(&OPEN).remove(&(handle as usize)).is_some() {
        return 0;
    }
    (originals().release_file)(handle)
}

/// Keeps the archiver's handle, so the archiver still releases its own mapping, and
/// hands the caller the replaced bytes in place of the mapped ones.
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
    if let Some(replaced) = translate_file(file_name, original, texts) {
        *data = replaced.as_ptr();
        *size = replaced.len() as u32;
        // Moving the Vec into the map keeps its heap buffer where it is.
        lock(&MAPPED).insert(handle as usize, replaced);
    }
    handle
}

unsafe extern "C" fn release_mapped(container: *mut c_void, handle: *mut c_void) -> i32 {
    let result = (originals().release_mapped)(container, handle);
    lock(&MAPPED).remove(&(handle as usize));
    result
}

/// `jmp target` written at `from`.
fn jump(from: usize, target: usize) -> [u8; 5] {
    let mut code = [JMP, 0, 0, 0, 0];
    code[1..].copy_from_slice(&(target.wrapping_sub(from + 5) as u32).to_le_bytes());
    code
}

/// Address of an archiver export.
unsafe fn export(module: windows::Win32::Foundation::HMODULE, name: &str) -> Result<usize, String> {
    let name = CString::new(name).map_err(|error| error.to_string())?;
    GetProcAddress(module, PCSTR(name.as_ptr().cast()))
        .map(|function| function as usize)
        .ok_or_else(|| format!("archiver.dll has no export {}", name.to_string_lossy()))
}

/// Patches for the archiver exports; no patches if the language replaces no file.
/// Fails without changing anything if an export does not start as expected (another
/// archiver version, or a hook of another mod).
pub(crate) unsafe fn hook_patches(texts: FileTexts) -> Result<Vec<crate::patch::Patch>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let module = GetModuleHandleA(PCSTR(c"archiver.dll".as_ptr().cast()))
        .map_err(|error| format!("archiver.dll is not loaded: {error}"))?;
    // (export, expected entry, hook); aliases that share the code must resolve to it.
    let hooks: [(&str, [u8; 5], usize, &[&str]); 11] = [
        (
            "arc_CreateFile",
            ENTRY,
            create_file as *const () as usize,
            &[],
        ),
        (
            "cpr_OpenFile",
            ENTRY_SECOND,
            open_file as *const () as usize,
            &[],
        ),
        (
            "arc_CreateMemMapped",
            ENTRY,
            create_mapped as *const () as usize,
            &[],
        ),
        (
            "arc_ReleaseMemMapped",
            ENTRY,
            release_mapped as *const () as usize,
            &[],
        ),
        (
            "arc_Read",
            ENTRY,
            read as *const () as usize,
            &["cpr_ReadFromFile"],
        ),
        ("arc_Seek", ENTRY, seek as *const () as usize, &[]),
        ("cpr_SeekFile", ENTRY, seek_file as *const () as usize, &[]),
        ("arc_GetPos", ENTRY, get_pos as *const () as usize, &[]),
        (
            "arc_FileLength",
            ENTRY,
            file_length as *const () as usize,
            &["cpr_GetFileLength"],
        ),
        ("arc_FileName", ENTRY, file_name as *const () as usize, &[]),
        (
            "arc_ReleaseFile",
            ENTRY,
            release_file as *const () as usize,
            &["cpr_CloseFile"],
        ),
    ];
    let trampolines = VirtualAlloc(
        None,
        0x1000,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE,
    ) as usize;
    if trampolines == 0 {
        return Err(format!(
            "VirtualAlloc for trampolines: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut patches = Vec::new();
    let mut original = [0usize; 11];
    for (index, (name, entry, hook, aliases)) in hooks.iter().enumerate() {
        let target = export(module, name)?;
        for alias in *aliases {
            if export(module, alias)? != target {
                return Err(format!("archiver.dll: {alias} is no longer {name}"));
            }
        }
        let before = crate::read_process_bytes(target, entry.len())?;
        if before != entry {
            return Err(format!(
                "archiver.dll: {name} starts with {before:02X?}, expected {entry:02X?}"
            ));
        }
        // Trampoline: the copied entry, then back behind it.
        let trampoline = trampolines + index * 16;
        let code = (trampoline as *mut u8).cast::<[u8; 10]>();
        let mut bytes = [0u8; 10];
        bytes[..5].copy_from_slice(entry);
        bytes[5..].copy_from_slice(&jump(trampoline + 5, target + 5));
        code.write(bytes);
        original[index] = trampoline;
        patches.push(crate::patch::Patch {
            address: target,
            before,
            after: jump(target, *hook).to_vec(),
        });
    }
    ORIGINALS
        .set(Originals {
            create_file: std::mem::transmute::<usize, CreateFileFn>(original[0]),
            open_file: std::mem::transmute::<usize, OpenFileFn>(original[1]),
            create_mapped: std::mem::transmute::<usize, CreateMappedFn>(original[2]),
            release_mapped: std::mem::transmute::<usize, ReleaseMappedFn>(original[3]),
            read: std::mem::transmute::<usize, ReadFn>(original[4]),
            seek: std::mem::transmute::<usize, SeekFn>(original[5]),
            seek_file: std::mem::transmute::<usize, SeekFileFn>(original[6]),
            get_pos: std::mem::transmute::<usize, HandleFn>(original[7]),
            file_length: std::mem::transmute::<usize, HandleFn>(original[8]),
            file_name: std::mem::transmute::<usize, NameFn>(original[9]),
            release_file: std::mem::transmute::<usize, HandleFn>(original[10]),
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

    #[test]
    fn jumps_are_relative_to_the_next_instruction() {
        assert_eq!(jump(0x1000, 0x1005), [JMP, 0, 0, 0, 0]);
        assert_eq!(jump(0x1000, 0x1000), [JMP, 0xfb, 0xff, 0xff, 0xff]);
        assert_eq!(jump(0x1000_2020, 0x1000_3000), [JMP, 0xdb, 0x0f, 0, 0]);
    }

    #[test]
    fn virtual_files_seek_like_the_archiver() {
        let mut file = VirtualFile {
            name: CString::new("x").unwrap(),
            data: vec![0; 10],
            position: 0,
        };
        assert_eq!(set_position(&mut file, 4), 4);
        assert_eq!(set_position(&mut file, 10), 10);
        assert_eq!(set_position(&mut file, 11), -1);
        assert_eq!(set_position(&mut file, -1), -1);
        assert_eq!(file.position, 10);
    }
}
