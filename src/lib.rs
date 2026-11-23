mod language;
mod patch;

use language::{fill, fill_bytes, Fragments, Row};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::{c_void, CStr, CString},
    panic,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::System::{
    Diagnostics::Debug::{OutputDebugStringA, ReadProcessMemory},
    LibraryLoader::GetModuleHandleW,
    Threading::GetCurrentProcess,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

struct TextEntry {
    slot_rva: usize,
    text_rva: usize,
    source: CString,
    translation: CString,
}

#[derive(Debug, Default)]
struct RenderTranslations {
    exact: HashMap<Vec<u8>, CString>,
    insensitive: HashMap<Vec<u8>, CString>,
    resources: HashMap<u32, (CString, CString)>,
    /// Exact texts that other mods draw from their own DLLs. These win over
    /// `exact` only for callers outside the game image, so a mod's ware label
    /// can differ from the game's identically spelled building name.
    addon: HashMap<Vec<u8>, CString>,
    /// Templates and month names for the render-time pattern translators.
    fragments: Fragments,
}

impl RenderTranslations {
    fn get(&self, text: &[u8]) -> Option<&CString> {
        self.exact.get(text).or_else(|| {
            let mut key = text.to_vec();
            key.make_ascii_lowercase();
            self.insensitive.get(&key)
        })
    }
}

static TRANSLATIONS: OnceLock<Vec<TextEntry>> = OnceLock::new();
static RENDER_TRANSLATIONS: OnceLock<RenderTranslations> = OnceLock::new();
static ORIGINAL_LOAD_STRING: OnceLock<usize> = OnceLock::new();
static GAME_BASE: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_MEASURE_TEXT: OnceLock<usize> = OnceLock::new();
static INSTALL_RESULT: OnceLock<Result<usize, String>> = OnceLock::new();
const LOAD_STRING_IAT_RVA: usize = 0x26A494;
const UI_RENDER_TEXT_AT_RVA: usize = 0xBB3E0;
const UI_RENDER_TEXT_IAT_RVA: usize = 0x2DAA08;
const GAME_IMAGE_SIZE: usize = 0x309000;
const TOOLTIP_MEASURE_CALL_RVA: usize = 0xB8239;
const TOOLTIP_MEASURE_FUNCTION_RVA: usize = 0xC1B20;
// Fragments with grammar or identifier meaning must never become global dictionary keys.
const DIRECT_ONLY_SLOT_RVAS: &[usize] = &[
    0x299928, 0x29C9F8, 0x29D8F0, 0x29D8F4, 0x29D9F4, 0x29D9F8, 0x29DA48, 0x2AB64C, 0x2AB654,
    0x2B0384, 0x2B74EC, 0x2B7600, 0x2C0DB0, 0x2C0ED8, 0x2C0EDC, 0x2C0EE0, 0x2C0EE4, 0x2C0EE8,
    0x2C0EEC, 0x2C0EF0, 0x2C0EF4, 0x2C0EF8, 0x2C0EFC, 0x2C0F00, 0x2C0F04, 0x2C0F08, 0x2C0F0C,
    0x2C0F10,
];
const DIRECT_ONLY_SLOT_RANGES: &[std::ops::RangeInclusive<usize>] = &[
    0x29AF44..=0x29AFA0,
    0x2A051C..=0x2A0538,
    0x2A0BD0..=0x2A0CA8, // Family labels and inflected adjective tables.
    0x2A177C..=0x2A17A0,
    0x2AF3C0..=0x2AF3D8, // Guard-post settings use context-specific casing.
];
const BUILDING_NAME_RVA_RANGE: std::ops::RangeInclusive<usize> = 0x2A54E8..=0x2A5798;
const RENDER_TEXT_PREFERRED_SLOT_RVAS: &[usize] = &[0x2A4390, 0x2AEF1C];
fn is_report_date(value: &str) -> bool {
    let mut parts = value.split_whitespace();
    let Some(day) = parts.next().and_then(|part| part.strip_suffix('.')) else {
        return false;
    };
    let Some(month) = parts.next() else {
        return false;
    };
    let Some(year) = parts.next() else {
        return false;
    };
    day.bytes().all(|byte| byte.is_ascii_digit())
        && matches!(
            month,
            "Jan"
                | "Feb"
                | "Mar"
                | "Apr"
                | "May"
                | "Jun"
                | "Jul"
                | "Aug"
                | "Sep"
                | "Oct"
                | "Nov"
                | "Dec"
        )
        && year.len() == 4
        && year.bytes().all(|byte| byte.is_ascii_digit())
        && parts.next().is_none()
}

fn translate_report_date(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let text = std::str::from_utf8(value).ok()?;
    let translated = if let Some(date) = text.strip_prefix("on ") {
        let date = date.trim_end();
        if !is_report_date(date) {
            return None;
        }
        let padding = &text[3 + date.len()..];
        let localized = localize_report_months(frags, date)?;
        format!(
            "{}{padding}",
            fill(frags.get("report_on")?, &[("date", &localized)])
        )
    } else {
        let range = text.strip_prefix("from ")?;
        let (start, end) = range.split_once(" to ")?;
        let end_date = end.trim_end();
        if !is_report_date(start) || !is_report_date(end_date) {
            return None;
        }
        let padding = &end[end_date.len()..];
        let start = localize_report_months(frags, start)?;
        let end = localize_report_months(frags, end_date)?;
        format!(
            "{}{padding}",
            fill(
                frags.get("report_range")?,
                &[("start", &start), ("end", &end)]
            )
        )
    };
    cstring_cp1252(translated, "translated report date")
}

fn cstring_cp1252(value: String, field: &str) -> Option<CString> {
    CString::new(encode_cp1252(&value, field).ok()?).ok()
}

fn localize_report_months(frags: &Fragments, date: &str) -> Option<String> {
    let mut parts = date.split_whitespace();
    let day = parts.next()?;
    let month = parts.next()?;
    let year = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let month = frags.get(&format!("month.{month}"))?;
    Some(format!("{day} {month} {year}"))
}

fn translate_bare_report_date(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let text = std::str::from_utf8(value).ok()?;
    let (day, rest) = text.split_once('.')?;
    let month_start = rest.bytes().position(|byte| !byte.is_ascii_whitespace())?;
    let month_end = month_start
        + rest[month_start..]
            .find(char::is_whitespace)
            .unwrap_or(rest.len() - month_start);
    let spacing_after_day = &rest[..month_start];
    let month = &rest[month_start..month_end];
    let year_and_padding = &rest[month_end..];
    let year = year_and_padding.trim();
    if day.is_empty()
        || !day.bytes().all(|byte| byte.is_ascii_digit())
        || !is_report_date(&format!("{day}. {month} {year}"))
    {
        return None;
    }
    let localized = localize_report_months(frags, &format!("{day}. {month} {year}"))?;
    let localized_month = localized.split_whitespace().nth(1)?;
    cstring_cp1252(
        format!("{day}.{spacing_after_day}{localized_month}{year_and_padding}"),
        "localized game date",
    )
}

fn translate_chronicle_header(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let (head, tail) = frags.get("chronicle")?.split_once("{date}")?;
    let head = encode_cp1252(head, "fragment").ok()?;
    let tail = encode_cp1252(tail, "fragment").ok()?;
    let date = value
        .strip_prefix(head.as_slice())?
        .strip_suffix(tail.as_slice())?;
    let localized_date = translate_bare_report_date(frags, date)?;
    let mut translated = head;
    translated.extend_from_slice(localized_date.as_bytes());
    translated.extend_from_slice(&tail);
    CString::new(translated).ok()
}

fn translate_goods_supply_town(frags: &Fragments, value: &[u8]) -> Option<CString> {
    const PREFIX: &[u8] = b"Goods supply in ";
    let text_end = value.iter().rposition(|byte| !byte.is_ascii_whitespace())? + 1;
    let (text, padding) = value.split_at(text_end);
    let town = text.strip_prefix(PREFIX)?;
    if town.is_empty()
        || !town.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || (0xc0..=0xff).contains(byte) || b" -'".contains(byte)
        })
    {
        return None;
    }
    let mut translated = fill_bytes(frags.get("goods_supply")?, "town", town)?;
    translated.extend_from_slice(padding);
    CString::new(translated).ok()
}

fn translate_town_information(frags: &Fragments, value: &[u8]) -> Option<CString> {
    if let Some(population) = value.strip_suffix(b" inhabitants") {
        if !population.is_empty()
            && population
                .split(|byte| *byte == b'.' || *byte == b',')
                .all(|group| !group.is_empty() && group.iter().all(u8::is_ascii_digit))
        {
            let template = frags.get("town_inhabitants")?;
            return CString::new(fill_bytes(template, "n", population)?).ok();
        }
        return None;
    }
    let translated = if let Some(name) = value.strip_prefix(b"Information about ") {
        if name.is_empty()
            || !name.iter().all(|byte| {
                byte.is_ascii_alphabetic() || (0xc0..=0xff).contains(byte) || b" -'".contains(byte)
            })
        {
            return None;
        }
        fill_bytes(frags.get("town_information")?, "name", name)?
    } else {
        let population = value.strip_prefix(b"Population: ")?;
        if population.is_empty() || !population.iter().all(u8::is_ascii_digit) {
            return None;
        }
        fill_bytes(frags.get("town_population")?, "n", population)?
    };
    CString::new(translated).ok()
}

fn translate_lord_mayor(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let (key, name) = if let Some(name) = value.strip_prefix(b"Lord Mayor: the ") {
        ("lord_mayor", name)
    } else {
        let name = value.strip_prefix(b"Lady Mayoress: the ")?;
        ("lady_mayoress", name)
    };
    if name.is_empty()
        || !name.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || (0xc0..=0xff).contains(byte) || b" -'".contains(byte)
        })
    {
        return None;
    }
    CString::new(fill_bytes(frags.get(key)?, "name", name)?).ok()
}

fn translate_tavern_sailors(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let text = std::str::from_utf8(value).ok()?;
    let count = text
        .strip_prefix("There are ")?
        .strip_suffix(" sailors in the tavern.")?;
    if count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    cstring_cp1252(
        fill(frags.get("tavern_sailors")?, &[("n", count)]),
        "tavern sailor count",
    )
}

fn translate_pirate_capture(frags: &Fragments, value: &[u8]) -> Option<CString> {
    const PREFIXES: &[&[u8]] = &[
        b"The pirates take ",
        b"Die Piraten nehmen sich ",
        b"Die Piraten kapern ",
    ];
    let mut ship = PREFIXES.iter().find_map(|prefix| {
        value
            .get(..prefix.len())
            .filter(|head| head.eq_ignore_ascii_case(prefix))
            .map(|_| &value[prefix.len()..])
    })?;
    if let Some(name) = ship.strip_prefix(b"the ") {
        ship = name;
    }
    if ship.is_empty() || ship.iter().any(|byte| *byte < b' ' || *byte == 0x7f) {
        return None;
    }
    CString::new(fill_bytes(frags.get("pirate_capture")?, "ship", ship)?).ok()
}

fn translate_pirate_captain_intro(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let text = std::str::from_utf8(value).ok()?;
    let count = text
        .strip_prefix("A fierce looking pirate captain with ")?
        .strip_suffix(" henchmen offers")?;
    if count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    cstring_cp1252(
        fill(frags.get("pirate_captain")?, &[("n", count)]),
        "pirate captain announcement",
    )
}

fn translate_manager_stage(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let text = std::str::from_utf8(value).ok()?;
    let stage = text.strip_prefix("Manager (Stage ")?.strip_suffix(')')?;
    if stage.is_empty() || !stage.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    cstring_cp1252(
        fill(frags.get("manager_stage")?, &[("n", stage)]),
        "manager stage",
    )
}

fn translate_city_signature(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let prefix = encode_cp1252(frags.get("city_prefix")?, "fragment").ok()?;
    let prefix = prefix.as_slice();
    const TOWNS: &[&[u8]] = &[
        b"Edinburgh",
        b"Newcastle",
        b"Scarborough",
        b"Boston",
        b"London",
        b"Br\xfcgge",
        b"Haarlem",
        b"Harlingen",
        b"Groningen",
        b"K\xf6ln",
        b"Bremen",
        b"Ripen",
        b"Hamburg",
        b"Flensburg",
        b"L\xfcbeck",
        b"Rostock",
        b"Bergen",
        b"Stavanger",
        b"T\xf6nsberg",
        b"Oslo",
        b"Aalborg",
        b"G\xf6teborg",
        b"Naestved",
        b"Malm\xf6",
        b"Ahus",
        b"Stockholm",
        b"Visby",
        b"Helsinki",
        b"Stettin",
        b"R\xfcgenwald",
        b"Danzig",
        b"Thorn",
        b"K\xf6nigsberg",
        b"Memel",
        b"Windau",
        b"Riga",
        b"Pernau",
        b"Reval",
        b"Ladoga",
        b"Novgorod",
    ];
    let town = value.strip_prefix(prefix)?;
    if !TOWNS.iter().any(|name| name.eq_ignore_ascii_case(town)) {
        return None;
    }
    let mut translated = Vec::with_capacity(prefix.len() + 1 + town.len());
    translated.extend_from_slice(prefix);
    translated.push(b' ');
    translated.extend_from_slice(town);
    CString::new(translated).ok()
}

fn translate_day_count(frags: &Fragments, value: &[u8]) -> Option<CString> {
    let count = value
        .strip_suffix(b" day")
        .or_else(|| value.strip_suffix(b" days"))?;
    if count.is_empty() || !count.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let template = frags.get(if count == b"1" { "day_one" } else { "day_many" })?;
    CString::new(fill_bytes(template, "n", count)?).ok()
}

fn find_padded_render_translation(
    text: &[u8],
    translations: &RenderTranslations,
) -> Option<CString> {
    let start = text.iter().position(|byte| !byte.is_ascii_whitespace())?;
    let end = text.iter().rposition(|byte| !byte.is_ascii_whitespace())? + 1;
    let translation = translations.get(&text[start..end])?.as_bytes();
    let translated_start = translation
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(translation.len());
    let translated_end = translation
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(translated_start, |index| index + 1);
    let mut padded =
        Vec::with_capacity(start + translated_end - translated_start + text.len() - end);
    padded.extend_from_slice(&text[..start]);
    padded.extend_from_slice(&translation[translated_start..translated_end]);
    padded.extend_from_slice(&text[end..]);
    CString::new(padded).ok()
}

fn translate_render_text(value: &[u8]) -> Option<Cow<'static, CStr>> {
    let translations = RENDER_TRANSLATIONS.get()?;
    let frags = &translations.fragments;
    if let Some(date) = translate_bare_report_date(frags, value) {
        return Some(Cow::Owned(date));
    }
    if let Some(header) = translate_chronicle_header(frags, value) {
        return Some(Cow::Owned(header));
    }
    if let Some(label) = translate_goods_supply_town(frags, value) {
        return Some(Cow::Owned(label));
    }
    if let Some(signature) = translate_city_signature(frags, value) {
        return Some(Cow::Owned(signature));
    }
    if let Some(translation) = translations.get(value) {
        return Some(Cow::Borrowed(translation.as_c_str()));
    }
    if let Some(translation) = find_padded_render_translation(value, translations) {
        return Some(Cow::Owned(translation));
    }
    translate_report_date(frags, value)
        .or_else(|| translate_town_information(frags, value))
        .or_else(|| translate_lord_mayor(frags, value))
        .or_else(|| translate_tavern_sailors(frags, value))
        .or_else(|| translate_pirate_captain_intro(frags, value))
        .or_else(|| translate_manager_stage(frags, value))
        .or_else(|| translate_day_count(frags, value))
        .or_else(|| translate_pirate_capture(frags, value))
        .map(Cow::Owned)
}

/// Defines a cdecl stub for a patched `jmp` target. On entry `[esp]` is the
/// return address into whoever called the original function; the stub passes
/// it as an extra first argument: `$target(caller, x, y, text)`.
macro_rules! caller_forwarding_stub {
    ($name:ident => $target:path) => {
        #[unsafe(naked)]
        unsafe extern "C" fn $name() {
            std::arch::naked_asm!(
                // Each push moves esp by 4, so [esp + 12] walks text, y, x, return address.
                "push dword ptr [esp + 12]",
                "push dword ptr [esp + 12]",
                "push dword ptr [esp + 12]",
                "push dword ptr [esp + 12]",
                "call {target}",
                "add esp, 16",
                "ret",
                target = sym $target,
            );
        }
    };
}

caller_forwarding_stub!(render_text_hook => render_text);

fn is_addon_caller(caller: usize, base: usize) -> bool {
    base != 0 && !(base..base + GAME_IMAGE_SIZE).contains(&caller)
}

fn translate_addon_render_text(value: &[u8]) -> Option<Cow<'static, CStr>> {
    RENDER_TRANSLATIONS
        .get()
        .and_then(|translations| translations.addon.get(value))
        .map(|translation| Cow::Borrowed(translation.as_c_str()))
        .or_else(|| translate_render_text(value))
}

unsafe extern "C" fn render_text(caller: usize, x: i32, y: i32, text: *const c_void) {
    let base = GAME_BASE.load(Ordering::Acquire);
    let translate = if is_addon_caller(caller, base) {
        translate_addon_render_text
    } else {
        translate_render_text
    };
    let original_text = if text.is_null() {
        None
    } else {
        Some(CStr::from_ptr(text.cast()).to_bytes())
    };
    let replacement = original_text.and_then(translate);
    let text = replacement
        .as_ref()
        .map_or(text, |translation| translation.as_ptr().cast());
    let dispatch = if base == 0 {
        0
    } else {
        std::ptr::read_volatile((base + UI_RENDER_TEXT_IAT_RVA) as *const u32) as usize
    };
    // The game fills this import during WinMain, after the modloader invokes start().
    // Reading it at draw time also preserves other mods that wrap the renderer.
    if dispatch == 0 || dispatch == render_text_hook as *const () as usize {
        report_install_error("Text renderer import is invalid at draw time");
        return;
    }
    let draw_text: unsafe extern "C" fn(i32, i32, *const c_void) = std::mem::transmute(dispatch);
    draw_text(x, y, text);
}

unsafe extern "thiscall" fn measure_tooltip_text_hook(
    font: *mut c_void,
    text: *const c_void,
) -> i32 {
    let translation = if text.is_null() {
        None
    } else {
        translate_render_text(CStr::from_ptr(text.cast()).to_bytes())
    };
    let text = translation
        .as_ref()
        .map_or(text, |value| value.as_ptr().cast());
    let original: unsafe extern "thiscall" fn(*mut c_void, *const c_void) -> i32 =
        std::mem::transmute(
            *ORIGINAL_MEASURE_TEXT
                .get()
                .expect("Tooltip measurement not initialized"),
        );
    original(font, text)
}

/// Render map of the German tables (the reference language).
#[cfg(test)]
fn load_render_translations(entries: &[TextEntry]) -> Result<RenderTranslations, String> {
    let rows = render_overrides()?;
    let fragments = Fragments::from_rows(&rows)?;
    let overrides = rows
        .into_iter()
        .filter(|(scope, _, _)| *scope != "frag")
        .collect::<Vec<_>>();
    build_render_translations(entries, &overrides, fragments, true)
}

/// `strict` (German) requires the preferred render slots to exist; other languages
/// translate a subset of the slots and may lack them.
fn build_render_translations(
    entries: &[TextEntry],
    overrides: &[Row],
    fragments: Fragments,
    strict: bool,
) -> Result<RenderTranslations, String> {
    let mut exact: HashMap<Vec<u8>, CString> = HashMap::new();
    let mut ambiguous = HashSet::new();
    for entry in entries {
        if DIRECT_ONLY_SLOT_RVAS.contains(&entry.slot_rva)
            || DIRECT_ONLY_SLOT_RANGES
                .iter()
                .any(|range| range.contains(&entry.slot_rva))
        {
            continue;
        }
        let source = entry.source.as_bytes().to_vec();
        if ambiguous.contains(&source) {
            continue;
        }
        match exact.get(&source) {
            Some(translation) if translation.as_bytes() != entry.translation.as_bytes() => {
                exact.remove(&source);
                ambiguous.insert(source);
            }
            Some(_) => {}
            None => {
                exact.insert(source, entry.translation.clone());
            }
        }
    }
    for entry in entries
        .iter()
        .filter(|entry| BUILDING_NAME_RVA_RANGE.contains(&entry.text_rva))
    {
        let source = entry.source.as_bytes().to_vec();
        ambiguous.remove(&source);
        exact.insert(source, entry.translation.clone());
    }
    for slot_rva in RENDER_TEXT_PREFERRED_SLOT_RVAS {
        let Some(entry) = entries.iter().find(|entry| entry.slot_rva == *slot_rva) else {
            if strict {
                return Err(format!(
                    "Preferred render slot 0x{slot_rva:X} is absent from the text table"
                ));
            }
            continue;
        };
        let source = entry.source.as_bytes().to_vec();
        if !ambiguous.remove(&source) {
            if strict {
                return Err(format!(
                    "Preferred render slot 0x{slot_rva:X} no longer resolves an ambiguous source"
                ));
            }
            continue;
        }
        exact.insert(source, entry.translation.clone());
    }

    let mut translations = RenderTranslations {
        exact,
        fragments,
        ..RenderTranslations::default()
    };
    for &(scope, source, translation) in overrides {
        let source_bytes = encode_cp1252(source, "render source")?;
        let translated = CString::new(encode_cp1252(translation, "render translation")?)
            .map_err(|error| error.to_string())?;
        if let Some(id) = scope.strip_prefix("resource:") {
            let id = id.parse::<u32>().map_err(|error| error.to_string())?;
            if translations
                .resources
                .insert(
                    id,
                    (
                        CString::new(source_bytes).map_err(|error| error.to_string())?,
                        translated,
                    ),
                )
                .is_some()
            {
                return Err(format!("Duplicate resource translation {id}"));
            }
        } else if scope == "render" {
            if ambiguous.contains(&source_bytes) {
                return Err(format!(
                    "Ambiguous render override needs a resource or pointer context: {source}"
                ));
            }
            translations.exact.insert(source_bytes, translated);
        } else if scope == "addon" {
            if translations
                .addon
                .insert(source_bytes, translated)
                .is_some()
            {
                return Err(format!("Duplicate addon translation: {source}"));
            }
        } else {
            return Err(format!("Unknown translation scope: {scope}"));
        }
    }
    // A context-dependent source must not reappear through a casing fallback.
    let mut ambiguous_insensitive = ambiguous
        .into_iter()
        .map(|mut source| {
            source.make_ascii_lowercase();
            source
        })
        .collect::<HashSet<_>>();
    for (source, translation) in &translations.exact {
        let mut key = source.clone();
        key.make_ascii_lowercase();
        if ambiguous_insensitive.contains(&key) {
            continue;
        }
        match translations.insensitive.get(&key) {
            Some(existing) if existing.as_bytes() != translation.as_bytes() => {
                translations.insensitive.remove(&key);
                ambiguous_insensitive.insert(key);
            }
            Some(_) => {}
            None => {
                translations.insensitive.insert(key, translation.clone());
            }
        }
    }

    if strict && translations.exact.is_empty() {
        return Err("Render translation map is empty".into());
    }
    Ok(translations)
}

fn render_overrides() -> Result<Vec<(&'static str, &'static str, &'static str)>, String> {
    include_str!("../data/render_translations.tsv")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            match fields.as_slice() {
                [scope, source, translation] if !source.is_empty() && !translation.is_empty() => {
                    Ok((*scope, *source, *translation))
                }
                _ => Err(format!("Malformed render translation: {line}")),
            }
        })
        .collect()
}

unsafe extern "system" fn load_string_hook(
    instance: isize,
    id: u32,
    buffer: *mut u8,
    capacity: i32,
) -> i32 {
    let original: unsafe extern "system" fn(isize, u32, *mut u8, i32) -> i32 = std::mem::transmute(
        *ORIGINAL_LOAD_STRING
            .get()
            .expect("Resource loader not initialized"),
    );
    let count = original(instance, id, buffer, capacity);
    if instance as usize != GAME_BASE.load(Ordering::Acquire)
        || count <= 0
        || buffer.is_null()
        || capacity <= 0
    {
        return count;
    }
    let Some((source, translation)) = RENDER_TRANSLATIONS
        .get()
        .and_then(|table| table.resources.get(&id))
    else {
        return count;
    };
    // Resource IDs disambiguate labels that also occur as unrelated gameplay text.
    let expected_length = source.as_bytes().len().min(capacity as usize - 1);
    if count as usize != expected_length
        || std::slice::from_raw_parts(buffer, count as usize)
            != &source.as_bytes()[..expected_length]
    {
        return count;
    }
    let length = translation.as_bytes().len().min(capacity as usize - 1);
    std::ptr::copy_nonoverlapping(translation.as_ptr().cast::<u8>(), buffer, length);
    *buffer.add(length) = 0;
    length as i32
}

fn relative_jump(address: usize, target: usize, opcode: u8) -> Vec<u8> {
    let mut bytes = vec![opcode];
    bytes.extend_from_slice(
        &(target as u32)
            .wrapping_sub(address as u32)
            .wrapping_sub(5)
            .to_le_bytes(),
    );
    bytes
}

unsafe fn hook_patches(base: usize) -> Result<Vec<patch::Patch>, String> {
    let renderer = base + UI_RENDER_TEXT_AT_RVA;
    let renderer_before = read_process_bytes(renderer, 6)?;
    let mut expected = vec![0xff, 0x25];
    expected.extend_from_slice(&((base + UI_RENDER_TEXT_IAT_RVA) as u32).to_le_bytes());
    if renderer_before != expected {
        return Err("Unexpected text renderer import thunk".into());
    }
    let mut renderer_after = relative_jump(renderer, render_text_hook as *const () as usize, 0xe9);
    renderer_after.push(0x90);
    let measure = base + TOOLTIP_MEASURE_CALL_RVA;
    let measure_before = read_process_bytes(measure, 5)?;
    if measure_before != relative_jump(measure, base + TOOLTIP_MEASURE_FUNCTION_RVA, 0xe8) {
        return Err("Unexpected tooltip measurement call".into());
    }
    let resource = base + LOAD_STRING_IAT_RVA;
    let resource_before = read_process_bytes(resource, 4)?;
    let original = u32::from_le_bytes(resource_before.as_slice().try_into().unwrap()) as usize;
    if original == 0 {
        return Err("Resource loader import is not initialized".into());
    }
    ORIGINAL_LOAD_STRING
        .set(original)
        .map_err(|_| "Resource loader already initialized")?;
    ORIGINAL_MEASURE_TEXT
        .set(base + TOOLTIP_MEASURE_FUNCTION_RVA)
        .map_err(|_| "Tooltip measurement already initialized")?;
    Ok(vec![
        patch::Patch {
            address: renderer,
            before: renderer_before,
            after: renderer_after,
        },
        patch::Patch {
            address: measure,
            before: measure_before,
            after: relative_jump(
                measure,
                measure_tooltip_text_hook as *const () as usize,
                0xe8,
            ),
        },
        patch::Patch {
            address: resource,
            before: resource_before,
            after: (load_string_hook as *const () as usize as u32)
                .to_le_bytes()
                .to_vec(),
        },
    ])
}

fn report_install_error(message: &str) {
    let message = message.replace('\0', "");
    if let Ok(message) = CString::new(format!("patrician3_localization: {message}")) {
        unsafe { OutputDebugStringA(PCSTR::from_raw(message.as_ptr().cast())) };
    }
}

fn parse_rva(value: &str, field: &str) -> Result<usize, String> {
    usize::from_str_radix(
        value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
            .unwrap_or(value),
        16,
    )
    .map_err(|_| format!("Invalid {field}: {value}"))
}

fn decode_table_text(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next() {
                Some('\\') => decoded.push('\\'),
                Some('n') => decoded.push('\n'),
                Some('x')
                    if characters
                        .as_str()
                        .get(..2)
                        .is_some_and(|value| value.eq_ignore_ascii_case("0D")) =>
                {
                    characters.next();
                    characters.next();
                    decoded.push('\r');
                }
                Some(other) => {
                    decoded.push('\\');
                    decoded.push(other);
                }
                None => decoded.push('\\'),
            }
        } else {
            decoded.push(character);
        }
    }
    decoded
}

// The five undefined Windows-1252 bytes have no Unicode character.
const CP1252_EXTENDED: [char; 32] = [
    '€', '\0', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\0', 'Ž', '\0', '\0', '‘',
    '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\0', 'ž', 'Ÿ',
];

fn encode_cp1252(value: &str, field: &str) -> Result<Vec<u8>, String> {
    value
        .chars()
        .map(|character| {
            let codepoint = character as u32;
            if codepoint <= 0x7f || (0xa0..=0xff).contains(&codepoint) {
                Ok(codepoint as u8)
            } else if let Some(index) = CP1252_EXTENDED
                .iter()
                .position(|candidate| *candidate == character)
            {
                Ok(0x80 + index as u8)
            } else {
                Err(format!(
                    "Unsupported CP1252 character U+{codepoint:04X} in {field}"
                ))
            }
        })
        .collect()
}

fn load_translations() -> Result<Vec<TextEntry>, String> {
    let input = include_str!("../data/german_localization.tsv");
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut entries = Vec::new();
    let mut slots = HashSet::new();
    let mut previous_slot = None;

    for line in input
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
    {
        let mut fields = line.split('\t');
        let slot_rva = parse_rva(fields.next().ok_or("Missing slot RVA")?, "slot RVA")?;
        let text_rva = parse_rva(fields.next().ok_or("Missing source RVA")?, "source RVA")?;
        let source = decode_table_text(fields.next().ok_or("Missing English source text")?);
        let translation = decode_table_text(fields.next().ok_or("Missing German translation")?);
        if fields.next().is_some() || source.is_empty() || translation.is_empty() {
            return Err(format!(
                "Malformed or empty translation at slot 0x{slot_rva:X}"
            ));
        }
        if !slots.insert(slot_rva) || previous_slot.is_some_and(|previous| slot_rva <= previous) {
            return Err(format!("Duplicate or unsorted slot 0x{slot_rva:X}"));
        }
        previous_slot = Some(slot_rva);

        let source_bytes = encode_cp1252(&source, "English source")?;
        let translated_bytes = encode_cp1252(&translation, "German translation")?;
        entries.push(TextEntry {
            slot_rva,
            text_rva,
            source: CString::new(source_bytes).map_err(|e| e.to_string())?,
            translation: CString::new(translated_bytes).map_err(|e| e.to_string())?,
        });
    }

    if entries.is_empty() || entries.len() > 5000 {
        return Err(format!("Unexpected translation count: {}", entries.len()));
    }
    Ok(entries)
}

unsafe fn read_process_bytes(address: usize, length: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0; length];
    let mut read = 0usize;
    if !ReadProcessMemory(
        GetCurrentProcess(),
        address as *const c_void,
        bytes.as_mut_ptr().cast(),
        length,
        Some(&mut read),
    )
    .as_bool()
        || read != length
    {
        return Err(format!("Cannot read process memory at 0x{address:X}"));
    }
    Ok(bytes)
}

unsafe fn install() -> Result<usize, String> {
    if std::mem::size_of::<usize>() != 4 {
        return Err("Only x86 is supported".into());
    }

    let base = GetModuleHandleW(None).map_err(|e| e.to_string())?.0 as usize;
    let read16 = |offset| ((base + offset) as *const u16).read_unaligned();
    let read32 = |offset| ((base + offset) as *const u32).read_unaligned();
    if read16(0) != 0x5a4d {
        return Err("Missing MZ header".into());
    }
    let pe = read32(0x3c) as usize;
    if pe > 0x1000
        || read32(pe) != 0x4550
        || read16(pe + 4) != 0x14c
        || read32(pe + 8) != 0x3f544d3c
        || read32(pe + 24 + 56) != 0x309000
        || read32(pe + 24 + 16) != 0x23c947
    {
        return Err("Unsupported executable headers".into());
    }

    let language = language::configured_language()?;
    let bundle = language::load(language.code)?;
    let render_translations = build_render_translations(
        &bundle.entries,
        &bundle.overrides,
        bundle.fragments,
        bundle.strict,
    )?;
    // The reference language must match the pristine image exactly. Other languages
    // skip slots that another mod already redirected, instead of failing to start.
    let strict = bundle.strict;
    let mut entries = Vec::with_capacity(bundle.entries.len());
    let mut skipped = 0usize;
    for entry in bundle.entries {
        let source_end = entry
            .text_rva
            .checked_add(entry.source.as_bytes_with_nul().len())
            .ok_or("Source RVA overflow")?;
        if entry.slot_rva + 4 > 0x309000 || source_end > 0x309000 {
            return Err(format!(
                "Translation RVA is outside the supported image at 0x{:X}",
                entry.slot_rva
            ));
        }
        let slot = (base + entry.slot_rva) as *const u32;
        if slot.read_unaligned() as usize != base + entry.text_rva {
            if strict {
                return Err(format!(
                    "Unexpected source pointer at slot 0x{:X}",
                    entry.slot_rva
                ));
            }
            skipped += 1;
            continue;
        }
        let actual = std::slice::from_raw_parts(
            (base + entry.text_rva) as *const u8,
            entry.source.as_bytes_with_nul().len(),
        );
        if actual != entry.source.as_bytes_with_nul() {
            if strict {
                return Err(format!(
                    "Unexpected original text at slot 0x{:X}",
                    entry.slot_rva
                ));
            }
            skipped += 1;
            continue;
        }
        entries.push(entry);
    }
    if skipped > 0 {
        report_install_error(&format!(
            "{skipped} text slots belong to another mod and were left untouched"
        ));
    }

    let mut patches = hook_patches(base)?;
    patches.extend(entries.iter().map(|entry| {
        patch::Patch {
            address: base + entry.slot_rva,
            before: ((base + entry.text_rva) as u32).to_le_bytes().to_vec(),
            after: (entry.translation.as_ptr() as usize as u32)
                .to_le_bytes()
                .to_vec(),
        }
    }));
    let count = entries.len();
    // Keep replacement strings alive even if Windows prevents a complete rollback.
    TRANSLATIONS
        .set(entries)
        .map_err(|_| "Translations already initialized")?;
    RENDER_TRANSLATIONS
        .set(render_translations)
        .map_err(|_| "Render translations already initialized")?;
    GAME_BASE.store(base, Ordering::Release);
    patch::apply(&patches)?;
    Ok(count)
}

#[no_mangle]
pub extern "C" fn start() -> u32 {
    let result = INSTALL_RESULT.get_or_init(|| {
        panic::catch_unwind(|| unsafe { install() }).unwrap_or_else(|payload| {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("Unknown panic");
            Err(format!("Localization initialization panicked: {message}"))
        })
    });
    match result {
        Ok(_) => 0,
        Err(error) => {
            report_install_error(error);
            let message = format!(
                "Die Übersetzung konnte nicht vollständig geladen werden.\nThe localization could not be loaded completely.\n\n{error}"
            )
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
            let title = "Patrician III Localization"
                .encode_utf16()
                .chain(Some(0))
                .collect::<Vec<_>>();
            unsafe {
                MessageBoxW(
                    None,
                    PCWSTR(message.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    MB_OK | MB_ICONERROR,
                );
            }
            1
        }
    }
}

#[cfg(test)]
mod tests;
