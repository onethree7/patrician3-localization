"""Table cell codec for mission texts (mirror of src/files/mission.rs).

`\\n` = CR LF, `\\t`, `\\\\`, `\\xHH`, `{%X:HH}` = format token `%X` with its argument byte.
"""
import re

TOKEN = re.compile(r"\{%(.):([0-9A-F]{2})\}")


def encode(raw: bytes) -> str:
    """Raw CP1252 mission bytes -> table cell."""
    out, i = [], 0
    while i < len(raw):
        c = raw[i]
        if c == 0x25 and i + 2 < len(raw):
            out.append("{%%%s:%02X}" % (chr(raw[i + 1]), raw[i + 2]))
            i += 3
            continue
        if c == 0x0D and raw[i + 1 : i + 2] == b"\n":
            out.append("\\n")
            i += 2
            continue
        if c == 0x5C:
            out.append("\\\\")
        elif c == 0x09:
            out.append("\\t")
        elif c < 0x20:
            out.append("\\x%02X" % c)
        else:
            out.append(bytes([c]).decode("cp1252"))
        i += 1
    return "".join(out)


def decode(cell: str) -> bytes:
    """Table cell -> raw CP1252 mission bytes."""
    out, i = bytearray(), 0
    while i < len(cell):
        match = TOKEN.match(cell, i)
        if match:
            out += b"%" + match.group(1).encode("cp1252") + bytes([int(match.group(2), 16)])
            i = match.end()
            continue
        c = cell[i]
        if c == "\\":
            n = cell[i + 1]
            if n in "nt\\":
                out += {"n": b"\r\n", "t": b"\t", "\\": b"\\"}[n]
                i += 2
                continue
            if n == "x":
                out.append(int(cell[i + 2 : i + 4], 16))
                i += 4
                continue
            raise ValueError(cell)
        out += c.encode("cp1252")
        i += 1
    return bytes(out)


def tokens(cell: str) -> list:
    return sorted(TOKEN.findall(cell))
