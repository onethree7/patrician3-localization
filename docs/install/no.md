# Patrician III Localization – Installasjon (Norsk)

Denne veiledningen installerer den norske oversettelsen av Patrician III (GOG eller Steam).

1. Installer Patrician III fra GOG eller Steam.
2. Last ned [P3Modding-modloaderen](https://p3modding.github.io/modloader.html) og kopier `p3_modloader.dll` og `Patrician3_modloader.exe` til spillmappen (mappen som inneholder spillets `.exe`).
3. Last ned `patrician3-localization-<versjon>.zip` fra [utgivelsessiden](../../../../releases) og pakk den ut i spillmappen. I mappen `mods` ligger deretter `patrician3_localization.dll` og `localization.ini`.
4. Åpne `mods\localization.ini` i Notisblokk og sett `language=no`. Uten filen bruker modden tysk.
5. Mer trengs ikke: oppdrag, kampanjelisten, opplæringen og rulleteksten er oversatt, og zip-filen inneholder også hovedmenyens bilder for hvert språk (`mods\localization\<code>\`).
6. Start spillet med `Patrician3_modloader.exe`.

Valgfritt: flere moder fra [P3Moddings `mods.zip`](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) legges i samme `mods`-mappe.

**Oppdatering fra den gamle tyske moden:** slett først `mods\german_localization.dll`. To kopier av moden kommer i veien for hverandre.

## Problemer?

- **Spillet er fortsatt engelsk:** ligger `localization.ini` i mappen `mods`, og starter du med `Patrician3_modloader.exe`?
- **En feilmelding ved oppstart nevner språk:** språkkoden i `localization.ini` er skrevet feil.
- **Hovedmenyen på et annet språk eller markeringen kuttet opp:** løse filer fra den tyske ressurs-zip-filen i tidligere versjoner går foran moden. Slett `images\Options\Hauptmenu.jpg`, `p_hmenu01.bmp`, `hmenu_colormap.bmp`, `scripts\textures.ini` i spillmappen (og `maps\Standard24.gmf`, `missions` og `missions_addon` hvis en eldre zip la dem til), eller installer spillfilene på nytt.

Tilbake til [oversikten](../../README.md)
