//! Plain text files: ini files (`key=value`), name lists (one entry per line) and the
//! sprite rectangles of `textures.ini`.

use super::Lookup;
use std::collections::HashMap;

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

/// `textures.ini` with the sprite rectangles of every section whose `TextureFile` is a
/// key of `sprites`: `Count` and the active `OffsetNSize<n>` lines are replaced,
/// comments and every other line stay. `None` if no section matches.
pub(super) fn translate_textures(
    data: &[u8],
    sprites: &HashMap<String, Vec<String>>,
) -> Option<Vec<u8>> {
    let text: String = data.iter().map(|byte| *byte as char).collect();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(newline).collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut changed = false;
    let mut start = 0;
    while start < lines.len() {
        let end = (start + 1..lines.len())
            .find(|&index| lines[index].trim_start().starts_with('['))
            .unwrap_or(lines.len());
        let section = &lines[start..end];
        let rects = section.iter().find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim().eq_ignore_ascii_case("TextureFile"))
                .then(|| sprites.get(&super::normalize(value.trim().as_bytes())))
                .flatten()
        });
        let Some(rects) = rects else {
            out.extend(section.iter().map(|line| line.to_string()));
            start = end;
            continue;
        };
        changed = true;
        let key = |line: &str| {
            line.split_once('=')
                .map(|(key, _)| key.trim().to_ascii_lowercase())
                .unwrap_or_default()
        };
        for line in section {
            let name = key(line);
            if name.starts_with("offsetnsize") {
                continue;
            }
            if name == "count" {
                out.push(format!("Count={}", rects.len()));
                out.extend(
                    rects
                        .iter()
                        .enumerate()
                        .map(|(index, rect)| format!("OffsetNSize{index}={rect}")),
                );
                continue;
            }
            out.push(line.to_string());
        }
        start = end;
    }
    changed.then(|| out.join(newline).chars().map(|c| c as u8).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_rectangles_replace_only_their_section() {
        let ini = b"[TEX13]\r\nTextureFile=images/x.bmp\r\nCount=1\r\nOffsetNSize0=0 0 1 1\r\n\r\n\
[TEX14]\r\nTextureFile=images/options/p_hmenu01.bmp\r\nUseAlpha=false\r\nCount=8\r\n\
; Deutsch\r\n;OffsetNSize0=0 0 155 54\r\n OffsetNSize0=0 0 157 55\r\n OffsetNSize1=0 55 191 68\r\n\r\n[TEX15]\r\n";
        let sprites = super::super::load_sprites(
            "images/options/p_hmenu01.bmp\t0 0 10 20\nimages/options/p_hmenu01.bmp\t0 20 30 40\n",
        )
        .unwrap();
        let out = String::from_utf8(translate_textures(ini, &sprites).unwrap()).unwrap();
        assert_eq!(
            out,
            "[TEX13]\r\nTextureFile=images/x.bmp\r\nCount=1\r\nOffsetNSize0=0 0 1 1\r\n\r\n\
[TEX14]\r\nTextureFile=images/options/p_hmenu01.bmp\r\nUseAlpha=false\r\nCount=2\r\n\
OffsetNSize0=0 0 10 20\r\nOffsetNSize1=0 20 30 40\r\n; Deutsch\r\n;OffsetNSize0=0 0 155 54\r\n\r\n[TEX15]\r\n"
        );
        assert!(translate_textures(b"[TEX1]\nTextureFile=a.bmp\n", &sprites).is_none());
        assert!(super::super::load_sprites("a.bmp\t1 2 3\n").is_err());
    }

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
