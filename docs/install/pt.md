# Patrician III Localization – Instalação (Português)

Este guia instala a tradução portuguesa de Patrician III (GOG ou Steam).

1. Instala o Patrician III a partir do GOG ou do Steam.
2. Descarrega o [modloader P3Modding](https://p3modding.github.io/modloader.html) e copia `p3_modloader.dll` e `Patrician3_modloader.exe` para a pasta do jogo (a pasta que contém o `.exe` do jogo).
3. Na [página de versões](../../../../releases) descarrega `patrician3-localization-<versão>.zip` e extrai-o para a pasta do jogo. Na pasta `mods` ficam `patrician3_localization.dll` e `localization.ini`.
4. Abre `mods\localization.ini` com o Bloco de Notas e define `language=pt`. Sem este ficheiro, o mod usa alemão.
5. Não é preciso mais nada: as missões, o tutorial, os créditos e as imagens do menu continuam em inglês. Não extraias os recursos alemães (`resources-de`).
6. Inicia o jogo com `Patrician3_modloader.exe`.

Opcional: outros mods do [`mods.zip` do P3Modding](https://github.com/P3Modding/p3-lib/releases/tag/0.1.15) vão para a mesma pasta `mods`.

**Atualização a partir do antigo mod alemão:** apaga primeiro `mods\german_localization.dll`. Duas cópias do mod interferem uma com a outra.

## Problemas?

- **O jogo continua em inglês:** o `localization.ini` está na pasta `mods` e iniciaste com `Patrician3_modloader.exe`?
- **Ao iniciar aparece um erro com uma lista de idiomas:** o código de idioma em `localization.ini` está mal escrito.
- **Menus em português mas missões em alemão:** extraíste os recursos alemães. Apaga as pastas `images`, `maps`, `missions`, `missions_addon` e `scripts` que eles adicionaram, ou reinstala os ficheiros do jogo.

Voltar à [vista geral](../../README.md)
