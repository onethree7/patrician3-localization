"""Map files (*.gmf): decoder and town names (mirror of src/files/map.rs)."""
import struct

TOWN_NAMES = 40


def decompress(data: bytes) -> bytes:
    size = struct.unpack_from("<i", data, 0)[0]
    if size <= 0:
        return data[4 : 4 - size]
    pos, buf, cnt = 4, 0, 0

    def bits(k):
        nonlocal pos, buf, cnt
        while cnt < k:
            buf |= struct.unpack_from("<I", data, pos)[0] << cnt
            pos += 4
            cnt += 32
        value = buf & ((1 << k) - 1)
        buf >>= k
        cnt -= k
        return value

    out = bytearray()
    while len(out) < size:
        if not bits(1):
            out.append(bits(8))
            continue
        cls = bits(3)
        dist = bits(cls + 1) + (1 << (cls + 1)) - 1
        length, width = 2, 2
        while True:
            chunk = bits(width)
            length += chunk
            if chunk != (1 << width) - 1:
                break
            width += 1
        for _ in range(length):
            out.append(out[-dist])
    return bytes(out)


def town_names(data: bytes) -> list:
    raw = decompress(data)
    towns = struct.unpack_from("<I", raw, 0)[0]
    at = 4
    if raw[4:6] == b"\xfe\xaf":
        at += 4 + struct.unpack_from("<H", raw, 6)[0]
    at += 11 * towns
    return raw[at:].split(b"\0")[:TOWN_NAMES]
