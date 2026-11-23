# Patrician III Localization – Installazione (Italiano)

Questa guida installa la traduzione italiana di Patrician III (GOG o Steam).

1. Installa Patrician III da GOG o Steam.
2. Scarica il [modloader di P3Modding](https://p3modding.github.io/modloader.html) e copia `p3_modloader.dll` e `Patrician3_modloader.exe` nella cartella del gioco (quella che contiene il `.exe` del gioco).
3. Dalla [pagina delle release](../../../../releases) scarica `patrician3-localization-<versione>.zip` ed estrailo nella cartella del gioco. Nella cartella `mods` troverai `patrician3_localization.dll` e `localization.ini`.
4. Apri `mods\localization.ini` con il Blocco note e imposta `language=it`. Senza questo file la mod usa il tedesco.
5. Non serve altro: missioni, tutorial, crediti e immagini del menu restano in inglese. Non estrarre le risorse tedesche (`resources-de`).
6. Avvia il gioco con `Patrician3_modloader.exe`.

Facoltativo: altre mod dal [`mods.zip` di P3Modding](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) vanno nella stessa cartella `mods`.

**Aggiornamento dalla vecchia mod tedesca:** elimina prima `mods\german_localization.dll`. Due copie della mod si ostacolerebbero a vicenda.

## Problemi?

- **Il gioco è ancora in inglese:** `localization.ini` è nella cartella `mods` e hai avviato con `Patrician3_modloader.exe`?
- **All'avvio compare un errore con un elenco di lingue:** il codice lingua in `localization.ini` è scritto male.
- **Menu in italiano ma missioni in tedesco:** hai estratto le risorse tedesche. Elimina le cartelle `images`, `maps`, `missions`, `missions_addon` e `scripts` che hanno aggiunto, oppure reinstalla i file del gioco.

Torna alla [panoramica](../../README.md)
