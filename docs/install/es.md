# Patrician III Localization – Instalación (Español)

Esta guía instala la traducción al español de Patrician III (GOG o Steam).

1. Instala Patrician III desde GOG o Steam.
2. Descarga el [cargador de mods P3Modding](https://p3modding.github.io/modloader.html) y copia `p3_modloader.dll` y `Patrician3_modloader.exe` en la carpeta del juego (la que contiene el `.exe` del juego).
3. En la [página de versiones](../../../../releases) descarga `patrician3-localization-<versión>.zip` y descomprímelo en la carpeta del juego. En la carpeta `mods` aparecerán `patrician3_localization.dll` y `localization.ini`.
4. Abre `mods\localization.ini` con el Bloc de notas y escribe `language=es`. Sin este archivo, el mod usa alemán.
5. No hace falta nada más: las misiones, el tutorial, los créditos y las imágenes del menú siguen en inglés. No descomprimas los recursos alemanes (`resources-de`).
6. Inicia el juego con `Patrician3_modloader.exe`.

Opcional: otros mods del [`mods.zip` de P3Modding](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) van en la misma carpeta `mods`.

**Actualización desde el antiguo mod alemán:** borra primero `mods\german_localization.dll`. Dos copias del mod se estorbarían entre sí.

## ¿Problemas?

- **El juego sigue en inglés:** ¿está `localization.ini` en la carpeta `mods` y has iniciado con `Patrician3_modloader.exe`?
- **Al iniciar aparece un error con una lista de idiomas:** el código de idioma en `localization.ini` está mal escrito.
- **Menús en español pero misiones en alemán:** has descomprimido los recursos alemanes. Borra las carpetas `images`, `maps`, `missions`, `missions_addon` y `scripts` que añadieron, o reinstala los archivos del juego.

Volver a la [vista general](../../README.md)
