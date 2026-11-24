# Patrician III Localization – Installation (Deutsch)

Diese Anleitung installiert die deutsche Übersetzung von Patrician III (GOG oder Steam).

1. Patrician III von GOG oder Steam installieren.
2. Den [P3Modding-Modloader](https://p3modding.github.io/modloader.html) herunterladen und `p3_modloader.dll` sowie `Patrician3_modloader.exe` ins Spielverzeichnis kopieren (der Ordner, in dem die Spiel-`.exe` liegt).
3. Auf der [Releases-Seite](../../../../releases) die Datei `patrician3-localization-<version>.zip` herunterladen und ins Spielverzeichnis entpacken. Im Ordner `mods` liegen danach `patrician3_localization.dll` und `localization.ini`.
4. Standardmäßig ist Deutsch eingestellt. In `mods\localization.ini` muss `language=de` stehen (oder die Datei fehlt).
5. Mehr ist nicht nötig: Missionen, Kampagnenliste, Tutorial, Credits sowie Stadt- und Schiffsnamen übersetzt die Mod selbst, und die ZIP enthält auch die Hauptmenü-Bilder jeder Sprache (`mods\localization\<code>\`).
6. Das Spiel mit `Patrician3_modloader.exe` starten.

Optional: Weitere Mods aus [P3Moddings `mods.zip`](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) kommen in denselben Ordner `mods`.

**Update vom alten Deutschmod:** Zuerst `mods\german_localization.dll` löschen. Zwei Kopien des Mods würden sich gegenseitig stören.

## Probleme?

- **Das Spiel ist englisch:** Liegt `localization.ini` im Ordner `mods`, und startest du über `Patrician3_modloader.exe`?
- **Fehlermeldung beim Start mit einer Liste von Sprachen:** Der Sprachcode in `localization.ini` ist falsch geschrieben.
- **Hauptmenü in einer anderen Sprache oder die Hervorhebung zerstückelt:** Lose Dateien aus der Resources-ZIP früherer Versionen haben Vorrang vor der Mod. Lösche im Spielverzeichnis `images\Options\Hauptmenu.jpg`, `p_hmenu01.bmp`, `hmenu_colormap.bmp`, `scripts\textures.ini` (und `maps\Standard24.gmf`, `missions`, `missions_addon`, falls eine ältere ZIP sie angelegt hat) oder installiere die Spieldateien neu.

Zurück zur [Übersicht](../../README.md)
