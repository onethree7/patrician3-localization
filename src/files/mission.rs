//! Mission scripts (`*.p2m`) and the text codec of their table cells.
//!
//! u16 statement count n, u16, optionally n*4 bytes 0xCD, n u32 absolute statement
//! offsets, then the statements. Jumps address statements by index. Texts live in
//!   0xFE <reg> <strings...>        inline message
//!   0x1D <a> <b> <strings...>      inline message (rare)
//!   0xFF <strings...>              data block, addressed by absolute pointers in
//!                                  0x37 <u32 title> <u32 text> and the last u32 of 0xF7
//! Strings end with NUL; `%X` is always followed by one raw argument byte.

use std::collections::{HashMap, HashSet};

/// Decodes a mission table cell: `\n` = CR LF, `\t`, `\\`, `\xHH`, and
/// `{%X:HH}` = format token `%X` with its raw argument byte.
pub(super) fn decode_mission_text(value: &str) -> Result<Vec<u8>, String> {
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
                out.extend(crate::encode_cp1252(&chars[i + 2].to_string(), "token")?);
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
        out.extend(crate::encode_cp1252(&c.to_string(), "mission text")?);
        i += 1;
    }
    Ok(out)
}

/// Format tokens (`%X` plus argument byte) of a raw mission string, sorted.
pub(super) fn mission_tokens(raw: &[u8]) -> Vec<[u8; 2]> {
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

/// `translate` with a plain table.
#[cfg(test)]
pub(super) fn translate_mission(data: &[u8], texts: &HashMap<Vec<u8>, Vec<u8>>) -> Option<Vec<u8>> {
    translate(data, &|original| texts.get(original).map(Vec::as_slice))
}

/// Returns the translated mission, or `None` if it is malformed or nothing changed.
pub(super) fn translate<'t>(data: &[u8], lookup: &super::Lookup<'t>) -> Option<Vec<u8>> {
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

    let mut targets = HashSet::new();
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn mission_text_codec() {
        assert_eq!(
            decode_mission_text(r"A {%t:08}\n\\f1_ \t \x01 ä").unwrap(),
            b"A %t\x08\r\n\\f1_ \t \x01 \xe4"
        );
        assert!(decode_mission_text("50% off").is_err());
        assert!(decode_mission_text("{%t:8}").is_err());
    }
}
