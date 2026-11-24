//! Language registry and per-language data.
//!
//! German keeps its legacy tables (`german_localization.tsv`, `render_translations.tsv`).
//! Every other language is a single table in `data/lang/<code>.tsv` with rows of
//! `scope<TAB>English<TAB>translation`:
//!
//! * `0x…`        a text slot of the game image (the English text must match the slot table)
//! * `render`     exact-match text drawn at render time (also by other mods)
//! * `addon`      like `render`, but only for callers outside the game image
//! * `resource:N` string resource N
//! * `frag`       a named fragment/template used by the render-time pattern translators
//!
//! Anything a language does not list stays English.

use super::{decode_table_text, encode_cp1252, load_translations, parse_rva, TextEntry};
use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
};

pub(crate) const DEFAULT_LANGUAGE: &str = "de";
pub(crate) const CONFIG_FILE: &str = "localization.ini";

pub(crate) struct LanguageInfo {
    pub code: &'static str,
    pub name: &'static str,
    /// Embedded table; empty for German (legacy tables).
    pub table: &'static str,
    /// Mission and campaign texts (`data/files/<code>.tsv`), see `files.rs`.
    pub files: &'static str,
}

pub(crate) const LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo {
        code: "de",
        name: "Deutsch",
        table: "",
        files: include_str!("../data/files/de.tsv"),
    },
    LanguageInfo {
        code: "fr",
        name: "Français",
        table: include_str!("../data/lang/fr.tsv"),
        files: include_str!("../data/files/fr.tsv"),
    },
    LanguageInfo {
        code: "nl",
        name: "Nederlands",
        table: include_str!("../data/lang/nl.tsv"),
        files: include_str!("../data/files/nl.tsv"),
    },
    LanguageInfo {
        code: "es",
        name: "Español",
        table: include_str!("../data/lang/es.tsv"),
        files: include_str!("../data/files/es.tsv"),
    },
    LanguageInfo {
        code: "it",
        name: "Italiano",
        table: include_str!("../data/lang/it.tsv"),
        files: include_str!("../data/files/it.tsv"),
    },
    LanguageInfo {
        code: "sv",
        name: "Svenska",
        table: include_str!("../data/lang/sv.tsv"),
        files: include_str!("../data/files/sv.tsv"),
    },
    LanguageInfo {
        code: "da",
        name: "Dansk",
        table: include_str!("../data/lang/da.tsv"),
        files: include_str!("../data/files/da.tsv"),
    },
    LanguageInfo {
        code: "no",
        name: "Norsk",
        table: include_str!("../data/lang/no.tsv"),
        files: include_str!("../data/files/no.tsv"),
    },
    LanguageInfo {
        code: "pt",
        name: "Português",
        table: include_str!("../data/lang/pt.tsv"),
        files: include_str!("../data/files/pt.tsv"),
    },
];

pub(crate) type Row = (&'static str, &'static str, &'static str);

pub(crate) struct Bundle {
    pub entries: Vec<TextEntry>,
    /// Rows for the render map (`render`, `addon`, `resource:N`); slot and `frag` rows are removed.
    pub overrides: Vec<Row>,
    pub fragments: Fragments,
    /// German must resolve every preferred render slot; other languages may be partial.
    pub strict: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Fragments {
    values: HashMap<&'static str, String>,
}

impl Fragments {
    pub(crate) fn from_rows(rows: &[Row]) -> Result<Self, String> {
        let mut values = HashMap::new();
        for (scope, key, value) in rows {
            if *scope != "frag" {
                continue;
            }
            encode_cp1252(value, "fragment")?;
            if key.starts_with("month.") && value.contains(char::is_whitespace) {
                return Err(format!("Month name must not contain spaces: {key}"));
            }
            if values.insert(*key, (*value).to_string()).is_some() {
                return Err(format!("Duplicate fragment {key}"));
            }
        }
        Ok(Self { values })
    }

    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    #[cfg(test)]
    pub(crate) fn keys(&self) -> impl Iterator<Item = &&'static str> {
        self.values.keys()
    }
}

/// Replaces every `{name}` in `template`.
pub(crate) fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// Fills the single `{name}` marker of `template` with raw (CP1252) bytes.
pub(crate) fn fill_bytes(template: &str, name: &str, value: &[u8]) -> Option<Vec<u8>> {
    let (head, tail) = template.split_once(&format!("{{{name}}}"))?;
    let mut out = encode_cp1252(head, "fragment").ok()?;
    out.extend_from_slice(value);
    out.extend(encode_cp1252(tail, "fragment").ok()?);
    Some(out)
}

pub(crate) fn scoped_rows(input: &'static str) -> Result<Vec<Row>, String> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    input
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            match fields.as_slice() {
                [scope, source, translation]
                    if !scope.is_empty() && !source.is_empty() && !translation.is_empty() =>
                {
                    Ok((*scope, *source, *translation))
                }
                _ => Err(format!("Malformed translation row: {line}")),
            }
        })
        .collect()
}

pub(crate) fn find(code: &str) -> Option<&'static LanguageInfo> {
    LANGUAGES.iter().find(|language| language.code == code)
}

pub(crate) fn available_codes() -> String {
    LANGUAGES
        .iter()
        .map(|language| format!("{} ({})", language.code, language.name))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn resolve(code: &str) -> Result<&'static LanguageInfo, String> {
    find(code).ok_or_else(|| {
        format!(
            "Unbekannte Sprache / unknown language '{code}' in {CONFIG_FILE}. Verfügbar / available: {}",
            available_codes()
        )
    })
}

/// Extracts `language=xx` (case-insensitive key, `;`/`#` comments, `de-DE` style tags allowed).
pub(crate) fn parse_language_setting(text: &str) -> Option<String> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .find_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with([';', '#', '[']) {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            if !key.trim().eq_ignore_ascii_case("language") {
                return None;
            }
            let value = value.trim().trim_matches('"').to_ascii_lowercase();
            let code = value.split(['-', '_']).next().unwrap_or_default();
            (!code.is_empty()).then(|| code.to_string())
        })
}

/// Language chosen by `localization.ini` next to the game (in `mods\` or the game folder).
pub(crate) fn configured_language() -> Result<&'static LanguageInfo, String> {
    let setting = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let dir = exe.parent()?.to_path_buf();
            [dir.join("mods").join(CONFIG_FILE), dir.join(CONFIG_FILE)]
                .into_iter()
                .find_map(|path| std::fs::read(path).ok())
        })
        .and_then(|bytes| parse_language_setting(&String::from_utf8_lossy(&bytes)));
    resolve(setting.as_deref().unwrap_or(DEFAULT_LANGUAGE))
}

fn load_extra_slots() -> Result<Vec<TextEntry>, String> {
    let input = include_str!("../data/extra_slots.tsv");
    let mut entries = Vec::new();
    for line in input
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        let [slot, text, english] = fields.as_slice() else {
            return Err(format!("Malformed extra slot: {line}"));
        };
        let source = CString::new(encode_cp1252(&decode_table_text(english), "extra source")?)
            .map_err(|error| error.to_string())?;
        entries.push(TextEntry {
            slot_rva: parse_rva(slot, "extra slot RVA")?,
            text_rva: parse_rva(text, "extra source RVA")?,
            translation: source.clone(),
            source,
        });
    }
    Ok(entries)
}

pub(crate) fn load(code: &str) -> Result<Bundle, String> {
    let info = resolve(code)?;
    load_table(info.code, info.table)
}

pub(crate) fn load_table(code: &'static str, table: &'static str) -> Result<Bundle, String> {
    if code == DEFAULT_LANGUAGE {
        let rows = super::render_overrides()?;
        let fragments = Fragments::from_rows(&rows)?;
        let overrides = rows
            .into_iter()
            .filter(|(scope, _, _)| *scope != "frag")
            .collect();
        return Ok(Bundle {
            entries: load_translations()?,
            overrides,
            fragments,
            strict: true,
        });
    }

    let rows = scoped_rows(table)?;
    let fragments = Fragments::from_rows(&rows)?;
    let mut known: HashMap<usize, TextEntry> = HashMap::new();
    for entry in load_translations()?.into_iter().chain(load_extra_slots()?) {
        let slot = entry.slot_rva;
        if known.insert(slot, entry).is_some() {
            return Err(format!("Slot 0x{slot:X} is listed twice"));
        }
    }

    let mut entries = Vec::new();
    let mut overrides = Vec::new();
    let mut seen = HashSet::new();
    for row @ (scope, source, translation) in rows {
        if let Some(rva) = scope.strip_prefix("0x") {
            let slot =
                usize::from_str_radix(rva, 16).map_err(|_| format!("Invalid slot RVA: {scope}"))?;
            let base = known
                .get(&slot)
                .ok_or_else(|| format!("Slot {scope} is not in the slot tables"))?;
            let english = encode_cp1252(&decode_table_text(source), "English source")?;
            if english != base.source.as_bytes() {
                return Err(format!(
                    "English text of slot {scope} does not match: {source}"
                ));
            }
            if !seen.insert(slot) {
                return Err(format!("Slot {scope} is translated twice"));
            }
            let translated = encode_cp1252(&decode_table_text(translation), "translation")?;
            entries.push(TextEntry {
                slot_rva: slot,
                text_rva: base.text_rva,
                source: base.source.clone(),
                translation: CString::new(translated).map_err(|error| error.to_string())?,
            });
        } else if scope != "frag" {
            overrides.push(row);
        }
    }
    entries.sort_by_key(|entry| entry.slot_rva);
    Ok(Bundle {
        entries,
        overrides,
        fragments,
        strict: false,
    })
}
