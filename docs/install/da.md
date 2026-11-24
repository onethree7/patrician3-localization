# Patrician III Localization – Installation (Dansk)

Denne vejledning installerer den danske oversættelse af Patrician III (GOG eller Steam).

1. Installer Patrician III fra GOG eller Steam.
2. Download [P3Modding-modloaderen](https://p3modding.github.io/modloader.html) og kopier `p3_modloader.dll` og `Patrician3_modloader.exe` til spilmappen (mappen med spillets `.exe`).
3. Download `patrician3-localization-<version>.zip` fra [udgivelsessiden](../../../../releases) og udpak den i spilmappen. I mappen `mods` ligger derefter `patrician3_localization.dll` og `localization.ini`.
4. Åbn `mods\localization.ini` i Notesblok og sæt `language=da`. Uden filen bruger modden tysk.
5. Mere skal der ikke til: missioner og kampagnelisten er oversat; tutorial, rulletekster og menubilleder forbliver engelske. Udpak ikke de tyske ressourcer (`resources-de`).
6. Start spillet med `Patrician3_modloader.exe`.

Valgfrit: flere mods fra [P3Moddings `mods.zip`](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) lægges i samme `mods`-mappe.

**Opdatering fra den gamle tyske mod:** slet først `mods\german_localization.dll`. To kopier af modden kommer i vejen for hinanden.

## Problemer?

- **Spillet er stadig engelsk:** ligger `localization.ini` i mappen `mods`, og starter du med `Patrician3_modloader.exe`?
- **En fejlmeddelelse ved start nævner sprog:** sprogkoden i `localization.ini` er stavet forkert.
- **Danske menuer men tyske missioner:** du har udpakket de tyske ressourcer. Slet mapperne `images`, `maps`, `missions`, `missions_addon` og `scripts`, som de tilføjede, eller geninstaller spilfilerne.

Tilbage til [oversigten](../../README.md)
