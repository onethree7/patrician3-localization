//! Maps (`maps/*.gmf`): the town names the game shows on the sea chart, in the
//! title bar and in every text that names a town.
//!
//! A map file is `i32 size` followed by a bit stream (32-bit little-endian words,
//! least significant bit first), the format Patrician3.exe decodes at 0x6387E0:
//!   flag 0: literal byte (8 bits)
//!   flag 1: 3-bit class c, distance = bits(c + 1) + 2^(c + 1) - 1, length = 2 plus
//!           chunks of 2, 3, 4, ... bits, continuing while a chunk is all ones.
//! A negative size means the data follows uncompressed; the map loader does not
//! accept that, so a translated map is written back as a stream of literals.
//!
//! The decoded map (loader at 0x5449F4): u32 town count n, an optional block
//! `u16 0xAFFE, u16 length, length bytes`, n*4, n*4, n, n and n bytes of town data,
//! then 40 NUL-terminated town names.

use super::Lookup;

/// The game always stores 40 town names, also for maps with fewer towns.
const TOWN_NAMES: usize = 40;
/// Upper bound for a decoded map; real maps have well under 1 KiB.
const MAX_SIZE: usize = 1 << 20;

fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    let size = i32::from_le_bytes(data.get(..4)?.try_into().ok()?);
    if size <= 0 {
        let raw = data.get(4..4 + size.unsigned_abs() as usize)?;
        return Some(raw.to_vec());
    }
    let size = size as usize;
    if size > MAX_SIZE {
        return None;
    }
    let mut position = 4;
    let mut buffer = 0u64;
    let mut available = 0u32;
    let mut bits = |count: u32| -> Option<usize> {
        while available < count {
            let word = u32::from_le_bytes(data.get(position..position + 4)?.try_into().ok()?);
            buffer |= u64::from(word) << available;
            position += 4;
            available += 32;
        }
        let value = (buffer & ((1 << count) - 1)) as usize;
        buffer >>= count;
        available -= count;
        Some(value)
    };
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        if bits(1)? == 0 {
            out.push(bits(8)? as u8);
            continue;
        }
        let class = bits(3)? as u32;
        let distance = bits(class + 1)? + (1 << (class + 1)) - 1;
        let mut length = 2;
        let mut width = 2;
        loop {
            let chunk = bits(width)?;
            length += chunk;
            if chunk != (1 << width) - 1 {
                break;
            }
            width += 1;
            if width > 24 {
                return None;
            }
        }
        if distance > out.len() || out.len() + length > size {
            return None;
        }
        for _ in 0..length {
            out.push(out[out.len() - distance]);
        }
    }
    Some(out)
}

/// Encodes `raw` as literals only, which the game's decoder reads like any map.
fn compress(raw: &[u8]) -> Vec<u8> {
    let mut out = (raw.len() as i32).to_le_bytes().to_vec();
    let mut buffer = 0u64;
    let mut used = 0u32;
    for byte in raw {
        // Flag bit 0, then the byte, least significant bit first.
        buffer |= u64::from(*byte) << (used + 1);
        used += 9;
        while used >= 32 {
            out.extend_from_slice(&(buffer as u32).to_le_bytes());
            buffer >>= 32;
            used -= 32;
        }
    }
    out.extend_from_slice(&(buffer as u32).to_le_bytes());
    // The decoder loads the next word as soon as the current one is used up.
    out.extend_from_slice(&[0; 4]);
    out
}

/// Offset of the first town name in a decoded map.
fn names_offset(raw: &[u8]) -> Option<usize> {
    let towns = u32::from_le_bytes(raw.get(..4)?.try_into().ok()?) as usize;
    if towns > TOWN_NAMES {
        return None;
    }
    let mut at = 4;
    if raw.get(at..at + 2)? == [0xfe, 0xaf] {
        at += 4 + u16::from_le_bytes(raw.get(at + 2..at + 4)?.try_into().ok()?) as usize;
    }
    Some(at + 11 * towns)
}

/// Town names of a map file, for the extraction tests.
#[cfg(test)]
pub(super) fn town_names(data: &[u8]) -> Option<Vec<Vec<u8>>> {
    let raw = decompress(data)?;
    let mut at = names_offset(&raw)?;
    let mut names = Vec::with_capacity(TOWN_NAMES);
    for _ in 0..TOWN_NAMES {
        let end = at + raw.get(at..)?.iter().position(|byte| *byte == 0)?;
        names.push(raw[at..end].to_vec());
        at = end + 1;
    }
    Some(names)
}

/// Replaces town names; `None` if the map is malformed or nothing changed.
pub(super) fn translate<'t>(data: &[u8], lookup: &Lookup<'t>) -> Option<Vec<u8>> {
    let raw = decompress(data)?;
    let mut at = names_offset(&raw)?;
    let mut out = raw.get(..at)?.to_vec();
    let mut changed = false;
    for _ in 0..TOWN_NAMES {
        let end = at + raw.get(at..)?.iter().position(|byte| *byte == 0)?;
        let name = &raw[at..end];
        match lookup(name) {
            Some(translation) if !name.is_empty() => {
                out.extend_from_slice(translation);
                changed = true;
            }
            _ => out.extend_from_slice(name),
        }
        out.push(0);
        at = end + 1;
    }
    out.extend_from_slice(&raw[at..]);
    changed.then(|| compress(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A map with two towns and the 0xAFFE block, as the game writes it.
    fn raw_map() -> Vec<u8> {
        let mut raw = 2u32.to_le_bytes().to_vec();
        raw.extend([0xfe, 0xaf, 1, 0, 0]);
        raw.extend([7u8; 22]);
        for index in 0..TOWN_NAMES {
            raw.extend(format!("Town{index}\0").bytes());
        }
        raw.extend(b"Luebeck\0");
        raw
    }

    #[test]
    fn literal_stream_round_trips() {
        let raw = raw_map();
        assert_eq!(decompress(&compress(&raw)).unwrap(), raw);
        assert_eq!(decompress(&compress(b"")).unwrap(), b"");
    }

    #[test]
    fn back_references_decode() {
        // "ab" as literals, then class 0 (distance 2) with length 2 + 2 = 4: "ababab".
        let mut bits: Vec<u8> = Vec::new();
        let mut push =
            |value: u32, count: u32| (0..count).for_each(|i| bits.push((value >> i & 1) as u8));
        push(0, 1);
        push(b'a'.into(), 8);
        push(0, 1);
        push(b'b'.into(), 8);
        push(1, 1);
        push(0, 3);
        push(1, 1);
        push(2, 2);
        let mut data = 6i32.to_le_bytes().to_vec();
        let mut words = vec![0u8; bits.len().div_ceil(32) * 4 + 4];
        for (index, bit) in bits.iter().enumerate() {
            words[index / 8] |= bit << (index % 8);
        }
        data.extend(words);
        assert_eq!(decompress(&data).unwrap(), b"ababab");
    }

    #[test]
    fn town_names_are_replaced() {
        let data = compress(&raw_map());
        let rows: [(&[u8], &[u8]); 1] = [(b"Town3", b"L\xfcbeck")];
        let lookup = |name: &[u8]| rows.iter().find(|(a, _)| *a == name).map(|(_, b)| *b);
        let names = town_names(&translate(&data, &lookup).unwrap()).unwrap();
        assert_eq!(names[3], b"L\xfcbeck");
        assert_eq!(names[39], b"Town39");
        let raw = decompress(&translate(&data, &lookup).unwrap()).unwrap();
        assert!(raw.ends_with(b"Town39\0Luebeck\0"));
        assert!(translate(&data, &|_| None).is_none());
    }
}
