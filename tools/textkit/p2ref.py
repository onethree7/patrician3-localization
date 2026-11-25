#!/usr/bin/env python3
"""Compares the German tables with the German original Patrizier 2 Gold.

Usage: p2ref.py <Patrician3.exe (English GOG)> <Patrizier 2.exe (German)> [repository] [output folder]

Patrician III is the English release of Patrizier II Gold; both executables share the text
pointer tables and the string resources, at different addresses. The script

1. collects every text slot (pointer in .data/.rdata to a text) of both executables,
2. aligns the slots: anchors are slots whose German text in data/german_localization.tsv occurs
   exactly once in Patrizier 2, plus longer texts that are identical in both executables;
   between two anchors the slots are matched by placeholder signature and identical texts,
3. reads the string resources of both executables by resource ID.

Output (default target/p2ref/, not part of the repository because it holds the original texts):
  slots.tsv      slot, match quality, English, German table, Patrizier 2
  missing.tsv    slots without a German table row whose Patrizier 2 text differs from English
  resources.tsv  resource ID, English, Patrizier 2
The alignment between anchors is a heuristic; every difference needs a look at the context.
"""
import bisect
import difflib
import os
import re
import struct
import sys
from collections import defaultdict


class Image:
    def __init__(self, path):
        self.data = open(path, "rb").read()
        pe = struct.unpack_from("<I", self.data, 0x3C)[0]
        count = struct.unpack_from("<H", self.data, pe + 6)[0]
        optional = struct.unpack_from("<H", self.data, pe + 20)[0]
        self.base = struct.unpack_from("<I", self.data, pe + 24 + 28)[0]
        self.resource_rva = struct.unpack_from("<I", self.data, pe + 24 + 96 + 2 * 8)[0]
        self.sections = []
        for index in range(count):
            header = pe + 24 + optional + 40 * index
            name = self.data[header:header + 8].rstrip(b"\0")
            vsize, rva, rsize, raw = struct.unpack_from("<IIII", self.data, header + 8)
            self.sections.append((name, rva, max(vsize, rsize), raw))

    def offset(self, rva):
        for _, start, size, raw in self.sections:
            if start <= rva < start + size:
                return rva - start + raw
        return None

    def text(self, rva):
        offset = self.offset(rva)
        if offset is None:
            return None
        end = self.data.find(b"\0", offset)
        if end < 0 or end - offset > 4000:
            return None
        return self.data[offset:end]

    def slots(self):
        """Pointer slots in .data/.rdata whose target is a printable text."""
        found = {}
        for name, start, size, _ in self.sections:
            if name not in (b".data", b".rdata"):
                continue
            for rva in range(start, start + size - 3, 4):
                offset = self.offset(rva)
                if offset + 4 > len(self.data):
                    break
                target = struct.unpack_from("<I", self.data, offset)[0] - self.base
                if target == rva or not 0x278000 <= target < 0x306000:
                    continue
                text = self.text(target)
                if text and all(byte >= 0x20 or byte in (9, 10, 13) for byte in text):
                    found[rva] = text
        return found

    def strings(self):
        """String table resources (type 6) by ID."""
        root = self.offset(self.resource_rva)
        result = {}

        def entries(directory):
            named, numbered = struct.unpack_from("<HH", self.data, directory + 12)
            for index in range(named + numbered):
                key, value = struct.unpack_from("<II", self.data, directory + 16 + 8 * index)
                yield key, value

        for kind, kind_dir in entries(root):
            if kind != 6:
                continue
            for block, block_dir in entries(root + (kind_dir & 0x7FFFFFFF)):
                for _, leaf in entries(root + (block_dir & 0x7FFFFFFF)):
                    rva, size = struct.unpack_from("<II", self.data, root + leaf)
                    data = self.data[self.offset(rva):self.offset(rva) + size]
                    position = 0
                    for index in range(16):
                        length = struct.unpack_from("<H", data, position)[0]
                        position += 2
                        value = data[position:position + 2 * length].decode("utf-16-le")
                        position += 2 * length
                        if value:
                            result[(block - 1) * 16 + index] = value
        return result


def decode(value):
    """Same escapes as decode_table_text in src/lib.rs."""
    escapes = {"\\\\": "\\", "\\n": "\n"}
    return re.sub(r"\\\\|\\n|\\x0[dD]", lambda m: escapes.get(m.group(0), "\r"), value)


def encode(value):
    return value.replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\x0D")


def table(path):
    rows = []
    for line in open(path, encoding="utf-8-sig"):
        line = line.rstrip("\r\n")
        if line and not line.startswith("#"):
            slot, source, english, german = line.split("\t")
            rows.append((int(slot, 16), int(source, 16), english, german))
    return rows


def signature(text):
    return (tuple(re.findall(rb"%[-$0-9.]*[a-zA-Z]", text)),
            text[:3] if text.startswith(b"\\") else b"", text.count(b"\n"))


def align(english, german, rows):
    by_text = defaultdict(list)
    for slot, text in german.items():
        by_text[text].append(slot)
    anchors = {}
    for slot, _, _, translation in rows:
        try:
            candidates = by_text.get(decode(translation).encode("cp1252"), [])
        except UnicodeEncodeError:
            continue
        if len(candidates) == 1:
            anchors[slot] = candidates[0]
    used = set(anchors.values())
    for slot, text in english.items():
        candidates = by_text.get(text, [])
        if slot not in anchors and len(text) > 6 and len(candidates) == 1 and candidates[0] not in used:
            anchors[slot] = candidates[0]
    # Keep the longest chain of anchors that is increasing on both sides.
    items = sorted(anchors.items())
    tails, tail_index, previous = [], [], [None] * len(items)
    for index, (_, target) in enumerate(items):
        position = bisect.bisect_left(tails, target)
        if position == len(tails):
            tails.append(target)
            tail_index.append(index)
        else:
            tails[position] = target
            tail_index[position] = index
        previous[index] = tail_index[position - 1] if position else None
    chain, index = [], tail_index[-1]
    while index is not None:
        chain.append(items[index])
        index = previous[index]
    anchors = dict(reversed(chain))

    mapping = {slot: (target, "anchor") for slot, target in anchors.items()}
    english_slots, german_slots, keys = sorted(english), sorted(german), sorted(anchors)
    for low, high in zip(keys, keys[1:]):
        left = english_slots[bisect.bisect_right(english_slots, low):bisect.bisect_left(english_slots, high)]
        right = german_slots[bisect.bisect_right(german_slots, anchors[low]):
                             bisect.bisect_left(german_slots, anchors[high])]
        if not left or not right:
            continue
        if high - low == anchors[high] - anchors[low]:
            for slot in left:
                if slot + anchors[low] - low in german:
                    mapping[slot] = (slot + anchors[low] - low, "fill")
            continue
        common = {english[s] for s in left} & {german[s] for s in right}
        a = [english[s] if english[s] in common else signature(english[s]) for s in left]
        b = [german[s] if german[s] in common else signature(german[s]) for s in right]
        for block in difflib.SequenceMatcher(None, a, b, autojunk=False).get_matching_blocks():
            for step in range(block.size):
                mapping[left[block.a + step]] = (right[block.b + step], "fill")
    return mapping


def printable(text):
    return re.fullmatch(rb"[\x20-\x7e\xa0-\xff\n\r\t]+", text) and re.search(rb"[A-Za-z]{3}", text)


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    english_image, german_image = Image(sys.argv[1]), Image(sys.argv[2])
    repository = sys.argv[3] if len(sys.argv) > 3 else os.path.join(os.path.dirname(__file__), "..", "..")
    output = sys.argv[4] if len(sys.argv) > 4 else os.path.join(repository, "target", "p2ref")
    os.makedirs(output, exist_ok=True)
    rows = table(os.path.join(repository, "data", "german_localization.tsv"))
    known = {row[0] for row in rows}
    for line in open(os.path.join(repository, "data", "extra_slots.tsv"), encoding="utf-8-sig"):
        if line.startswith("0x"):
            known.add(int(line.split("\t")[0], 16))
    english, german = english_image.slots(), german_image.slots()
    mapping = align(english, german, rows)

    def original(slot):
        target, quality = mapping.get(slot, (None, "none"))
        return (german[target].decode("cp1252", "replace") if target else ""), quality

    differ = 0
    with open(os.path.join(output, "slots.tsv"), "w", encoding="utf-8", newline="\n") as out:
        out.write("# slot\tmatch\tEnglish\tGerman table\tPatrizier 2\n")
        for slot, _, source, translation in rows:
            text, quality = original(slot)
            if text and text != decode(translation):
                differ += 1
            out.write(f"0x{slot:X}\t{quality}\t{source}\t{translation}\t{encode(text)}\n")
    missing = 0
    translated = {row[0] for row in rows}
    with open(os.path.join(output, "missing.tsv"), "w", encoding="utf-8", newline="\n") as out:
        out.write("# slot\tmatch\tEnglish\tPatrizier 2 (slots without German table row)\n")
        for slot, text in sorted(english.items()):
            if slot in translated or not printable(text):
                continue
            target, quality = mapping.get(slot, (None, None))
            if target and german[target] != text and printable(german[target]):
                missing += 1
                out.write(f"0x{slot:X}\t{quality}{'/extra' if slot in known else ''}\t"
                          f"{encode(text.decode('cp1252', 'replace'))}\t"
                          f"{encode(german[target].decode('cp1252', 'replace'))}\n")
    resources_en, resources_de = english_image.strings(), german_image.strings()
    with open(os.path.join(output, "resources.tsv"), "w", encoding="utf-8", newline="\n") as out:
        out.write("# resource\tEnglish\tPatrizier 2\n")
        for key in sorted(set(resources_en) | set(resources_de)):
            out.write(f"{key}\t{encode(resources_en.get(key, ''))}\t{encode(resources_de.get(key, ''))}\n")
    print(f"{len(rows)} table rows, {sum(1 for r in rows if r[0] in mapping)} aligned, "
          f"{differ} differ from Patrizier 2; {missing} slots without row; "
          f"{len(resources_de)} resources -> {output}")


if __name__ == "__main__":
    main()
