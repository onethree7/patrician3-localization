//! Translated game files.
//!
//! The game and its renderer read their data files through archiver.dll. The hooks
//! in `hooks` serve every file listed in [`FILES`] with its texts replaced; every
//! other file, and every file whose texts do not match the English originals, reaches
//! the game unchanged.
//!
//! Pictures with text (the main menu) are not tables but whole files per language,
//! in `mods\localization\<code>\` next to the DLL (`data/gfx/<code>/` in the
//! repository, made by `tools/gfx/gen.py`), with `textures.tsv` giving the sprite
//! rectangles of their sheets in `scripts/textures.ini`.
//!
//! The tables (`data/files/<code>.tsv`) have one model for all files:
//! `scope<TAB>English<TAB>translation`. The scope is a pattern of [`FILES`]
//! (`*.p2m`, `*.gmf`, `scripts/tutorial_eng.ini`, ...) or, for missions, the name of
//! one mission (`kampagne2.p2m`), whose rows win over the `*.p2m` rows. The
//! translation `=` keeps the English text; the English text `*` with the
//! translation `=` keeps every text of the scope. `data/files/source.tsv` lists
//! every English text of every listed file (`tools/textkit/extract.py`); the tests
//! require each language to decide on all of them.

mod hooks;
mod map;
mod mission;
mod text;

use std::collections::HashMap;

pub(crate) use hooks::hook_patches;

/// How a file stores its texts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    /// Mission script; table cells use the codec of `mission::decode_mission_text`.
    Mission,
    /// Map; the 40 town names.
    Map,
    /// `key=value` lines; whole values are replaced.
    Ini,
    /// One entry per line, split at control characters.
    Lines,
    /// One name per line, read by the name generator (0x5128D0), which ends a name at
    /// every byte below `A` except space and `.`; translations must avoid them.
    Names,
    /// Picture replaced as a whole by the language's file of the same name.
    Image,
    /// `scripts/textures.ini`: the `OffsetNSize` rectangles of a replaced sprite sheet.
    Textures,
}

impl Format {
    /// Formats whose texts the tables list.
    fn has_texts(self) -> bool {
        !matches!(self, Format::Image | Format::Textures)
    }
}

/// Every game file with texts. Paths are lowercase, with `/` and without `./`.
pub(crate) const FILES: &[(&str, Format)] = &[
    ("*.p2m", Format::Mission),
    ("*.gmf", Format::Map),
    ("scripts/missions_eng.ini", Format::Ini),
    ("scripts/tutorial_eng.ini", Format::Ini),
    ("scripts/credits_eng.ini", Format::Ini),
    ("scripts/stadtauswahl.ini", Format::Ini),
    ("scripts/namenstaedte_eng.txt", Format::Lines),
    ("scripts/namenschiffe_eng.txt", Format::Names),
    ("scripts/namenmaenner.txt", Format::Names),
    ("scripts/namenfrauen.txt", Format::Names),
    ("scripts/namenfamilien.txt", Format::Names),
    ("images/options/hauptmenu.jpg", Format::Image),
    ("images/options/p_hmenu01.bmp", Format::Image),
    ("images/options/hmenu_colormap.bmp", Format::Image),
    ("scripts/textures.ini", Format::Textures),
];

/// Sprite rectangles of the replaced sheets, `data/gfx/<code>/textures.tsv`.
pub(crate) const TEXTURES_FILE: &str = "textures.tsv";

/// Translation that keeps the English text.
const KEEP: &str = "=";
/// English text of a row that keeps every text of its scope.
const ALL: &str = "*";

type Table = HashMap<Vec<u8>, Vec<u8>>;

/// Translates one text, or `None` to keep it.
pub(super) type Lookup<'t> = dyn Fn(&[u8]) -> Option<&'t [u8]> + 't;

/// English text -> translation per scope, as raw CP1252 bytes.
#[derive(Debug, Default)]
pub(crate) struct FileTexts {
    scopes: HashMap<String, Table>,
    /// [`Format::Image`] path -> the language's picture.
    images: HashMap<String, Vec<u8>>,
    /// Texture file (as in `TextureFile=`, normalized) -> `x y width height` per sprite.
    sprites: HashMap<String, Vec<String>>,
}

impl FileTexts {
    pub(crate) fn is_empty(&self) -> bool {
        self.scopes.values().all(HashMap::is_empty) && self.images.is_empty()
    }

    /// Adds the pictures of a language folder, all or none. A missing folder leaves the
    /// English pictures; a folder with pictures but without their rectangles is an error.
    pub(crate) fn load_pictures(&mut self, folder: &std::path::Path) -> Result<(), String> {
        let Ok(entries) = std::fs::read_dir(folder) else {
            return Ok(());
        };
        let names: HashMap<String, std::path::PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().to_lowercase(),
                    entry.path(),
                )
            })
            .collect();
        let mut images = HashMap::new();
        for (pattern, format) in FILES {
            if *format != Format::Image {
                continue;
            }
            if let Some(path) = names.get(base_name(pattern)) {
                let data =
                    std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
                images.insert(pattern.to_string(), data);
            }
        }
        if images.is_empty() {
            return Ok(());
        }
        let path = names
            .get(TEXTURES_FILE)
            .ok_or_else(|| format!("{}: {TEXTURES_FILE} is missing", folder.display()))?;
        let table = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        self.sprites = load_sprites(&table)?;
        self.images = images;
        Ok(())
    }
}

/// Parses `texture file<TAB>x y width height` rows; the row order is the sprite index.
pub(crate) fn load_sprites(table: &str) -> Result<HashMap<String, Vec<String>>, String> {
    let mut sprites: HashMap<String, Vec<String>> = HashMap::new();
    for line in table.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (file, rect) = line
            .split_once('\t')
            .ok_or_else(|| format!("Malformed sprite row: {line}"))?;
        let numbers: Vec<&str> = rect.split_whitespace().collect();
        if numbers.len() != 4 || numbers.iter().any(|n| n.parse::<u32>().is_err()) {
            return Err(format!("Malformed sprite rectangle: {line}"));
        }
        sprites
            .entry(normalize(file.as_bytes()))
            .or_default()
            .push(numbers.join(" "));
    }
    Ok(sprites)
}

/// Game file name as a lowercase path. Bytes map one to one to characters
/// (Latin-1), which equals CP1252 for the letters game file names use.
fn normalize(name: &[u8]) -> String {
    let name: String = name.iter().map(|byte| *byte as char).collect();
    let name = name.replace('\\', "/").to_lowercase();
    name.trim_start_matches("./").to_owned()
}

fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The [`FILES`] entry of a normalized path.
fn classify(path: &str) -> Option<(&'static str, Format)> {
    FILES
        .iter()
        .copied()
        .find(|(pattern, _)| match pattern.strip_prefix('*') {
            Some(extension) => path.ends_with(extension),
            None => path == *pattern,
        })
}

/// Pattern and format of a table scope.
fn scope_format(scope: &str) -> Option<(&'static str, Format)> {
    FILES
        .iter()
        .copied()
        .find(|(pattern, _)| *pattern == scope)
        .or_else(|| {
            let one_file = !scope.contains(['*', '/']);
            one_file.then(|| classify(scope)).flatten()
        })
}

fn decode_cell(format: Format, value: &str) -> Result<Vec<u8>, String> {
    if format == Format::Mission {
        return mission::decode_mission_text(value);
    }
    let raw = crate::encode_cp1252(value, "file text")?;
    if raw.iter().any(|byte| matches!(byte, 0 | b'\r' | b'\n')) {
        return Err(format!("Control character in file text: {value}"));
    }
    Ok(raw)
}

/// Bytes the name generator does not split names at.
fn is_name_byte(byte: u8) -> bool {
    byte >= b'A' || byte == b' ' || byte == b'.'
}

/// Parses a `scope<TAB>English<TAB>translation` table.
pub(crate) fn load_file_texts(table: &str) -> Result<FileTexts, String> {
    let table = table.strip_prefix('\u{feff}').unwrap_or(table);
    let mut texts = FileTexts::default();
    let mut seen = std::collections::HashSet::new();
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
        if !seen.insert((*scope, *source)) {
            return Err(format!("Duplicate file text row: {line}"));
        }
        let (pattern, format) =
            scope_format(scope).ok_or_else(|| format!("Unknown file text scope '{scope}'"))?;
        if !format.has_texts() {
            return Err(format!("{scope} has no texts: {line}"));
        }
        if *source == ALL {
            if *translation != KEEP {
                return Err(format!("A '{ALL}' row can only keep texts: {line}"));
            }
            continue;
        }
        let source = decode_cell(format, source)?;
        if *translation == KEEP {
            continue;
        }
        let translation = decode_cell(format, translation)?;
        // Rows for one mission are taken from a shipped mission whose script supplies
        // the arguments, so only the generic rows must keep the tokens.
        if pattern == *scope
            && format == Format::Mission
            && mission::mission_tokens(&source) != mission::mission_tokens(&translation)
        {
            return Err(format!("Format tokens differ in mission text: {line}"));
        }
        if format == Format::Names && !translation.iter().copied().all(is_name_byte) {
            return Err(format!("Name with a character the game splits at: {line}"));
        }
        texts
            .scopes
            .entry(scope.to_string())
            .or_default()
            .insert(source, translation);
    }
    Ok(texts)
}

/// Returns the file with its texts translated, or `None` to serve it unchanged.
fn translate_file(name: &[u8], data: &[u8], texts: &FileTexts) -> Option<Vec<u8>> {
    let path = normalize(name);
    let (pattern, format) = classify(&path)?;
    match format {
        Format::Image => return texts.images.get(pattern).cloned(),
        Format::Textures => return text::translate_textures(data, &texts.sprites),
        _ => {}
    }
    let tables: Vec<&Table> = [path.as_str(), base_name(&path), pattern]
        .iter()
        .filter_map(|scope| texts.scopes.get(*scope))
        .collect();
    if tables.is_empty() {
        return None;
    }
    let lookup = |original: &[u8]| {
        tables
            .iter()
            .find_map(|table| table.get(original))
            .map(Vec::as_slice)
    };
    match format {
        Format::Mission => mission::translate(data, &lookup),
        Format::Map => map::translate(data, &lookup),
        Format::Ini => text::translate_ini(data, &lookup),
        Format::Lines | Format::Names => text::translate_lines(data, &lookup),
        Format::Image | Format::Textures => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::LANGUAGES;
    use std::collections::{BTreeMap, HashSet};

    #[test]
    fn file_names_find_their_format() {
        assert_eq!(
            classify(&normalize(b"./Missions_Addon\\Kampagne2.P2M")),
            Some(("*.p2m", Format::Mission))
        );
        assert_eq!(
            classify(&normalize(b"maps/Standard24.gmf")),
            Some(("*.gmf", Format::Map))
        );
        assert_eq!(
            classify(&normalize(b"./scripts/NamenSchiffe_eng.txt")),
            Some(("scripts/namenschiffe_eng.txt", Format::Names))
        );
        assert_eq!(classify("scripts/namenschiffe.txt"), None);
        assert_eq!(
            scope_format("kampagne2.p2m"),
            Some(("*.p2m", Format::Mission))
        );
        assert_eq!(scope_format("missions/kampagne2.p2m"), None);
        assert_eq!(scope_format("p2m"), None);
    }

    #[test]
    fn rows_for_one_mission_win_over_generic_rows() {
        let texts = load_file_texts(
            "*.p2m\tTitle\tAllgemein\nkampagne3_entf\u{fc}hrung.p2m\tTitle\tNur hier\n",
        )
        .unwrap();
        let mission = |name: &[u8]| {
            let data = [1u8, 0, 0, 0, 8, 0, 0, 0, 0xfe, 0x07]
                .iter()
                .copied()
                .chain(*b"Title\0")
                .collect::<Vec<_>>();
            translate_file(name, &data, &texts).unwrap()
        };
        // The game passes file names as CP1252 bytes.
        assert!(mission(b"missions_addon\\Kampagne3_Entf\xfchrung.p2m").ends_with(b"Nur hier\0"));
        assert!(mission(b"missions\\000010_m.p2m").ends_with(b"Allgemein\0"));
    }

    #[test]
    fn keep_rows_and_name_rules() {
        let texts = load_file_texts(
            "scripts/namenmaenner.txt\t*\t=\n*.gmf\tBoston\t=\n*.gmf\tLuebeck\tL\u{fc}beck\n",
        )
        .unwrap();
        assert_eq!(texts.scopes["*.gmf"].len(), 1);
        assert!(load_file_texts("*.gmf\t*\tX\n").is_err());
        assert!(load_file_texts("scripts/namenschiffe_eng.txt\tSea Eagle\tSea-Eagle\n").is_err());
        assert!(load_file_texts("scripts/namenstaedte_eng.txt\tBruges\tBr\u{fc}gge's\n").is_ok());
        assert!(load_file_texts("*.p2m\tA {%t:01}\tB\n").is_err());
        assert!(load_file_texts("x.p2m\tA {%t:01}\tB\n").is_ok());
        assert!(load_file_texts("*.gmf\tA\tB\n*.gmf\tA\tC\n").is_err());
    }

    /// `data/files/source.tsv`: scope -> English texts.
    fn source() -> BTreeMap<&'static str, HashSet<&'static str>> {
        let mut source: BTreeMap<_, HashSet<_>> = BTreeMap::new();
        for line in include_str!("../../data/files/source.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (scope, text) = line.split_once('\t').unwrap();
            assert!(
                classify(scope).is_some() || scope.starts_with('*'),
                "{scope}"
            );
            source.entry(scope).or_default().insert(text);
        }
        source
    }

    #[test]
    fn source_lists_every_file() {
        let source = source();
        for (pattern, format) in FILES {
            if !format.has_texts() {
                continue;
            }
            assert!(
                source.contains_key(pattern),
                "no source texts for {pattern}"
            );
        }
    }

    /// Every language decides on every English text: a translation, `=` or a `*` row.
    #[test]
    fn tables_cover_exactly_the_extracted_english() {
        const FONT_EXTRA: &str = "äöüÄÖÜßáàâéèêíìîóòôúùû";
        let source = source();
        let mut report = String::new();
        for language in LANGUAGES {
            load_file_texts(language.files).unwrap_or_else(|e| panic!("{}: {e}", language.code));
            let mut decided: HashMap<&str, HashSet<&str>> = HashMap::new();
            let mut whole: HashSet<&str> = HashSet::new();
            for line in language.files.lines() {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let mut fields = line.split('\t');
                let (scope, english, translation) = (
                    fields.next().unwrap(),
                    fields.next().unwrap(),
                    fields.next().unwrap(),
                );
                let (pattern, _) = scope_format(scope).unwrap();
                if english == ALL {
                    whole.insert(pattern);
                    continue;
                }
                assert!(
                    source[pattern].contains(english),
                    "{}: {scope}: unknown English text {english}",
                    language.code
                );
                if pattern == scope {
                    decided.entry(pattern).or_default().insert(english);
                }
                if language.code != "de" {
                    if let Some(c) = translation
                        .chars()
                        .find(|c| !(c.is_ascii() || FONT_EXTRA.contains(*c)))
                    {
                        panic!("{}: {c:?} has no glyph in: {translation}", language.code);
                    }
                }
            }
            for (scope, texts) in &source {
                let done = if whole.contains(scope) {
                    texts.len()
                } else {
                    decided.get(scope).map_or(0, |set| set.len())
                };
                report.push_str(&format!(
                    "{} {scope}: {done}/{}\n",
                    language.code,
                    texts.len()
                ));
                if done != texts.len() {
                    let missing: Vec<_> = texts
                        .iter()
                        .filter(|text| !decided.get(scope).is_some_and(|set| set.contains(*text)))
                        .take(3)
                        .collect();
                    panic!(
                        "{}: {scope}: {done} of {} texts decided, for example {missing:?}\n{report}",
                        language.code,
                        texts.len()
                    );
                }
            }
        }
        println!("{report}");
    }

    /// Texts name towns as the map of their language does (`data/glossary/towns.tsv`).
    #[test]
    fn town_names_follow_the_glossary() {
        let glossary = include_str!("../../data/glossary/towns.tsv");
        let mut rows = glossary.lines().filter(|line| !line.starts_with('#'));
        let header: Vec<&str> = rows.next().unwrap().split('\t').collect();
        let towns: Vec<Vec<&str>> = rows.map(|row| row.split('\t').collect()).collect();
        for language in LANGUAGES {
            let column = header
                .iter()
                .position(|code| *code == language.code)
                .unwrap();
            let forbidden: Vec<(&str, &str)> = towns
                .iter()
                .flat_map(|town| {
                    let target = if town[column].is_empty() {
                        town[0]
                    } else {
                        town[column]
                    };
                    std::iter::once(town[0])
                        .chain(town[1].split(',').filter(|alias| !alias.is_empty()))
                        .filter(move |variant| *variant != target)
                        .map(move |variant| (variant, target))
                })
                .collect();
            for line in language.files.lines() {
                let fields: Vec<&str> = line.split('\t').collect();
                let [scope, english, translation] = fields.as_slice() else {
                    continue;
                };
                if *english == ALL || scope_format(scope).is_some_and(|(_, f)| f == Format::Names) {
                    continue;
                }
                let text = if *translation == KEEP {
                    *english
                } else {
                    *translation
                };
                for (variant, target) in &forbidden {
                    for (at, _) in text.match_indices(variant) {
                        // "Hamburg" inside "Hamburgo" is already the target.
                        if text[at..].starts_with(target) {
                            continue;
                        }
                        let before = text[..at].chars().next_back();
                        let after = text[at + variant.len()..]
                            .chars()
                            .find(|c| !c.is_lowercase());
                        let word_start = !before.is_some_and(char::is_alphanumeric);
                        let word_end = !after.is_some_and(char::is_alphanumeric);
                        assert!(
                            !(word_start && word_end),
                            "{}: '{variant}' should read '{target}' in: {text}",
                            language.code
                        );
                    }
                }
            }
        }
    }
}
