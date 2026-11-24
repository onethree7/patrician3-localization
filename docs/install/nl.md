# Patrician III Localization – Installatie (Nederlands)

Deze handleiding installeert de Nederlandse vertaling van Patrician III (GOG of Steam).

1. Installeer Patrician III via GOG of Steam.
2. Download de [P3Modding-modloader](https://p3modding.github.io/modloader.html) en kopieer `p3_modloader.dll` en `Patrician3_modloader.exe` naar de spelmap (de map met het `.exe`-bestand van het spel).
3. Download op de [releasepagina](../../../../releases) het bestand `patrician3-localization-<versie>.zip` en pak het uit in de spelmap. In de map `mods` staan daarna `patrician3_localization.dll` en `localization.ini`.
4. Open `mods\localization.ini` met Kladblok en zet `language=nl`. Zonder dit bestand gebruikt de mod Duits.
5. Verder hoeft u niets te installeren: missies, de campagnelijst, de tutorial en de aftiteling zijn vertaald; alleen de menuafbeeldingen blijven Engels. Pak de Duitse bronnen (`resources-de`) niet uit.
6. Start het spel met `Patrician3_modloader.exe`.

Optioneel: meer mods uit [`mods.zip` van P3Modding](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) gaan in dezelfde map `mods`.

**Update van de oude Duitse mod:** verwijder eerst `mods\german_localization.dll`. Twee kopieën van de mod komen elkaar in de weg.

## Problemen?

- **Het spel is nog Engels:** staat `localization.ini` in de map `mods` en start u via `Patrician3_modloader.exe`?
- **Een foutmelding bij het starten noemt talen:** de taalcode in `localization.ini` is verkeerd gespeld.
- **Nederlandse menu's maar Duitse missies:** u hebt de Duitse bronnen uitgepakt. Verwijder de mappen `images`, `maps`, `missions`, `missions_addon` en `scripts` die daaruit kwamen, of installeer de spelbestanden opnieuw.

Terug naar het [overzicht](../../README.md)
