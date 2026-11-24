# Patrician III Localization – Installasjon (Norsk)

Denne veiledningen installerer den norske oversettelsen av Patrician III (GOG eller Steam).

1. Installer Patrician III fra GOG eller Steam.
2. Last ned [P3Modding-modloaderen](https://p3modding.github.io/modloader.html) og kopier `p3_modloader.dll` og `Patrician3_modloader.exe` til spillmappen (mappen som inneholder spillets `.exe`).
3. Last ned `patrician3-localization-<versjon>.zip` fra [utgivelsessiden](../../../../releases) og pakk den ut i spillmappen. I mappen `mods` ligger deretter `patrician3_localization.dll` og `localization.ini`.
4. Åpne `mods\localization.ini` i Notisblokk og sett `language=no`. Uten filen bruker modden tysk.
5. Mer trengs ikke: oppdrag og kampanjelisten er oversatt; opplæring, rulletekst og menybilder forblir engelske. Ikke pakk ut de tyske ressursene (`resources-de`).
6. Start spillet med `Patrician3_modloader.exe`.

Valgfritt: flere moder fra [P3Moddings `mods.zip`](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) legges i samme `mods`-mappe.

**Oppdatering fra den gamle tyske moden:** slett først `mods\german_localization.dll`. To kopier av moden kommer i veien for hverandre.

## Problemer?

- **Spillet er fortsatt engelsk:** ligger `localization.ini` i mappen `mods`, og starter du med `Patrician3_modloader.exe`?
- **En feilmelding ved oppstart nevner språk:** språkkoden i `localization.ini` er skrevet feil.
- **Norske menyer, men tyske oppdrag:** du har pakket ut de tyske ressursene. Slett mappene `images`, `maps`, `missions`, `missions_addon` og `scripts` som de la til, eller installer spillfilene på nytt.

Tilbake til [oversikten](../../README.md)
