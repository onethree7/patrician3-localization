"""Game files with texts (mirror of FILES in src/files/mod.rs) and how to extract them."""
import re

# pattern -> (format, ini keys that hold texts)
FILES = {
    "*.p2m": ("mission", None),
    "*.gmf": ("map", None),
    "scripts/missions_eng.ini": ("ini", re.compile(r"(Name|Text|QuestName|QuestText)(_?\d+)?")),
    "scripts/tutorial_eng.ini": ("ini", re.compile(r"WindowText\d+")),
    "scripts/credits_eng.ini": ("ini", re.compile(r"Text_\d+")),
    "scripts/stadtauswahl.ini": ("ini", re.compile(r"Text")),
    "scripts/namenstaedte_eng.txt": ("lines", None),
    "scripts/namenschiffe_eng.txt": ("names", None),
    "scripts/namenmaenner.txt": ("names", None),
    "scripts/namenfrauen.txt": ("names", None),
    "scripts/namenfamilien.txt": ("names", None),
}

# Game font: ASCII plus these letters.
FONT_EXTRA = set("äöüÄÖÜßáàâéèêíìîóòôúùû")


def is_name_byte(byte: int) -> bool:
    """The name generator splits names at every byte below 'A' except space and '.'."""
    return byte >= 0x41 or byte in b" ."
