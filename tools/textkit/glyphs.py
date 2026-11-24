"""Replaces characters the game font cannot draw (see docs/LANGUAGES.md#character-set)."""
import unicodedata

from files import FONT_EXTRA

COMMON = {"«": '"', "»": '"', "“": '"', "”": '"', "„": '"', "‘": "'", "’": "'",
          "–": "-", "—": "-", "…": "...", "º": "o", "ª": "a", "°": "o"}
PER_LANGUAGE = {
    "fr": {"ç": "c", "ï": "i", "œ": "oe", "ë": "e", "ÿ": "y"},
    "nl": {"ë": "e", "ï": "i"},
    "sv": {"å": "a", "Å": "A"},
    "da": {"å": "aa", "æ": "ae", "ø": "oe", "Å": "Aa", "Æ": "Ae", "Ø": "Oe"},
    "no": {"å": "aa", "æ": "ae", "ø": "oe", "Å": "Aa", "Æ": "Ae", "Ø": "Oe"},
    "es": {"ñ": "ni", "Ñ": "Ni", "¡": "", "¿": ""},
    "pt": {"ã": "a", "õ": "o", "ç": "c", "Ã": "A", "Õ": "O", "Ç": "C"},
    "it": {"È": "E'"},
}


def fix(language: str, text: str) -> str:
    if language == "de":
        return text
    for a, b in {**COMMON, **PER_LANGUAGE.get(language, {})}.items():
        text = text.replace(a, b)
    out = []
    for c in text:
        if c.isascii() or c in FONT_EXTRA:
            out.append(c)
        else:
            base = unicodedata.normalize("NFD", c)[0]
            out.append(base if base.isascii() else c)
    return "".join(out)
