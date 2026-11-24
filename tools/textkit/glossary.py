#!/usr/bin/env python3
"""Town names per language (data/glossary/towns.tsv).

Usage: glossary.py <language>...   rewrites data/files/<language>.tsv so that every text
uses the language's town names, and writes the map rows (*.gmf) and the town lines of
scripts/namenstaedte_eng.txt. Ship and person names are left alone.
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
from files import FILES  # noqa: E402

REPO = os.path.join(os.path.dirname(__file__), "..", "..")
NAME_FILES = {p for p, (fmt, _) in FILES.items() if fmt == "names"}
TOWN_LIST = "scripts/namenstaedte_eng.txt"
HEADER = "# Town names (data/glossary/towns.tsv, tools/textkit/glossary.py)"


def load():
    rows = [l.rstrip("\n").split("\t") for l in open(os.path.join(REPO, "data/glossary/towns.tsv"), encoding="utf-8")
            if not l.startswith("#")]
    header, rows = rows[0], rows[1:]
    towns = []
    for row in rows:
        english, aliases = row[0], [a for a in row[1].split(",") if a]
        target = {lang: (row[i] or english) for i, lang in enumerate(header) if i >= 2}
        towns.append((english, aliases, target))
    return towns


def replacer(towns, language):
    pairs = []
    for english, aliases, target in towns:
        for variant in [english] + aliases:
            if variant != target[language]:
                pairs.append((variant, target[language]))
    pairs.sort(key=lambda p: -len(p[0]))
    pattern = re.compile(r"\b(" + "|".join(re.escape(v) for v, _ in pairs) + r")(?=[a-zäöüß]*\b)")
    table = dict(pairs)

    def substitute(match):
        variant = match.group(1)
        # "Hamburg" inside "Hamburgo" is already the target.
        if match.string.startswith(table[variant], match.start()):
            return variant
        return table[variant]

    return lambda text: pattern.sub(substitute, text)


def main():
    towns = load()
    source = {}
    for line in open(os.path.join(REPO, "data/files/source.tsv"), encoding="utf-8"):
        if not line.startswith("#"):
            scope, text = line.rstrip("\n").split("\t", 1)
            source.setdefault(scope, []).append(text)
    for language in sys.argv[1:]:
        path = os.path.join(REPO, f"data/files/{language}.tsv")
        replace = replacer(towns, language)
        lines = open(path, encoding="utf-8").read().splitlines()
        out = []
        for line in lines:
            if line == HEADER:
                continue
            if not line or line.startswith("#"):
                out.append(line)
                continue
            scope, english, translation = line.split("\t")
            if scope == "*.gmf" or (scope == TOWN_LIST and english in source[TOWN_LIST][:40]):
                continue  # rewritten below
            if scope not in NAME_FILES and english != "*":
                text = english if translation == "=" else translation
                new = replace(text)
                translation = "=" if new == english else new
            out.append(f"{scope}\t{english}\t{translation}")
        while out and not out[-1]:
            out.pop()
        out.append(HEADER)
        by_name = {}
        for english, aliases, target in towns:
            for variant in [english] + aliases:
                by_name[variant] = target[language]
        for name in source["*.gmf"]:
            out.append(f"*.gmf\t{name}\t{'=' if by_name[name] == name else by_name[name]}")
        for name in source[TOWN_LIST][:40]:
            out.append(f"{TOWN_LIST}\t{name}\t{'=' if by_name[name] == name else by_name[name]}")
        open(path, "w", encoding="utf-8", newline="\n").write("\n".join(out) + "\n")
        print(language, "ok")


if __name__ == "__main__":
    main()
