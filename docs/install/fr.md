# Patrician III Localization – Installation (Français)

Ce guide installe la traduction française de Patrician III (GOG ou Steam).

1. Installez Patrician III depuis GOG ou Steam.
2. Téléchargez le [chargeur de mods P3Modding](https://p3modding.github.io/modloader.html) et copiez `p3_modloader.dll` et `Patrician3_modloader.exe` dans le dossier du jeu (celui qui contient le `.exe` du jeu).
3. Sur la [page des versions](../../../../releases), téléchargez `patrician3-localization-<version>.zip` et extrayez-le dans le dossier du jeu. Le dossier `mods` contient alors `patrician3_localization.dll` et `localization.ini`.
4. Ouvrez `mods\localization.ini` avec le Bloc-notes et réglez `language=fr`. Sans ce fichier, le mod utilise l'allemand.
5. Rien d'autre à installer : les missions, la liste des campagnes, le tutoriel et les crédits sont traduits ; seules les images du menu restent en anglais. Ne copiez pas les ressources allemandes (`resources-de`).
6. Lancez le jeu avec `Patrician3_modloader.exe`.

Facultatif : d'autres mods de [`mods.zip` de P3Modding](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) vont dans le même dossier `mods`.

**Mise à jour depuis l'ancien mod allemand :** supprimez d'abord `mods\german_localization.dll`. Deux copies du mod se gêneraient mutuellement.

## Un problème ?

- **Le jeu est resté en anglais :** `localization.ini` est-il dans le dossier `mods`, et avez-vous lancé `Patrician3_modloader.exe` ?
- **Un message d'erreur au démarrage énumère des langues :** le code de langue dans `localization.ini` est mal écrit.
- **Menus en français mais missions en allemand :** vous avez extrait les ressources allemandes. Supprimez les dossiers `images`, `maps`, `missions`, `missions_addon` et `scripts` qu'elles ont ajoutés, ou réinstallez les fichiers du jeu.

Retour à la [vue d'ensemble](../../README.md)
