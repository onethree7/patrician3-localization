"""Mission script reader (mirror of the layout documented in src/files/mission.rs)."""
import struct


def strings(data: bytes) -> list:
    """All texts of a mission, in file order; [] if the file is malformed."""
    count = struct.unpack_from("<H", data, 0)[0]
    if count == 0:
        return []
    table = 4 + 4 * count if data[4 : 4 + 4 * count] == b"\xcd" * (4 * count) else 4
    offsets = list(struct.unpack_from("<%dI" % count, data, table))
    if offsets[0] != table + 4 * count or any(b <= a for a, b in zip(offsets, offsets[1:])):
        return []
    ends = offsets[1:] + [len(data)]
    statements = [data[a:b] for a, b in zip(offsets, ends)]
    targets = set()
    for s in statements:
        if s[0] == 0x37 and len(s) == 9:
            targets.update(struct.unpack_from("<2I", s, 1))
        elif s[0] == 0xF7 and len(s) >= 5:
            targets.add(struct.unpack_from("<I", s, len(s) - 4)[0])
    out = []
    for offset, s in zip(offsets, statements):
        if s[0] == 0xFE and len(s) >= 2:
            p = 2
        elif s[0] == 0xFF and offset + 1 in targets:
            p = 1
        elif s[0] == 0x1D and len(s) > 8 and b"\0" in s[3:]:
            p = 3
        else:
            continue
        while p < len(s):
            q = p
            while s[q] != 0:
                q += 3 if s[q] == 0x25 else 1
            out.append(s[p:q])
            p = q + 1
    return out
