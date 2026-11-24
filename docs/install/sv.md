# Patrician III Localization – Installation (Svenska)

Den här guiden installerar den svenska översättningen av Patrician III (GOG eller Steam).

1. Installera Patrician III från GOG eller Steam.
2. Ladda ner [P3Modding-modloadern](https://p3modding.github.io/modloader.html) och kopiera `p3_modloader.dll` och `Patrician3_modloader.exe` till spelmappen (mappen som innehåller spelets `.exe`).
3. Ladda ner `patrician3-localization-<version>.zip` från [releasesidan](../../../../releases) och packa upp den i spelmappen. I mappen `mods` finns sedan `patrician3_localization.dll` och `localization.ini`.
4. Öppna `mods\localization.ini` i Anteckningar och ställ in `language=sv`. Utan filen använder modden tyska.
5. Mer behövs inte: uppdrag, kampanjlistan, handledningen och eftertexterna är översatta, och zip-filen innehåller även huvudmenyns bilder för varje språk (`mods\localization\<code>\`).
6. Starta spelet med `Patrician3_modloader.exe`.

Valfritt: fler moddar från [P3Moddings `mods.zip`](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) läggs i samma `mods`-mapp.

**Uppdatering från den gamla tyska modden:** ta först bort `mods\german_localization.dll`. Två kopior av modden stör varandra.

## Problem?

- **Spelet är fortfarande engelskt:** ligger `localization.ini` i mappen `mods`, och startar du med `Patrician3_modloader.exe`?
- **Ett felmeddelande vid start listar språk:** språkkoden i `localization.ini` är felstavad.
- **Huvudmenyn på ett annat språk eller markeringen sönderklippt:** lösa filer från den tyska resurs-zip-filen i tidigare versioner går före moden. Ta bort `images\Options\Hauptmenu.jpg`, `p_hmenu01.bmp`, `hmenu_colormap.bmp`, `scripts\textures.ini` i spelmappen (och `maps\Standard24.gmf`, `missions` och `missions_addon` om en äldre zip-fil lade till dem), eller installera om spelfilerna.

Tillbaka till [översikten](../../README.md)
