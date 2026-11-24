# Patrician III Localization

Play **Patrician III** (GOG and Steam) in your own language. This mod replaces the English game texts with translations: menus, trade goods, letters, chronicle and event messages, guild and Hanseatic League dialogues, missions and the campaign list. It is loaded by the [P3Modding modloader](https://p3modding.github.io/modloader.html) and also translates selected texts of the official mods.

**Version 0.2.1** · Tested with GOG, Steam and P3Modding 0.1.15

## Available languages

| Language | Code | Game texts (menus, goods, events, letters) | Missions and campaigns | Tutorial, credits, menu gfx |
|---|---|---|---|---|
| Deutsch | `de` | complete | translated | translated (separate download, see below) |
| Français | `fr` | complete, not proofread | translated, not proofread | English |
| Nederlands | `nl` | complete, not proofread | translated, not proofread | English |
| Español | `es` | complete, not proofread | translated, not proofread | English |
| Italiano | `it` | complete, not proofread | translated, not proofread | English |
| Svenska | `sv` | complete, not proofread | translated, not proofread | English |
| Dansk | `da` | complete, not proofread | translated, not proofread | English |
| Norsk (Bokmål) | `no` | complete, not proofread | translated, not proofread | English |
| Português | `pt` | complete, not proofread | translated, not proofread | English |

"Not proofread" means: every text is translated, but no native speaker has reviewed it yet. Some sentences, which the game assembles from several pieces, can read a little awkwardly. Corrections are very welcome, see [docs/LANGUAGES.md](docs/LANGUAGES.md).

> **Known limitation - missing font characters.** The game font has no glyphs for `å æ ø ñ ç ã õ ë ï œ`, `¡ ¿ « »` and capital accented letters, so these are replaced by plain letters in the translations (for example Swedish `Rådhus` is shown as `Radhus`, Danish/Norwegian `å` as `aa`). Proper characters need a patched game font. Details and the full substitution table: [docs/LANGUAGES.md](docs/LANGUAGES.md#character-set).

## Installation

You need two things besides this mod: the game itself and the modloader.

1. Install Patrician III from GOG or Steam.
2. Download the [P3Modding modloader](https://p3modding.github.io/modloader.html) and copy `p3_modloader.dll` and `Patrician3_modloader.exe` into the game folder (the folder that contains the game's `.exe`).
3. From the [Releases](../../releases) page, download **`patrician3-localization-<version>.zip`** and extract it into the game folder. This creates two files in the `mods` folder: `patrician3_localization.dll` and `localization.ini`.
4. Open `mods\localization.ini` with Notepad and set your language code, for example `language=fr` for French. Without this file the mod uses German.
5. **Only for German:** also download **`patrician3-localization-<version>-resources-de.zip`** and extract it into the game folder. It adds the German tutorial, credits, name lists, map, menu pictures and a few missions. The other missions are translated by the mod itself.
6. Start the game with `Patrician3_modloader.exe`.

Optional: further mods from [P3Moddings `mods.zip`](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) go into the same `mods` folder. Their texts are translated as well, as far as the mod knows them.

### Step-by-step guide in your language

[Deutsch](docs/install/de.md) · [Français](docs/install/fr.md) · [Nederlands](docs/install/nl.md) · [Español](docs/install/es.md) · [Italiano](docs/install/it.md) · [Svenska](docs/install/sv.md) · [Dansk](docs/install/da.md) · [Norsk](docs/install/no.md) · [Português](docs/install/pt.md)

### Updating from the old German mod

Earlier versions were called "Deutschmod" and shipped as `german_localization.dll`. **Delete `mods\german_localization.dll` first.** Two copies of the mod would patch the same texts and conflict. Then install as described above.

### Something went wrong?

- **The game is still English.** Check that `localization.ini` is in the `mods` folder and that you started the game with `Patrician3_modloader.exe`.
- **An error message appears at start and lists the available languages.** The language code in `localization.ini` is misspelled. Use one of: `de`, `fr`, `nl`, `es`, `it`, `sv`, `da`, `no`, `pt`.
- **French (or another language) menus, but German tutorial, credits or missions.** You extracted the German resources. They are meant for German only. Delete the files they added (the `images`, `maps`, `missions`, `missions_addon` and `scripts` folders from the zip) or reinstall the game files.

## Screenshots

![Main menu, game settings and market hall in German](docs/screenshots/uebersicht.jpg)

## How it works

The English game executable was analysed with REA and Ghidra to find texts, text pointers and how they are used. For German wording, the original texts of **Patrizier II Gold** served as one reference. The assignments were checked in their context; individual texts were adapted.

At startup the mod checks that the expected text addresses in the game match, then replaces the text pointers in memory. This keeps different translations of the same English word in their respective places. One text hook translates composed texts and labels of other mods. Guild and event messages are built from translated sentence parts such as articles, adjectives and ranks. A second hook takes the longer text of some languages into account for tooltips. A third hook reads missions and the campaign list while the game loads them from its archive: it takes the English original, swaps its texts for the configured language and hands the game the result; files that do not match the English originals stay untouched. All translation tables are embedded into the DLL.

## For developers

Rust and the Windows target `i686-pc-windows-msvc` are required. In the project folder:

```powershell
rustup target add i686-pc-windows-msvc
cargo build --locked --release --target i686-pc-windows-msvc
```

The DLL is created at `target/i686-pc-windows-msvc/release/patrician3_localization.dll` and belongs in the game at `mods/patrician3_localization.dll`.

`powershell -File tools\package.ps1` builds both release archives into `dist\`.

```text
patrician3-localization/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE                 Apache License 2.0
├── src/                    mod code and tests
├── data/                   translation tables, embedded into the DLL at build time
│   ├── german_localization.tsv
│   ├── render_translations.tsv
│   ├── extra_slots.tsv
│   ├── files/              mission and campaign texts, one table per language
│   └── lang/               one table per additional language
├── resources/
│   └── de/                 German game files (tutorial, credits, names, map, menu pictures, a few missions);
│                           the folders match the paths in the game folder
├── tools/                  packaging script, sample localization.ini
└── docs/                   language documentation, install guides, screenshots
```

Checks:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --target i686-pc-windows-msvc --all-targets -- -D warnings
cargo test --locked --target i686-pc-windows-msvc
```

## License

The mod code and our own contributions to the translation tables are licensed under the [Apache License 2.0](LICENSE). Game texts and the files under `resources/` are not covered by it; rights of third parties remain unaffected.

## Credits and origin

Thanks to **P3Modding** for the [modloader, tools and documentation](https://github.com/P3Modding/p3-lib) this mod builds on.

Special thanks to [**@Trolldemorted**](https://github.com/Trolldemorted) (Benedikt Radtke) for his outstanding reverse-engineering work.
