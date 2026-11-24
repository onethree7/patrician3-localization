//! Plain text files: ini files (`key=value`) and name lists (one entry per line).

use super::Lookup;

/// Splits off the line ending (any trailing CR and LF bytes).
fn split_ending(line: &[u8]) -> (&[u8], &[u8]) {
    let trailing = line
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\n' || **byte == b'\r')
        .count();
    line.split_at(line.len() - trailing)
}

/// Replaces whole `key=value` values; `None` if nothing changed.
pub(super) fn translate_ini<'t>(data: &[u8], lookup: &Lookup<'t>) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 4);
    let mut changed = false;
    for line in data.split_inclusive(|byte| *byte == b'\n') {
        let (content, ending) = split_ending(line);
        if let Some(equals) = content.iter().position(|byte| *byte == b'=') {
            if let Some(translation) = lookup(&content[equals + 1..]) {
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

/// Replaces whole lines; `None` if nothing changed.
pub(super) fn translate_lines<'t>(data: &[u8], lookup: &Lookup<'t>) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 4);
    let mut changed = false;
    for line in data.split_inclusive(|byte| *byte == b'\n') {
        let (content, ending) = split_ending(line);
        match lookup(content) {
            Some(translation) if !content.is_empty() => {
                out.extend_from_slice(translation);
                out.extend_from_slice(ending);
                changed = true;
            }
            _ => out.extend_from_slice(line),
        }
    }
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'t>(rows: &'t [(&'t [u8], &'t [u8])]) -> impl Fn(&[u8]) -> Option<&'t [u8]> {
        move |text| rows.iter().find(|(a, _)| *a == text).map(|(_, b)| *b)
    }

    #[test]
    fn ini_values_are_replaced_line_by_line() {
        let ini = b"[Campaign_1]\r\nName=The Fire\r\nTexture=41101\r\nText=Burn\n";
        let rows: &[(&[u8], &[u8])] = &[(b"The Fire", b"Das Feuer"), (b"Burn", b"Brand")];
        let out = translate_ini(ini, &lookup(rows)).unwrap();
        assert_eq!(
            out,
            b"[Campaign_1]\r\nName=Das Feuer\r\nTexture=41101\r\nText=Brand\n"
        );
        assert!(translate_ini(b"Texture=41101\r\n", &lookup(rows)).is_none());
    }

    #[test]
    fn list_lines_keep_their_endings() {
        let list = b"Bremen\r\nLuebeck\r\nRiga";
        let rows: &[(&[u8], &[u8])] = &[(b"Luebeck", b"L\xfcbeck"), (b"Riga", b"Riia")];
        let out = translate_lines(list, &lookup(rows)).unwrap();
        assert_eq!(out, b"Bremen\r\nL\xfcbeck\r\nRiia");
    }
}
