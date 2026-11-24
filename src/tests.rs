use super::*;

mod translation_tests {
    use super::{
        decode_table_text, encode_cp1252, find_padded_render_translation, load_render_translations,
        load_translations, render_overrides, translate_bare_report_date,
        translate_chronicle_header, translate_city_signature, translate_day_count,
        translate_goods_supply_town, translate_lord_mayor, translate_manager_stage,
        translate_pirate_captain_intro, translate_pirate_capture, translate_report_date,
        translate_tavern_sailors, translate_town_information, RenderTranslations,
    };
    use crate::language::{self, Fragments};
    use std::{collections::HashMap, ffi::CString, sync::OnceLock};

    fn de() -> &'static Fragments {
        static DE: OnceLock<Fragments> = OnceLock::new();
        DE.get_or_init(|| language::load("de").unwrap().fragments)
    }

    #[test]
    fn table_escapes_preserve_carriage_returns() {
        let decoded = decode_table_text(r"line\nreturn\x0D");
        assert_eq!(decoded, "line\nreturn\r");
        assert_eq!(encode_cp1252(&decoded, "test").unwrap(), b"line\nreturn\r");
        assert_eq!(decode_table_text(r"\r"), r"\r");
    }

    #[test]
    fn translates_observed_forms_and_keeps_values() {
        assert_eq!(
            translate_town_information(de(), b"Population: 1700")
                .unwrap()
                .as_bytes(),
            b"Einwohnerzahl: 1700"
        );
        assert_eq!(
            translate_town_information(de(), b"70.000 inhabitants")
                .unwrap()
                .as_bytes(),
            b"70.000 Einwohner"
        );
        assert!(translate_town_information(de(), b"several inhabitants").is_none());
        assert_eq!(
            translate_town_information(de(), b"Information about L\xfcbeck")
                .unwrap()
                .as_bytes(),
            b"Informationen \xfcber L\xfcbeck"
        );
    }

    #[test]
    fn leaves_unrecognized_text_alone() {
        assert!(translate_town_information(de(), b"Population: unknown").is_none());
        assert!(translate_town_information(de(), b"Information about ").is_none());
    }

    #[test]
    fn translates_lord_mayor_label_without_changing_the_name() {
        assert_eq!(
            translate_lord_mayor(de(), b"Lord Mayor: the erfahrene Jakob Strohmberg")
                .unwrap()
                .as_bytes(),
            b"B\xfcrgermeister: der erfahrene Jakob Strohmberg"
        );
        assert_eq!(
            translate_lord_mayor(de(), b"Lady Mayoress: the erfahrene Ada Hesse")
                .unwrap()
                .as_bytes(),
            b"B\xfcrgermeisterin: die erfahrene Ada Hesse"
        );
        assert!(translate_lord_mayor(de(), b"Lord Mayor: the ").is_none());
        assert!(translate_lord_mayor(de(), b"Lady Mayoress: the ").is_none());
    }

    #[test]
    fn translates_dynamic_tavern_sailor_count() {
        assert_eq!(
            translate_tavern_sailors(de(), b"There are 28 sailors in the tavern.")
                .unwrap()
                .as_bytes(),
            b"In der Schenke sind 28 Seeleute."
        );
        assert!(translate_tavern_sailors(de(), b"There are many sailors in the tavern.").is_none());
        assert!(translate_tavern_sailors(de(), b"There are 28 sailors in the tavern!").is_none());
    }

    #[test]
    fn translates_pirate_capture_without_assuming_ship_gender() {
        assert_eq!(
            translate_pirate_capture(de(), b"The pirates take the Kraier Sturmvogel")
                .unwrap()
                .as_bytes(),
            b"Das Schiff Kraier Sturmvogel wird von den Piraten gekapert."
        );
        assert_eq!(
            translate_pirate_capture(de(), b"Die Piraten kapern Kogge Hanseat")
                .unwrap()
                .as_bytes(),
            b"Das Schiff Kogge Hanseat wird von den Piraten gekapert."
        );
        assert!(translate_pirate_capture(de(), b"The pirates take ").is_none());
    }

    #[test]
    fn translates_dynamic_pirate_captain_count() {
        assert_eq!(
            translate_pirate_captain_intro(
                de(),
                b"A fierce looking pirate captain with 14 henchmen offers"
            )
            .unwrap()
            .as_bytes(),
            b"Ein finster dreinblickender Piratenkapit\xe4n mit 14 Schergen bietet"
        );
        assert!(translate_pirate_captain_intro(
            de(),
            b"A fierce looking pirate captain with many henchmen offers"
        )
        .is_none());
    }

    #[test]
    fn translates_dynamic_game_labels_without_losing_values() {
        assert_eq!(
            translate_manager_stage(de(), b"Manager (Stage 3)")
                .unwrap()
                .as_bytes(),
            b"Verwalter (Stufe 3)"
        );
        assert!(translate_manager_stage(de(), b"Manager (Stage III)").is_none());
    }

    #[test]
    fn localizes_report_dates_and_months_as_cp1252() {
        assert_eq!(
            translate_report_date(de(), b"on 31. May 1345  ")
                .unwrap()
                .as_bytes(),
            b"am 31. Mai 1345  "
        );
        assert_eq!(
            translate_report_date(de(), b"from 1. Mar 1345 to 2. Oct 1345")
                .unwrap()
                .as_bytes(),
            b"vom 1. M\xe4r 1345 bis zum 2. Okt 1345"
        );
        assert_eq!(
            translate_bare_report_date(de(), b"1.Oct 1345")
                .unwrap()
                .as_bytes(),
            b"1.Okt 1345"
        );
        assert_eq!(
            translate_bare_report_date(de(), b"15. Apr 1358")
                .unwrap()
                .as_bytes(),
            b"15. Apr 1358"
        );
        assert_eq!(
            translate_bare_report_date(de(), b"15. May 1358")
                .unwrap()
                .as_bytes(),
            b"15. Mai 1358"
        );
        assert!(translate_bare_report_date(de(), b"1.Jul").is_none());
    }

    #[test]
    fn localizes_chronicle_header_month_and_year() {
        assert_eq!(
            translate_chronicle_header(de(), b"Chronik vom 27. Oct 1400")
                .unwrap()
                .as_bytes(),
            b"Chronik vom 27. Okt 1400"
        );
        assert_eq!(
            translate_chronicle_header(de(), b"Chronik vom 15. Apr 1358")
                .unwrap()
                .as_bytes(),
            b"Chronik vom 15. Apr 1358"
        );
    }

    #[test]
    fn translates_dynamic_goods_supply_town_and_keeps_padding() {
        assert_eq!(
            translate_goods_supply_town(de(), b"Goods supply in L\xfcbeck  ")
                .unwrap()
                .as_bytes(),
            b"Warenversorgung in L\xfcbeck  "
        );
        assert!(translate_goods_supply_town(de(), b"Goods supply in ").is_none());
        assert!(translate_goods_supply_town(de(), b"Goods supply in L\xfcbeck!").is_none());
    }

    #[test]
    fn separates_city_name_from_german_city_signature() {
        assert_eq!(
            translate_city_signature(de(), b"Die StadtRiga")
                .unwrap()
                .as_bytes(),
            b"Die Stadt Riga"
        );
        assert_eq!(
            translate_city_signature(de(), b"Die StadtL\xfcbeck")
                .unwrap()
                .as_bytes(),
            b"Die Stadt L\xfcbeck"
        );
        assert!(translate_city_signature(de(), b"Die Stadtmauer").is_none());
        assert!(translate_city_signature(de(), b"Die StadtAugsburg").is_none());
    }

    #[test]
    fn translates_dynamic_day_counts_with_german_pluralization() {
        assert_eq!(
            translate_day_count(de(), b"1 day").unwrap().as_bytes(),
            b"1 Tag"
        );
        assert_eq!(
            translate_day_count(de(), b"2 days").unwrap().as_bytes(),
            b"2 Tage"
        );
        assert_eq!(
            translate_day_count(de(), b"0 days").unwrap().as_bytes(),
            b"0 Tage"
        );
        assert!(translate_day_count(de(), b"one day").is_none());
        assert_eq!(
            translate_day_count(de(), b"1 days").unwrap().as_bytes(),
            b"1 Tag"
        );
    }

    #[test]
    fn padded_translation_preserves_source_spacing() {
        let translations = RenderTranslations {
            exact: HashMap::from([(
                b"ship data".to_vec(),
                CString::new("Daten der Schiffe").unwrap(),
            )]),
            ..RenderTranslations::default()
        };
        assert_eq!(
            find_padded_render_translation(b" ship data  ", &translations)
                .unwrap()
                .as_bytes(),
            b" Daten der Schiffe  "
        );
        assert!(find_padded_render_translation(b" unknown ", &translations).is_none());
    }

    #[test]
    fn context_specific_fragments_are_not_global_render_keys() {
        let entries = load_translations().unwrap();
        let translations = load_render_translations(&entries).unwrap();
        for text in [b"to".as_slice(), b".\"", b"per", b"Name"] {
            assert!(translations.get(text).is_none());
        }
        let christen = entries
            .iter()
            .find(|entry| entry.slot_rva == 0x2B0384)
            .unwrap();
        assert_eq!(christen.source.as_bytes(), b"Name");
        assert_eq!(christen.translation.as_bytes(), b"Taufen");
        let feminine = entries
            .iter()
            .find(|entry| entry.slot_rva == 0x2A0C24)
            .unwrap();
        let masculine = entries
            .iter()
            .find(|entry| entry.slot_rva == 0x2A0C60)
            .unwrap();
        assert_eq!(feminine.source.as_bytes(), b"experienced");
        assert_eq!(masculine.source.as_bytes(), b"experienced");
        assert_eq!(feminine.translation.as_bytes(), b"erfahrene");
        assert_eq!(masculine.translation.as_bytes(), b"erfahrener");
        assert_eq!(translations.get(b"school").unwrap().as_bytes(), b"Schule");
        assert_eq!(
            translations
                .get(b"he demands 50% of the loot.")
                .unwrap()
                .as_bytes(),
            b"Er verlangt 50 % der Beute."
        );
        assert_eq!(
            translations.get(b"weekly production").unwrap().as_bytes(),
            b"Wochenproduktion"
        );
        assert_eq!(
            translations.get(b"trading information").unwrap().as_bytes(),
            b"Handelsinformationen"
        );
        assert_eq!(
            translations.get(b"no ships available").unwrap().as_bytes(),
            encode_cp1252("Keine Schiffe verfügbar", "test").unwrap()
        );
        assert_eq!(
            translations.get(b"dismiss sailors").unwrap().as_bytes(),
            b"Matrosen entlassen"
        );
        assert_eq!(
            translations.get(b"dismiss captain").unwrap().as_bytes(),
            b"Kapitain entlassen"
        );
        assert_eq!(
            translations.get(b"sea battle").unwrap().as_bytes(),
            b"Seeschlacht"
        );
        assert_eq!(
            translations.get(b"date of game").unwrap().as_bytes(),
            b"Spieldatum"
        );
        assert_eq!(
            translations.get(b"autosave").unwrap().as_bytes(),
            encode_cp1252("Automatisches Speichern", "test").unwrap()
        );
        assert_eq!(
            translations.get(b"city buildings").unwrap().as_bytes(),
            encode_cp1252("Stadtgebäude", "test").unwrap()
        );
        assert_eq!(translations.get(b"crew").unwrap().as_bytes(), b"Besatzung");
        assert_eq!(
            translations.get(b"pro load").unwrap().as_bytes(),
            b"pro Ladung"
        );
        assert_eq!(
            translations.get(b"1.000 load").unwrap().as_bytes(),
            b"1.000 Last"
        );
        assert_eq!(
            translations.get(b"2.000 load").unwrap().as_bytes(),
            b"2.000 Last"
        );
        assert_eq!(
            translations.get(b"choose mission").unwrap().as_bytes(),
            encode_cp1252("Mission wählen", "test").unwrap()
        );
        assert_eq!(translations.get(b"no").unwrap().as_bytes(), b"Nein");
        assert_eq!(
            translations.get(b"tooltips").unwrap().as_bytes(),
            b"Hinweise"
        );
        assert_eq!(translations.get(b"auction").unwrap().as_bytes(), b"Auktion");
        assert_eq!(translations.get(b"guild").unwrap().as_bytes(), b"Gilde");
        for (_, source, translation) in render_overrides()
            .unwrap()
            .into_iter()
            .filter(|(scope, _, _)| *scope == "render")
        {
            assert_eq!(
                translations
                    .get(&encode_cp1252(source, "official German name source").unwrap())
                    .unwrap()
                    .as_bytes(),
                encode_cp1252(translation, "official German name test").unwrap(),
                "official resource name was not translated: {source}"
            );
        }
        assert_eq!(
            translations
                .get(b"amtliche nachricht: elections")
                .unwrap()
                .as_bytes(),
            b"Amtliche Nachricht: Wahlen"
        );
        for (source, translation) in [
            (b"ships in port".as_slice(), b"Schiffe im Hafen".as_slice()),
            (b"ships at sea", b"Schiffe auf See"),
            (b"convoys in port", b"Konvois im Hafen"),
            (b"convoys at sea", b"Konvois auf See"),
        ] {
            assert_eq!(translations.get(source).unwrap().as_bytes(), translation);
        }
    }
}

mod renderer_tests {
    use super::*;

    #[test]
    fn embedded_table_parses_and_builds_render_translations() {
        let entries = load_translations().unwrap();
        let first = entries.first().unwrap();
        assert_eq!(first.slot_rva, 0x299868);
        assert_eq!(first.source.as_bytes(), b"Trading goods");
        assert_eq!(first.translation.as_bytes(), b"Warenhandel");
        let render = load_render_translations(&entries).unwrap();
        assert_eq!(
            render.get(b"trading goods".as_slice()).unwrap().as_bytes(),
            b"Warenhandel"
        );
        assert_eq!(render.get(b"Capacity").unwrap().as_bytes(), b"Laderaum");
        assert_eq!(
            render.get(b"capacity").unwrap().as_bytes(),
            encode_cp1252("Kapazität", "test").unwrap()
        );
        assert!(render.get(b"CAPACITY").is_none());
        assert_eq!(
            render.get(b"Town production").unwrap().as_bytes(),
            b"Stadtproduktion"
        );
    }

    #[test]
    fn translates_addon_labels_and_preserves_resource_town_names() {
        let entries = load_translations().unwrap();
        RENDER_TRANSLATIONS.get_or_init(|| load_render_translations(&entries).unwrap());
        for (source, target) in [
            ("Demolish building", "Gebäude abreißen"),
            ("Ready", "Bereit"),
            (
                " Weekly production (fixed)",
                " Wochenproduktion (korrigiert)",
            ),
            ("Public notices", "Öffentliche Nachrichten"),
            (
                "Quantity, purchase maximum",
                "Bestand, bis zu dem eingekauft wird",
            ),
            ("Utilization Markup", "Auslastungszuschlag"),
            ("Pending Experience (scaled)", "Ausstehende EP (skaliert)"),
            ("Experience (scaled)", "Erfahrung (skaliert)"),
            ("Quality Level", "Qualitätsstufe"),
            ("Required XP", "Benötigte EP"),
            (
                "Are you willing to announce a celebration?",
                "Seid Ihr willens, ein Fest anzukündigen?",
            ),
        ] {
            let actual = translate_render_text(source.as_bytes()).unwrap();
            assert_eq!(
                actual.to_bytes(),
                encode_cp1252(target, "expected").unwrap()
            );
        }
        assert!(translate_render_text(b"Cologne").is_none());
        assert!(translate_render_text(b"K\xf6ln").is_none());
        assert_eq!(
            translate_render_text(b"Information about K\xf6ln")
                .unwrap()
                .to_bytes(),
            b"Informationen \xfcber K\xf6ln",
        );
    }

    #[test]
    fn translates_addon_ware_debug_names_and_spaced_labels() {
        let entries = load_translations().unwrap();
        let render = load_render_translations(&entries).unwrap();
        for (source, target) in [
            ("IronGoods", "Eisenwaren"),
            ("Iron Goods", "Eisenwaren"),
            ("iron goods", "Eisenwaren"),
            ("WhaleOil", "Tran"),
            ("Whale Oil", "Tran"),
            ("PigIron", "Eisenerz"),
            ("Pig Iron", "Eisenerz"),
        ] {
            assert_eq!(
                render.get(source.as_bytes()).unwrap().as_bytes(),
                encode_cp1252(target, "expected").unwrap(),
                "{source}"
            );
        }
    }

    #[test]
    fn addon_callers_get_ware_names_without_renaming_buildings() {
        let entries = load_translations().unwrap();
        RENDER_TRANSLATIONS.get_or_init(|| load_render_translations(&entries).unwrap());
        let expected = |text| encode_cp1252(text, "expected").unwrap();
        assert_eq!(
            translate_render_text(b"Pottery").unwrap().to_bytes(),
            expected("Töpferei")
        );
        assert_eq!(
            translate_addon_render_text(b"Pottery").unwrap().to_bytes(),
            b"Keramik"
        );
        assert_eq!(
            translate_addon_render_text(b"IronGoods")
                .unwrap()
                .to_bytes(),
            b"Eisenwaren"
        );
        assert_eq!(
            translate_addon_render_text(b"Trading goods")
                .unwrap()
                .to_bytes(),
            b"Warenhandel"
        );
        let base = 0x400000;
        assert!(!is_addon_caller(base, base));
        assert!(!is_addon_caller(base + UI_RENDER_TEXT_AT_RVA, base));
        assert!(is_addon_caller(base + GAME_IMAGE_SIZE, base));
        assert!(is_addon_caller(0x1000_0000, base));
        assert!(!is_addon_caller(0x1000_0000, 0));
    }

    static FORWARDED: std::sync::Mutex<Option<(usize, i32, i32, usize)>> =
        std::sync::Mutex::new(None);

    unsafe extern "C" fn record_forwarded(
        caller: usize,
        x: i32,
        y: i32,
        text: *const std::ffi::c_void,
    ) {
        *FORWARDED.lock().unwrap() = Some((caller, x, y, text as usize));
    }

    caller_forwarding_stub!(forwarding_probe => record_forwarded);

    #[test]
    fn forwarding_stub_passes_caller_and_original_arguments() {
        let probe: unsafe extern "C" fn(i32, i32, *const std::ffi::c_void) =
            unsafe { std::mem::transmute(forwarding_probe as *const ()) };
        let text = b"Pottery\0";
        unsafe { probe(-7, 42, text.as_ptr().cast()) };
        let (caller, x, y, forwarded_text) = FORWARDED.lock().unwrap().unwrap();
        assert_eq!((x, y, forwarded_text), (-7, 42, text.as_ptr() as usize));
        let test_start = forwarding_stub_passes_caller_and_original_arguments as *const () as usize;
        assert!((test_start..test_start + 0x1000).contains(&caller));
    }
}

mod regression_tests {
    use super::*;

    #[test]
    fn cp1252_supports_punctuation_and_rejects_unrepresentable_characters() {
        assert_eq!(
            encode_cp1252("€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œžŸ", "test").unwrap(),
            [
                0x80, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8e, 0x91,
                0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9e, 0x9f
            ]
        );
        assert_eq!(
            encode_cp1252("ÄÖÜäöüß", "test").unwrap(),
            [0xc4, 0xd6, 0xdc, 0xe4, 0xf6, 0xfc, 0xdf]
        );
        assert!(encode_cp1252("漢", "test").is_err());
    }

    #[test]
    fn surrender_uses_resource_identity_without_overriding_gameplay_contexts() {
        let entries = load_translations().unwrap();
        let render = load_render_translations(&entries).unwrap();
        assert!(render.get(b"Surrender").is_none());
        assert!(render.get(b"sUrReNdEr").is_none());
        assert!(render.get(b"WEALTH").is_none());
        let (source, translation) = &render.resources[&20052];
        assert_eq!(source.as_bytes(), b"Surrender");
        assert_eq!(translation.as_bytes(), b"Aufgeben");
        unsafe extern "system" fn original_resource(
            _instance: isize,
            _id: u32,
            output: *mut u8,
            capacity: i32,
        ) -> i32 {
            if capacity <= 0 || output.is_null() {
                return 0;
            }
            let source = b"Surrender";
            let length = source.len().min(capacity as usize - 1);
            std::ptr::copy_nonoverlapping(source.as_ptr(), output, length);
            *output.add(length) = 0;
            length as i32
        }
        ORIGINAL_LOAD_STRING
            .set(original_resource as *const () as usize)
            .unwrap();
        RENDER_TRANSLATIONS.get_or_init(|| load_render_translations(&entries).unwrap());
        GAME_BASE.store(0x400000, Ordering::Release);
        for (capacity, expected) in [
            (32, b"Aufgeben".as_slice()),
            (9, b"Aufgeben"),
            (4, b"Auf"),
            (1, b""),
        ] {
            let mut buffer = [0xcc; 32];
            let count = unsafe { load_string_hook(0x400000, 20052, buffer.as_mut_ptr(), capacity) };
            assert_eq!(count as usize, expected.len());
            assert_eq!(&buffer[..expected.len()], expected);
            assert_eq!(buffer[expected.len()], 0);
            assert!(buffer[capacity as usize..].iter().all(|byte| *byte == 0xcc));
        }
        let mut untouched = [0xcc; 32];
        unsafe {
            load_string_hook(0x400000, 20044, untouched.as_mut_ptr(), 32);
        }
        assert_eq!(&untouched[..9], b"Surrender");
        unsafe {
            load_string_hook(0x500000, 20052, untouched.as_mut_ptr(), 32);
        }
        assert_eq!(&untouched[..9], b"Surrender");
        assert_eq!(
            unsafe { load_string_hook(0x400000, 20052, std::ptr::null_mut(), 0) },
            0
        );
        let contextual = entries
            .iter()
            .filter(|entry| entry.source.as_bytes() == b"Surrender")
            .map(|entry| entry.translation.as_bytes())
            .collect::<HashSet<_>>();
        assert_eq!(
            contextual,
            HashSet::from([b"Kapitulation".as_slice(), b"Kapitulieren".as_slice()])
        );
    }

    #[test]
    fn player_titles_are_translated_before_formatting_for_every_rank_and_adjective() {
        let entries = load_translations().unwrap();
        let at = |slot| {
            entries
                .iter()
                .find(|entry| entry.slot_rva == slot)
                .unwrap()
                .translation
                .as_bytes()
        };
        // FUN_0047dd40 selects sex 0/1, fourteen adjectives, and eight ranks per sex.
        assert_eq!(
            [at(0x2C0EF8), at(0x2C0EFC)],
            [b"Die".as_slice(), b"Der".as_slice()]
        );
        assert_eq!(
            [at(0x2C0ED8), at(0x2C0EDC)],
            [b"die".as_slice(), b"der".as_slice()]
        );
        // Index 12 is already the German word "aggressive" and needs no patch.
        for index in (0..14).filter(|index| *index != 12) {
            let adjective = at(0x29EE48 + index * 4);
            assert!(!adjective.is_empty());
            assert_eq!(adjective, at(0x2A0C18 + index * 4));
        }
        for sex in 0..2 {
            for rank in 0..8 {
                let title = at(0x2A0608 + (sex * 8 + rank) * 4);
                assert!(!title.is_empty());
                assert!(![
                    b"shopkeeper".as_slice(),
                    b"trader",
                    b"merchant",
                    b"Councillor",
                    b"Patrician"
                ]
                .contains(&title));
            }
        }
        for slot in [0x2B3678, 0x2AA184, 0x29C86C, 0x2C2A34] {
            let format = at(slot);
            assert!(!format.windows(4).any(|part| part == b"The "));
        }
    }
}

mod language_tests {
    use crate::language::{self, Bundle, Fragments, LANGUAGES};
    use crate::{
        build_render_translations, decode_table_text, encode_cp1252, load_translations,
        translate_bare_report_date, translate_chronicle_header, translate_day_count,
        translate_goods_supply_town, translate_lord_mayor, translate_manager_stage,
        translate_pirate_captain_intro, translate_pirate_capture, translate_report_date,
        translate_tavern_sailors, translate_town_information,
    };

    const REQUIRED_PLACEHOLDERS: &[(&str, &[&str])] = &[
        ("report_on", &["{date}"]),
        ("report_range", &["{start}", "{end}"]),
        ("chronicle", &["{date}"]),
        ("goods_supply", &["{town}"]),
        ("town_inhabitants", &["{n}"]),
        ("town_information", &["{name}"]),
        ("town_population", &["{n}"]),
        ("lord_mayor", &["{name}"]),
        ("lady_mayoress", &["{name}"]),
        ("tavern_sailors", &["{n}"]),
        ("pirate_capture", &["{ship}"]),
        ("pirate_captain", &["{n}"]),
        ("manager_stage", &["{n}"]),
        ("day_one", &["{n}"]),
        ("day_many", &["{n}"]),
    ];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    /// printf conversions (`%s`, `%u`, `%%`, ...) in order.
    fn conversions(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut chars = text.chars().peekable();
        while let Some(character) = chars.next() {
            if character != '%' {
                continue;
            }
            let mut conversion = String::from("%");
            for next in chars.by_ref() {
                conversion.push(next);
                if next.is_ascii_alphabetic() || next == '%' {
                    break;
                }
            }
            found.push(conversion);
        }
        found
    }

    /// Backslash markup (`\c`, `\f1_`, `\n`, ...) as (backslash, next character) pairs.
    fn markup(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut chars = text.chars();
        while let Some(character) = chars.next() {
            if character == '\\' {
                found.push(format!("\\{}", chars.next().unwrap_or(' ')));
            }
        }
        found
    }

    fn bundle(code: &str) -> Bundle {
        language::load(code).unwrap_or_else(|error| panic!("{code}: {error}"))
    }

    #[test]
    fn language_codes_are_unique_lowercase_and_german_is_the_default() {
        let mut seen = std::collections::HashSet::new();
        for language in LANGUAGES {
            assert!(seen.insert(language.code), "duplicate {}", language.code);
            assert_eq!(language.code, language.code.to_ascii_lowercase());
        }
        assert!(language::find(language::DEFAULT_LANGUAGE).is_some());
        assert!(language::resolve("xx").is_err());
    }

    #[test]
    fn language_setting_is_parsed_from_the_ini() {
        let parse = language::parse_language_setting;
        assert_eq!(parse("language=fr").as_deref(), Some("fr"));
        assert_eq!(
            parse("; comment\r\n[general]\r\n Language = \"FR-be\" \r\n").as_deref(),
            Some("fr")
        );
        assert_eq!(parse("\u{feff}language=nl_NL").as_deref(), Some("nl"));
        assert_eq!(parse("# language=fr\nvolume=3"), None);
        assert_eq!(parse("language="), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn german_bundle_equals_the_legacy_tables() {
        let german = bundle("de");
        assert!(german.strict);
        assert_eq!(german.entries.len(), load_translations().unwrap().len());
        assert!(german.fragments.get("month.Mar").is_some());
    }

    #[test]
    fn every_language_table_is_valid_and_keeps_placeholders_and_markup() {
        for language in LANGUAGES.iter().filter(|language| language.code != "de") {
            let bundle = bundle(language.code);
            assert!(!bundle.entries.is_empty(), "{} is empty", language.code);
            assert!(!bundle.strict);
            let rows = language::scoped_rows(language.table).unwrap();
            let mut checked = 0;
            for (scope, source, translation) in &rows {
                if !scope.starts_with("0x") {
                    continue;
                }
                let (source, translation) =
                    (decode_table_text(source), decode_table_text(translation));
                assert_eq!(
                    conversions(&source),
                    conversions(&translation),
                    "{}: conversions differ at {scope}: {source} -> {translation}",
                    language.code
                );
                assert_eq!(
                    markup(&source),
                    markup(&translation),
                    "{}: markup differs at {scope}: {source} -> {translation}",
                    language.code
                );
                assert_eq!(
                    source.matches('\n').count(),
                    translation.matches('\n').count(),
                    "{}: line breaks differ at {scope}",
                    language.code
                );
                assert_eq!(
                    source.starts_with(char::is_whitespace),
                    translation.starts_with(char::is_whitespace),
                    "{}: leading space differs at {scope}",
                    language.code
                );
                assert_eq!(
                    source.ends_with(char::is_whitespace),
                    translation.ends_with(char::is_whitespace),
                    "{}: trailing space differs at {scope}",
                    language.code
                );
                checked += 1;
            }
            assert_eq!(
                checked,
                bundle.entries.len(),
                "{}: slot rows",
                language.code
            );
            // Every translated slot must also exist in the German master or the extra table.
            assert!(bundle.entries.iter().all(|entry| entry.slot_rva > 0));
            build_render_translations(&bundle.entries, &bundle.overrides, bundle.fragments, false)
                .unwrap_or_else(|error| panic!("{}: {error}", language.code));
        }
    }

    /// The game font only has glyphs for ASCII, `äöüÄÖÜß` and the lowercase accents below
    /// (checked in game); any other character renders as a blank gap.
    #[test]
    fn language_tables_only_use_characters_the_game_font_can_draw() {
        const FONT_EXTRA: &str = "äöüÄÖÜßáàâéèêíìîóòôúùû";
        for language in LANGUAGES.iter().filter(|language| language.code != "de") {
            for (scope, _, translation) in language::scoped_rows(language.table).unwrap() {
                let text = decode_table_text(translation);
                if let Some(c) = text
                    .chars()
                    .find(|c| !(c.is_ascii() || FONT_EXTRA.contains(*c)))
                {
                    panic!("{}: {c:?} has no glyph at {scope}: {text}", language.code);
                }
            }
        }
    }

    #[test]
    fn language_fragments_are_complete_and_well_formed() {
        for language in LANGUAGES {
            let frags: Fragments = bundle(language.code).fragments;
            for (key, placeholders) in REQUIRED_PLACEHOLDERS {
                if let Some(template) = frags.get(key) {
                    for placeholder in *placeholders {
                        assert!(
                            template.contains(placeholder),
                            "{}: {key} lacks {placeholder}",
                            language.code
                        );
                    }
                    encode_cp1252(template, key).unwrap();
                }
            }
            let months = MONTHS
                .iter()
                .filter(|month| frags.get(&format!("month.{month}")).is_some())
                .count();
            assert!(
                months == 0 || months == 12,
                "{}: partial month table",
                language.code
            );
            let known = REQUIRED_PLACEHOLDERS
                .iter()
                .map(|(key, _)| *key)
                .chain(MONTHS.iter().map(|_| "month."))
                .chain(["city_prefix"])
                .collect::<Vec<_>>();
            for key in frags.keys() {
                assert!(
                    known.iter().any(|known| key.starts_with(known)),
                    "{}: unknown fragment {key}",
                    language.code
                );
            }
        }
    }

    #[test]
    fn dynamic_translators_use_the_language_fragments() {
        let french = bundle("fr").fragments;
        let fr = &french;
        assert_eq!(
            translate_report_date(fr, b"on 5. Mar 1300")
                .unwrap()
                .as_bytes(),
            b"le 5. mars 1300"
        );
        assert_eq!(
            translate_report_date(fr, b"from 1. Dec 1300 to 3. Jan 1301  ")
                .unwrap()
                .as_bytes(),
            b"du 1. d\xe9c. 1300 au 3. janv. 1301  "
        );
        assert_eq!(
            translate_bare_report_date(fr, b"12. Oct 1305")
                .unwrap()
                .as_bytes(),
            b"12. oct. 1305"
        );
        assert_eq!(
            translate_chronicle_header(fr, b"chronique du 5. May 1300")
                .unwrap()
                .as_bytes(),
            b"chronique du 5. mai 1300"
        );
        assert_eq!(
            translate_goods_supply_town(fr, b"Goods supply in L\xfcbeck  ")
                .unwrap()
                .as_bytes(),
            b"Approvisionnement en marchandises \xe0 L\xfcbeck  "
        );
        assert_eq!(
            translate_town_information(fr, b"12.500 inhabitants")
                .unwrap()
                .as_bytes(),
            b"12.500 habitants"
        );
        assert_eq!(
            translate_lord_mayor(fr, b"Lord Mayor: the Hinrich Castorp")
                .unwrap()
                .as_bytes(),
            b"Bourgmestre : Hinrich Castorp"
        );
        assert_eq!(
            translate_tavern_sailors(fr, b"There are 12 sailors in the tavern.")
                .unwrap()
                .as_bytes(),
            b"Il y a 12 matelots \xe0 la taverne."
        );
        assert_eq!(
            translate_pirate_capture(fr, b"The pirates take the Cog Anna")
                .unwrap()
                .as_bytes(),
            b"Le navire Cog Anna est pris par les pirates."
        );
        assert!(translate_pirate_captain_intro(
            fr,
            b"A fierce looking pirate captain with 7 henchmen offers"
        )
        .is_some());
        assert_eq!(
            translate_manager_stage(fr, b"Manager (Stage 3)")
                .unwrap()
                .as_bytes(),
            b"Administrateur (niveau 3)"
        );
        assert_eq!(
            translate_day_count(fr, b"1 day").unwrap().as_bytes(),
            b"1 jour"
        );
        assert_eq!(
            translate_day_count(fr, b"4 days").unwrap().as_bytes(),
            b"4 jours"
        );
        // A language without a fragment leaves the English text alone.
        let empty = Fragments::default();
        assert!(translate_report_date(&empty, b"on 5. Mar 1300").is_none());
        assert!(translate_day_count(&empty, b"4 days").is_none());
    }
}
