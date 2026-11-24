# Languages

The mod patches text pointers in the game image. The same technique works for every language that can
be displayed in the Windows-1252 character set of the game fonts. German is the reference language and
is loaded unchanged from `data/german_localization.tsv` and `data/render_translations.tsv`.

## Choosing a language

Put `localization.ini` into the `mods\` folder (next to the DLL) or into the game folder:

```ini
language=fr
```

Codes: `de`, `fr`, `nl`, `es`, `it`, `sv`, `da`, `no`, `pt`. `fr`, `fr-FR` and `FR_fr` are equivalent. An
unknown code aborts the start with a message that lists all available languages. Without the file,
German is used.

| Code | Language   | Status |
|------|------------|--------|
| de   | Deutsch    | complete (reference) |
| fr   | Français   | all slots translated, not proofread |
| nl   | Nederlands | all slots translated, not proofread |
| es   | Español    | all slots translated, not proofread |
| it   | Italiano   | all slots translated, not proofread |
| sv   | Svenska    | all slots translated, not proofread |
| da   | Dansk      | all slots translated, not proofread |
| no   | Norsk      | all slots translated, not proofread |
| pt   | Português  | all slots translated, not proofread |

Untranslated texts stay English. A slot that another mod has already redirected is skipped in the
additional languages. German still aborts with an error message in that case.

## File format `data/lang/<code>.tsv`

Three columns, tab-separated, `#` starts a comment:

| Scope        | Meaning |
|--------------|---------|
| `0x…`        | Text slot in the game image. Column 2 is the English original and must match the slot table exactly. |
| `render`     | Exact text that is translated only when drawn (also from other mods). |
| `addon`      | Like `render`, but only for callers outside the game image (e.g. goods names of a mod). |
| `resource:N` | String resource N. |
| `frag`       | Template for the pattern translators: month names `month.Jan` … `month.Dec`, `report_on`, `report_range`, `chronicle`, `goods_supply`, `town_inhabitants`, `town_information`, `town_population`, `lord_mayor`, `lady_mayoress`, `tavern_sailors`, `pirate_capture`, `pirate_captain`, `manager_stage`, `day_one`, `day_many`. Placeholders in `{…}`. |

Slots come from `data/german_localization.tsv` and `data/extra_slots.tsv` (texts that are identical in
German and English and therefore missing in the German table). Month names contain no space. If a
template is missing, the text concerned stays English.

For every embedded language, `cargo test` checks: valid slot, English original matches, `%`
placeholders, backslash markup, line breaks and edge spaces unchanged, characters representable in
Windows-1252, templates complete, render table buildable.

## Mission and campaign texts (`data/files/<code>.tsv`)

Missions (`*.p2m`) and the campaign list (`scripts/missions_eng.ini`) are not part of the game image. The
mod hooks the archive reads of `archiver.dll`, reads the English original, swaps its texts for the language
and gives the game the result from memory (`src/files.rs`). Files that do not match the English originals,
for example loose replacement files, reach the game unchanged. Three columns, tab-separated, `#` starts a
comment:

| Scope        | Meaning |
|--------------|---------|
| `p2m`        | Text of a mission script. Column 2 is the English original in the mission text codec: `\n` = line break, `\t`, `\\`, `\xHH`, `{%X:HH}` = format token `%X` with argument byte `HH`. The translation must keep the tokens. |
| `p2m:<file>` | Like `p2m`, but only in that mission (file name without folder, any case). It wins over `p2m` rows and is meant for an English sentence that a language translates differently from mission to mission. The tokens are not compared. |
| `ini`        | Value of a `key=value` line of the campaign list. |

`data/files/source.tsv` lists every English original (579 mission texts, 25 campaign texts). A table only
needs rows for texts that differ from English. `cargo test` checks that every row refers to an original, that
the rows decode, and that the tokens and the font limits (see below, except for German) are kept.

## What is translated (and what is not)

All 3186 text slots of the slot table are filled for fr, nl, es, it, sv, da, no and pt, plus the
templates (`frag`), render and addon texts. That covers menus, labels, names, letters, event and
chronicle texts and the composed sentence fragments. Missions (579 texts) and the campaign list
(25 texts) are translated for all languages as well, see below. Not translated: the tutorial script,
credits and name lists, and images that contain text. German additionally has translated game files,
shipped separately as `patrician3-localization-<version>-resources-de.zip` (source: `resources/de/`):
tutorial, credits, name lists, map, menu pictures and 19 mission files. Of these, 15 have scripts
that differ from the English ones, 3 carry title wording that the tables cannot give per position,
and `Kampagne3_Entfuehrung.p2m` is the name the campaign list uses while the English archive holds
the file under a name with an umlaut. All other German missions come from the tables like in any
other language.

Limits of quality (cannot be verified by machine):

- Fragment chains and gender, case and genitive forms follow the slot structure of the German
  template and are approximations; some sentences read choppily in the game.
- The order of placeholders follows English, even where the target language would order differently.
- No review by native speakers. Loading in the game has been checked for `de`, `nl` and `sv`. For all
  nine languages the campaign selection and the first campaign letter have been run in the game; other
  missions have not.

## Character set

The game font is a bitmap font that only contains ASCII, `ä ö ü Ä Ö Ü ß` and the lowercase accents
`á à â é è ê í ì î ó ò ô ú ù û` (checked glyph by glyph in the game). Every other character is drawn as
an empty gap, so the translations use these substitutions (a unit test enforces the allowed set):

| Language | Substitution |
|---|---|
| sv | `å` -> `a` |
| da, no | `å` -> `aa`, `æ` -> `ae`, `ø` -> `oe` (accepted digraph spelling) |
| es | `ñ` -> `ni`, `¡` `¿` dropped |
| pt | `ã` -> `a`, `õ` -> `o`, `ç` -> `c` |
| fr | `ç` -> `c`, `ï` -> `i`, `œ` -> `oe`, `« »` -> `"` |
| nl | `ë` -> `e`, `ï` -> `i` |
| it | `È` -> `E'` |
| all | capital accented letters lose the accent (`É` -> `E`), `º ª °` -> `o a o`, typographic quotes and dashes -> ASCII |

New fonts (or a font patch) would be needed for real `å æ ø ñ ç ã õ` glyphs. Not possible without
that: Polish, Czech, Hungarian (partly), Baltic languages, Cyrillic, Greek.

## Style: the Hanseatic Middle Ages

Texts are translated from the point of view of the Hanseatic period (Kontore, town council, guilds,
cogs), not in dictionary or machine style. Guidelines:

1. Historically attested terms of the respective language before modern administrative terms
   (Lord Mayor = bourgmestre / burgemeester / borgmester, not "Maire" or "Mayor").
2. Hanseatic terms without a common equivalent stay in their Hanseatic form
   (Alderman, Kontor, Schnigge, Kraier).
3. The player is addressed in the polite or plural form of the language, not in today's everyday
   address ("vous", "u", "ni", "De").
4. Currencies and measures in the unit of the time (Last, not tonne).
5. Short labels stay short: buttons in the base form, without punctuation.
6. English is authoritative for the meaning; German is only a reference for length and context.

### Working glossary (suggestions)

| English | Deutsch | Français | Nederlands | Español | Italiano | Svenska | Dansk | Norsk | Português |
|---|---|---|---|---|---|---|---|---|---|
| Trading office | Kontor | comptoir | kantoor | Kontor | fondaco | kontor | kontor | kontor | escritório |
| Hanseatic League | Hanse | Hanse | Hanze | Hansa | Lega anseatica | Hansan | Hansaen | Hansaen | Hansa |
| Hanse Day | Hansetag | diète hanséatique | Hanzedag | dieta hanseática | dieta anseatica | hansedag | hansedag | hansadag | assembleia da Hansa |
| Alderman | Eldermann | alderman | olderman | alderman | alderman | äldermann | oldermand | olderman | vereador |
| Lord Mayor | Bürgermeister | bourgmestre | burgemeester | burgomaestre | borgomastro | borgmästare | borgmester | borgermester | burgomestre |
| Guild | Gilde | guilde | gilde | gremio | gilda | gille | lav | gilde | guilda |
| Cog | Kogge | cogue | kogge | coca | cocca | kogg | kogge | kogge | cogue |
| Hulk | Holk | hourque | hulk | holk | holk | holk | holk | holk | hulque |
| Snaikka | Schnigge | schnigge | snikke | schnigge | schnigge | snäcka | snekke | schnigge | schnigge |
| Crayer | Kraier | kraier | kraeyer | kraier | kraier | kraier | kraier | kraier | kraier |
| Load (measure) | Last | last | last | last | last | läst | læst | last | last |
| Prince | Landesfürst | prince territorial | landsheer | príncipe | principe | furste | fyrste | fyrste | príncipe |
| Town hall | Rathaus | hôtel de ville | stadhuis | ayuntamiento | municipio | rådhus | rådhus | rådhus | câmara municipal |

All languages are translated in this form; the glossary is the common starting point, individual words
differ in the tables (e.g. Schnigge stays Schnigge, "Pitch shoot" is translated freely depending on the
language). All additional languages should be proofread by native speakers before publication.

## Adding a new language

1. Create `data/lang/<code>.tsv`: one line `0x…<TAB>English<TAB>translation` per slot; take edge markup
   (`\\c`, `\\f1_`, `\n`) from the English column.
2. Add the `frag` lines for months and pattern translators (see above).
3. Create `data/files/<code>.tsv` with the mission and campaign texts (rows `p2m` and `ini`, see above).
4. Add an entry to `LANGUAGES` in `src/language.rs` using `include_str!` for both tables.
5. Run `cargo test`; then set `language=<code>` in `localization.ini` and check in the game.
6. Add `docs/install/<code>.md` (copy an existing guide) and list the language in `README.md`.
