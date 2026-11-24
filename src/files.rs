//! Translated game files.
//!
//! Missions (`*.p2m`) and the ini files (campaign list, tutorial, credits) are read
//! through archiver.dll. The import hooks below read the English original, swap
//! its texts for the configured language and hand the game an in-memory copy.
//! Files the tables do not cover, or that do not match the English originals
//! (for example loose replacement files), reach the game unchanged.
//!
//! Table rows with the scope `p2m` apply to every mission. Rows with the scope
//! `p2m:<file name>` apply to that mission only and win over the generic rows;
//! they cover English sentences that a language translates differently depending
//! on the mission. Only the generic rows must keep the format tokens of the English
//! text; a row for one file may use other tokens, as the shipped file does.

use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    sync::{Mutex, OnceLock},
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
const IAT_CREATE_FILE: usize = 0x26a02c;
/// `arc_Read(handle, buffer, length) -> bytes read`.
const IAT_READ: usize = 0x26a050;

/// Ini files whose `key=value` values are replaced as a whole (`ini` rows).
const INI_FILES: [&str; 3] = [
    "scripts/missions_eng.ini",
    "scripts/tutorial_eng.ini",
    "scripts/credits_eng.ini",
];

/// English text -> translation, as raw CP1252 bytes.
#[derive(Debug, Default)]
pub(crate) struct FileTexts {
    pub mission: HashMap<Vec<u8>, Vec<u8>>,
    /// Per-mission overrides, keyed by the lowercase file name without folders.
    pub mission_files: HashMap<String, HashMap<Vec<u8>, Vec<u8>>>,
    pub ini: HashMap<Vec<u8>, Vec<u8>>,
}

impl FileTexts {
    pub(crate) fn is_empty(&self) -> bool {
        self.mission.is_empty() && self.mission_files.is_empty() && self.ini.is_empty()
    }
}

/// Lowercase file name without folders. Bytes map one to one to characters
/// (Latin-1), which equals CP1252 for the letters game file names use.
fn base_name(name: &[u8]) -> String {
    let name: String = name.iter().map(|byte| *byte as char).collect();
    let name = name.replace('\\', "/");
    name.rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

// ---------------------------------------------------------------------------
// Table format
// ---------------------------------------------------------------------------

/// Decodes a mission table cell: `\n` = CR LF, `\t`, `\\`, `\xHH`, and
/// `{%X:HH}` = format token `%X` with its raw argument byte.
pub(crate) fn decode_mission_text(value: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    let hex = |a: char, b: char| -> Result<u8, String> {
        u8::from_str_radix(&format!("{a}{b}"), 16)
            .map_err(|_| format!("Bad hex escape in: {value}"))
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            // {%X:HH}
            if i + 6 < chars.len()
                && chars[i + 1] == '%'
                && chars[i + 3] == ':'
                && chars[i + 6] == '}'
            {
                out.push(b'%');
                out.extend(super::encode_cp1252(&chars[i + 2].to_string(), "token")?);
                out.push(hex(chars[i + 4], chars[i + 5])?);
                i += 7;
                continue;
            }
            return Err(format!("Malformed token in: {value}"));
        }
        if c == '%' {
            return Err(format!("Bare % outside a token in: {value}"));
        }
        if c == '\\' {
            match chars.get(i + 1) {
                Some('n') => out.extend_from_slice(b"\r\n"),
                Some('t') => out.push(b'\t'),
                Some('\\') => out.push(b'\\'),
                Some('x') if i + 3 < chars.len() => {
                    out.push(hex(chars[i + 2], chars[i + 3])?);
                    i += 4;
                    continue;
                }
                _ => return Err(format!("Bad escape in: {value}")),
            }
            i += 2;
            continue;
        }
        out.extend(super::encode_cp1252(&c.to_string(), "mission text")?);
        i += 1;
    }
    Ok(out)
}

/// Format tokens (`%X` plus argument byte) of a raw mission string, sorted.
pub(crate) fn mission_tokens(raw: &[u8]) -> Vec<[u8; 2]> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' && i + 2 < raw.len() {
            tokens.push([raw[i + 1], raw[i + 2]]);
            i += 3;
        } else {
            i += 1;
        }
    }
    tokens.sort_unstable();
    tokens
}

/// Parses a `scope<TAB>English<TAB>translation` table (`p2m` and `ini` rows).
pub(crate) fn load_file_texts(table: &str) -> Result<FileTexts, String> {
    let table = table.strip_prefix('\u{feff}').unwrap_or(table);
    let mut texts = FileTexts::default();
    for line in table.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [scope, source, translation] = fields.as_slice() else {
            return Err(format!("Malformed file text row: {line}"));
        };
        if source.is_empty() || translation.is_empty() {
            return Err(format!("Empty file text row: {line}"));
        }
        let (map, source, translation) = match *scope {
            scope if scope == "p2m" || scope.starts_with("p2m:") => {
                let source = decode_mission_text(source)?;
                let translation = decode_mission_text(translation)?;
                // Rows for one file are taken verbatim from a shipped mission whose script
                // supplies the arguments, so only the generic rows must keep the tokens.
                if scope == "p2m" && mission_tokens(&source) != mission_tokens(&translation) {
                    return Err(format!("Format tokens differ in mission text: {line}"));
                }
                let map = match scope.strip_prefix("p2m:") {
                    None => &mut texts.mission,
                    Some("") => return Err(format!("Missing file name in scope: {line}")),
                    Some(file) => texts
                        .mission_files
                        .entry(file.to_ascii_lowercase())
                        .or_default(),
                };
                (map, source, translation)
            }
            "ini" => {
                let source = super::encode_cp1252(source, "ini text")?;
                let translation = super::encode_cp1252(translation, "ini text")?;
                if source.contains(&0) || translation.contains(&0) {
                    return Err(format!("NUL byte in ini text row: {line}"));
                }
                (&mut texts.ini, source, translation)
            }
            other => return Err(format!("Unknown file text scope '{other}'")),
        };
        if map.insert(source, translation).is_some() {
            return Err(format!("Duplicate file text row: {line}"));
        }
    }
    Ok(texts)
}

// ---------------------------------------------------------------------------
// Mission scripts (*.p2m)
// ---------------------------------------------------------------------------
//
// u16 statement count n, u16, optionally n*4 bytes 0xCD, n u32 absolute statement
// offsets, then the statements. Jumps address statements by index. Texts live in
//   0xFE <reg> <strings...>        inline message
//   0x1D <a> <b> <strings...>      inline message (rare)
//   0xFF <strings...>              data block, addressed by absolute pointers in
//                                  0x37 <u32 title> <u32 text> and the last u32 of 0xF7
// Strings end with NUL; `%X` is always followed by one raw argument byte.

fn read_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn pointer_slots(statement: &[u8]) -> &'static [usize] {
    match (statement.first(), statement.len()) {
        (Some(0x37), 9) => &[1, 5],
        _ => &[],
    }
}

fn last_u32_slot(statement: &[u8]) -> Option<usize> {
    (statement.first() == Some(&0xf7) && statement.len() >= 5).then(|| statement.len() - 4)
}

/// Splits `statement[start..]` into NUL-terminated strings; `None` if malformed.
fn split_strings(statement: &[u8], start: usize) -> Option<Vec<(usize, usize)>> {
    let mut ranges = Vec::new();
    let mut p = start;
    while p < statement.len() {
        let begin = p;
        loop {
            match *statement.get(p)? {
                0 => break,
                b'%' => p += 3,
                _ => p += 1,
            }
        }
        ranges.push((begin, p));
        p += 1;
    }
    (p == statement.len()).then_some(ranges)
}

/// Returns the translated mission, or `None` if it is malformed or nothing changed.
#[cfg(test)]
pub(crate) fn translate_mission(data: &[u8], texts: &HashMap<Vec<u8>, Vec<u8>>) -> Option<Vec<u8>> {
    translate_mission_with(data, |original| texts.get(original))
}

/// Like `translate_mission`, with the translations supplied by `lookup`.
fn translate_mission_with<'a>(
    data: &[u8],
    lookup: impl Fn(&[u8]) -> Option<&'a Vec<u8>>,
) -> Option<Vec<u8>> {
    let count = u16::from_le_bytes(data.get(0..2)?.try_into().ok()?) as usize;
    if count == 0 {
        return None;
    }
    let padded = data.get(4..4 + 4 * count)?.iter().all(|byte| *byte == 0xcd);
    let table = if padded { 4 + 4 * count } else { 4 };
    let code = table + 4 * count;
    let offsets: Vec<usize> = (0..count)
        .map(|i| read_u32(data, table + 4 * i).map(|value| value as usize))
        .collect::<Option<_>>()?;
    if offsets[0] != code
        || offsets.windows(2).any(|pair| pair[1] <= pair[0])
        || *offsets.last()? >= data.len()
    {
        return None;
    }
    let statements: Vec<&[u8]> = (0..count)
        .map(|i| &data[offsets[i]..offsets.get(i + 1).copied().unwrap_or(data.len())])
        .collect();

    let mut targets = std::collections::HashSet::new();
    for statement in &statements {
        for slot in pointer_slots(statement) {
            targets.insert(read_u32(statement, *slot)? as usize);
        }
        if let Some(slot) = last_u32_slot(statement) {
            targets.insert(read_u32(statement, slot)? as usize);
        }
    }

    let mut bodies = Vec::with_capacity(count);
    let mut moved = HashMap::new(); // old string start -> new string start
    let mut position = code;
    let mut changed = false;
    for (index, statement) in statements.iter().enumerate() {
        let prefix = match statement[0] {
            0xfe if statement.len() >= 2 => Some(2),
            0xff if targets.contains(&(offsets[index] + 1)) => Some(1),
            0x1d if statement.len() > 8 && statement[3..].contains(&0) => Some(3),
            _ => None,
        };
        let Some(prefix) = prefix else {
            bodies.push(statement.to_vec());
            position += statement.len();
            continue;
        };
        let mut body = statement[..prefix].to_vec();
        for (begin, end) in split_strings(statement, prefix)? {
            moved.insert(
                (offsets[index] + begin) as u32,
                (position + body.len()) as u32,
            );
            let original = &statement[begin..end];
            match lookup(original) {
                Some(translation) if !original.is_empty() => {
                    body.extend_from_slice(translation);
                    changed = true;
                }
                _ => body.extend_from_slice(original),
            }
            body.push(0);
        }
        position += body.len();
        bodies.push(body);
    }
    if !changed {
        return None;
    }

    let mut new_offsets = Vec::with_capacity(count);
    let mut at = code;
    for body in &bodies {
        new_offsets.push(at as u32);
        at += body.len();
    }
    for (statement, body) in statements.iter().zip(bodies.iter_mut()) {
        for slot in pointer_slots(statement) {
            let old = read_u32(statement, *slot)?;
            let new = *moved.get(&old)?;
            body[*slot..*slot + 4].copy_from_slice(&new.to_le_bytes());
        }
        if let Some(slot) = last_u32_slot(statement) {
            let old = read_u32(statement, slot)?;
            // Not every 0xF7 ends with a text pointer; other values are plain arguments.
            if let Some(new) = moved.get(&old) {
                body[slot..slot + 4].copy_from_slice(&new.to_le_bytes());
            }
        }
    }

    let mut out = data[..table].to_vec();
    for offset in new_offsets {
        out.extend_from_slice(&offset.to_le_bytes());
    }
    for body in bodies {
        out.extend(body);
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Ini files (campaign list, tutorial, credits)
// ---------------------------------------------------------------------------

/// Replaces `key=value` values found in `texts`; `None` if nothing changed.
pub(crate) fn translate_ini(data: &[u8], texts: &HashMap<Vec<u8>, Vec<u8>>) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 4);
    let mut changed = false;
    for line in data.split_inclusive(|byte| *byte == b'\n') {
        let content_end = line.len()
            - line
                .iter()
                .rev()
                .take_while(|byte| **byte == b'\n' || **byte == b'\r')
                .count();
        let (content, ending) = line.split_at(content_end);
        if let Some(equals) = content.iter().position(|byte| *byte == b'=') {
            if let Some(translation) = texts.get(&content[equals + 1..]) {
                out.extend_from_slice(&content[..=equals]);
                out.extend_from_slice(translation);
                out.extend_from_slice(ending);
                changed = true;
                continue;
            }
        }
        out.extend_from_slice(line);
    }
    changed.then_some(out)
}

// ---------------------------------------------------------------------------
// Archiver import hooks
// ---------------------------------------------------------------------------

type CreateFileFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
type ReadFn = unsafe extern "C" fn(*mut c_void, *mut u8, i32) -> i32;
type SeekFn = unsafe extern "C" fn(*mut c_void, i32) -> i32;
type HandleFn = unsafe extern "C" fn(*mut c_void) -> i32;

struct Originals {
    create_file: CreateFileFn,
    read: ReadFn,
    seek: SeekFn,
    get_pos: HandleFn,
    file_length: HandleFn,
    release_file: HandleFn,
}

struct VirtualFile {
    data: Vec<u8>,
    position: usize,
}

static ORIGINALS: OnceLock<Originals> = OnceLock::new();
static TEXTS: OnceLock<FileTexts> = OnceLock::new();
static OPEN: OnceLock<Mutex<HashMap<usize, Box<VirtualFile>>>> = OnceLock::new();

fn open_files() -> std::sync::MutexGuard<'static, HashMap<usize, Box<VirtualFile>>> {
    OPEN.get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn originals() -> &'static Originals {
    ORIGINALS.get().expect("File hooks not initialized")
}

enum Kind {
    Mission,
    Ini,
}

fn classify(name: &[u8]) -> Option<Kind> {
    let name = String::from_utf8_lossy(name)
        .to_ascii_lowercase()
        .replace('\\', "/");
    let name = name.trim_start_matches("./");
    if name.ends_with(".p2m") {
        Some(Kind::Mission)
    } else if INI_FILES.contains(&name) {
        Some(Kind::Ini)
    } else {
        None
    }
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

/// Translates a mission; rows for this very file win over the generic ones.
fn translate_mission_for(data: &[u8], file: &str, texts: &FileTexts) -> Option<Vec<u8>> {
    let specific = texts.mission_files.get(file);
    translate_mission_with(data, |original| {
        specific
            .and_then(|rows| rows.get(original))
            .or_else(|| texts.mission.get(original))
    })
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
    if handle.is_null() {
        // Only a missing mission is retried, under the name the archive really holds.
        let Some(alias) = archive_alias(CStr::from_ptr(name).to_bytes())
            .and_then(|alias| std::ffi::CString::new(alias).ok())
        else {
            return handle;
        };
        handle = (original.create_file)(container, alias.as_ptr());
        if handle.is_null() {
            return handle;
        }
    }
    let (Some(kind), Some(texts)) = (classify(CStr::from_ptr(name).to_bytes()), TEXTS.get()) else {
        return handle;
    };
    let Some(data) = read_whole(handle) else {
        // The original handle may have moved; reopen it for the game.
        (original.release_file)(handle);
        return (original.create_file)(container, name);
    };
    let translated = match kind {
        Kind::Mission => {
            translate_mission_for(&data, &base_name(CStr::from_ptr(name).to_bytes()), texts)
        }
        Kind::Ini => translate_ini(&data, &texts.ini),
    };
    (original.release_file)(handle);
    let file = Box::new(VirtualFile {
        data: translated.unwrap_or(data),
        position: 0,
    });
    let key = &*file as *const VirtualFile as usize;
    open_files().insert(key, file);
    key as *mut c_void
}

unsafe extern "C" fn read(handle: *mut c_void, buffer: *mut u8, length: i32) -> i32 {
    let mut files = open_files();
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
    let mut files = open_files();
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
    let files = open_files();
    match files.get(&(handle as usize)) {
        Some(file) => file.position as i32,
        None => {
            drop(files);
            (originals().get_pos)(handle)
        }
    }
}

unsafe extern "C" fn file_length(handle: *mut c_void) -> i32 {
    let files = open_files();
    match files.get(&(handle as usize)) {
        Some(file) => file.data.len() as i32,
        None => {
            drop(files);
            (originals().file_length)(handle)
        }
    }
}

unsafe extern "C" fn release_file(handle: *mut c_void) -> i32 {
    if open_files().remove(&(handle as usize)).is_some() {
        return 0;
    }
    (originals().release_file)(handle)
}

/// IAT patches for the archiver imports; no patches if the language has no file texts.
pub(crate) unsafe fn hook_patches(
    base: usize,
    texts: FileTexts,
) -> Result<Vec<crate::patch::Patch>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let slots: [(usize, usize); 6] = [
        (IAT_CREATE_FILE, create_file as *const () as usize),
        (IAT_READ, read as *const () as usize),
        (IAT_SEEK, seek as *const () as usize),
        (IAT_GET_POS, get_pos as *const () as usize),
        (IAT_FILE_LENGTH, file_length as *const () as usize),
        (IAT_RELEASE_FILE, release_file as *const () as usize),
    ];
    let mut current = [0usize; 6];
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
    use crate::language::LANGUAGES;

    fn table(rows: &[(&[u8], &[u8])]) -> HashMap<Vec<u8>, Vec<u8>> {
        rows.iter().map(|(a, b)| (a.to_vec(), b.to_vec())).collect()
    }

    /// Two statements: 0x37 <title> <text>, then the 0xFF data block.
    fn data_block_mission() -> Vec<u8> {
        let mut data = vec![2, 0, 1, 0];
        let code = 4 + 8;
        let block = code + 9;
        data.extend((code as u32).to_le_bytes());
        data.extend((block as u32).to_le_bytes());
        data.push(0x37);
        data.extend(((block + 1) as u32).to_le_bytes());
        data.extend(((block + 1 + 6) as u32).to_le_bytes());
        data.push(0xff);
        data.extend(b"Title\0Hi %s\x07!\0");
        data
    }

    #[test]
    fn mission_data_block_pointers_follow_their_texts() {
        let data = data_block_mission();
        let texts = table(&[
            (b"Title", b"Longer title"),
            (b"Hi %s\x07!", b"Hallo %s\x07!"),
        ]);
        let out = translate_mission(&data, &texts).unwrap();
        let block = u32::from_le_bytes(out[8..12].try_into().unwrap()) as usize;
        assert_eq!(out[block], 0xff);
        let title = u32::from_le_bytes(out[13..17].try_into().unwrap()) as usize;
        let text = u32::from_le_bytes(out[17..21].try_into().unwrap()) as usize;
        assert_eq!(&out[title..title + 13], b"Longer title\0");
        assert_eq!(&out[text..text + 11], b"Hallo %s\x07!\0");
        assert_eq!(out.len(), block + 1 + 13 + 11);
    }

    #[test]
    fn mission_inline_text_with_nul_token_argument() {
        // 0xFE statement after a jump statement; `%1` carries a NUL argument byte.
        let mut data = vec![2, 0, 0, 0];
        data.extend(12u32.to_le_bytes());
        data.extend(16u32.to_le_bytes());
        data.extend([0xfd, 0x0b, 0x01, 0x00]);
        data.extend(b"\xfe\x07Head\0Dear %1\x00 friend\0");
        let texts = table(&[(b"Dear %1\x00 friend", b"Lieber %1\x00 Freund")]);
        let out = translate_mission(&data, &texts).unwrap();
        assert_eq!(&out[..16], &data[..16]);
        assert_eq!(&out[16..], b"\xfe\x07Head\0Lieber %1\x00 Freund\0");
        assert!(translate_mission(&data, &HashMap::new()).is_none());
    }

    #[test]
    fn malformed_missions_are_left_alone() {
        let mut data = data_block_mission();
        data.pop(); // unterminated text
        let texts = table(&[(b"Title", b"X")]);
        assert!(translate_mission(&data, &texts).is_none());
        assert!(translate_mission(&[], &texts).is_none());
    }

    #[test]
    fn mission_rows_for_one_file_win_over_generic_rows() {
        let data = data_block_mission();
        let texts = load_file_texts(
            "p2m\tTitle\tAllgemein\np2m:Kampagne3_Entf\u{fc}hrung.p2m\tTitle\tNur hier\n",
        )
        .unwrap();
        // The game passes file names as CP1252 bytes.
        let specific = base_name(b"missions_addon\\Kampagne3_Entf\xfchrung.p2m");
        let other = base_name(b"missions\\000010_m.p2m");
        let specific = translate_mission_for(&data, &specific, &texts).unwrap();
        let generic = translate_mission_for(&data, &other, &texts).unwrap();
        assert!(specific.windows(8).any(|w| w == b"Nur hier"));
        assert!(!specific.windows(9).any(|w| w == b"Allgemein"));
        assert!(generic.windows(9).any(|w| w == b"Allgemein"));
        assert_eq!(base_name(b"./Missions/ABC.P2M"), "abc.p2m");
        assert!(load_file_texts("p2m:\tTitle\tX\n").is_err());
        assert!(load_file_texts("p2m:a.p2m\tTitle\tX\np2m:A.p2m\tTitle\tY\n").is_err());
    }

    #[test]
    fn ini_values_are_replaced_line_by_line() {
        let ini = b"[Campaign_1]\r\nName=The Fire\r\nTexture=41101\r\nText=Burn\n";
        let texts = table(&[(b"The Fire", b"Das Feuer"), (b"Burn", b"Brand")]);
        let out = translate_ini(ini, &texts).unwrap();
        assert_eq!(
            out,
            b"[Campaign_1]\r\nName=Das Feuer\r\nTexture=41101\r\nText=Brand\n"
        );
    }

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
    fn mission_text_codec() {
        assert_eq!(
            decode_mission_text(r"A {%t:08}\n\\f1_ \t \x01 ä").unwrap(),
            b"A %t\x08\r\n\\f1_ \t \x01 \xe4"
        );
        assert!(decode_mission_text("50% off").is_err());
        assert!(decode_mission_text("{%t:8}").is_err());
    }

    #[test]
    fn file_tables_load_and_only_translate_extracted_english() {
        let source = include_str!("../data/files/source.tsv");
        let english: std::collections::HashSet<(&str, &str)> = source
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.split_once('\t').unwrap())
            .collect();
        const FONT_EXTRA: &str = "äöüÄÖÜßáàâéèêíìîóòôúùû";
        for language in LANGUAGES {
            load_file_texts(language.files).unwrap_or_else(|e| panic!("{}: {e}", language.code));
            for line in language
                .files
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
            {
                let mut fields = line.split('\t');
                let (scope, source, translation) = (
                    fields.next().unwrap(),
                    fields.next().unwrap(),
                    fields.next().unwrap(),
                );
                let scope = if scope.starts_with("p2m:") {
                    "p2m"
                } else {
                    scope
                };
                assert!(
                    english.contains(&(scope, source)),
                    "{}: unknown source {source}",
                    language.code
                );
                if language.code != "de" {
                    if let Some(c) = translation
                        .chars()
                        .find(|c| !(c.is_ascii() || FONT_EXTRA.contains(*c)))
                    {
                        panic!("{}: {c:?} has no glyph in: {translation}", language.code);
                    }
                }
            }
        }
    }
}
