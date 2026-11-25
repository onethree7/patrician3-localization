#!/usr/bin/env python3
"""Builds the main menu pictures of each language from the English originals.

The English background keeps everything but the eight menu words: the words are
taken out, the page is rebuilt from the paper around them, and the translations are
set in Goudy Old Style Bold along the measured lines of the book. The initials are
set into one clean box (tools/gfx/box.png) in every line. Hover sprites and click
areas follow from the new words.

Usage:
  gen.py --archive <p2arch0_eng.cpr> --font <GOUDOSB.TTF> [--out data/gfx] [lang ...]
  gen.py --archive ... --font ... --box-from <folder with the German Hauptmenu.jpg
         and hmenu_colormap.bmp>      rebuilds tools/gfx/box.png first

Writes <out>/<lang>/Hauptmenu.jpg, p_hmenu01.bmp, hmenu_colormap.bmp and textures.tsv
(the sprite rectangles of p_hmenu01.bmp for scripts/textures.ini).
The font is not part of the repository (Monotype license); Windows has it as
C:\\Windows\\Fonts\\GOUDOSB.TTF.
"""
import argparse, io, json, os, re, struct, mmap
import numpy as np, cv2
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
SHEET = 'images/options/p_hmenu01.bmp'
EN = ['Tutorial', 'Campaign', 'Single Game', 'Multiplayer', 'Load Game', 'Options', 'Credits', 'Exit']
WORDS = {
    'en': EN,
    'de': ['Tutorial', 'Kampagne', 'Einzelspiel', 'Mehrspieler', 'Laden', 'Optionen', 'Credits', 'Verlassen'],
    'fr': ['Tutoriel', 'Campagne', 'Partie solo', 'Multijoueur', 'Charger', 'Options', 'Crédits', 'Quitter'],
    'nl': ['Tutorial', 'Campagne', 'Eén speler', 'Multiplayer', 'Laden', 'Opties', 'Credits', 'Afsluiten'],
    'es': ['Tutorial', 'Campaña', 'Un jugador', 'Multijugador', 'Cargar', 'Opciones', 'Créditos', 'Salir'],
    'it': ['Tutorial', 'Campagna', 'Gioco singolo', 'Multigiocatore', 'Carica', 'Opzioni', 'Crediti', 'Esci'],
    'sv': ['Övning', 'Kampanj', 'En spelare', 'Flera spelare', 'Ladda', 'Alternativ', 'Medverkande', 'Avsluta'],
    'da': ['Øvelse', 'Kampagne', 'Enkeltspil', 'Flerspiller', 'Indlæs', 'Indstillinger', 'Medvirkende', 'Afslut'],
    'no': ['Opplæring', 'Kampanje', 'Enkeltspill', 'Flerspiller', 'Last inn', 'Valg', 'Medvirkende', 'Avslutt'],
    'pt': ['Tutorial', 'Campanha', 'Um jogador', 'Multijogador', 'Carregar', 'Opções', 'Créditos', 'Sair'],
}
SIZE, CW, CH, OX, OY = 120, 2200, 240, 60, 170   # template canvas: font size, size, baseline origin
FONT = None
font = None

GAP = 0.12     # space between initial box and word, in box widths (the English art: 0.24)
WEIGHT = 0.5   # 0 = plain Goudy Bold, 1 = one pixel stroke at size 120 (top line)
STROKE = 1


def render(text, x=OX, stroke=None):
    im = Image.new('L', (CW, CH), 0)
    st = STROKE if stroke is None else stroke
    ImageDraw.Draw(im).text((x, OY), text, font=font, fill=255, anchor='ls', stroke_width=st, stroke_fill=255)
    return np.asarray(im, np.float32) / 255

def read_archive(path, names):
    """Files from an Ascaron archive (p2arch0_eng.cpr) by path, case-insensitive."""
    want = {n.lower().replace('/', '\\'): n for n in names}
    out = {}
    with open(path, 'rb') as f:
        m = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
        for x in re.finditer(rb'[A-Za-z0-9_\\/.]+\0', m):
            name = x.group()[:-1].decode('ascii', 'replace')
            key = name.lower().replace('/', '\\')
            if key in want and want[key] not in out and x.start() >= 12:
                off, size, _ = struct.unpack_from('<3I', m, x.start() - 12)
                if 0 < size and off + size <= len(m):
                    out[want[key]] = bytes(m[off:off + size])
    missing = set(names) - set(out)
    if missing:
        raise SystemExit(f'not in the archive: {sorted(missing)}')
    return out


ENGLISH = None


def load():
    f = ENGLISH
    B = np.asarray(Image.open(io.BytesIO(f['images/options/Hauptmenu.jpg'])).convert('RGB')).astype(np.float32)
    C = np.asarray(Image.open(io.BytesIO(f['images/options/hmenu_colormap.bmp'])))
    return B, C, Image.open(io.BytesIO(f['images/options/hmenu_colormap.bmp'])).getpalette()


def red(a):
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    return ((r > 120) & (g < 80) & (b < 80)).astype(np.uint8)

def boxes(B, C):
    """Red initial boxes, one per colormap index (the colormap separates touching boxes)."""
    r, g, b = B[..., 0], B[..., 1], B[..., 2]
    red = ((r > 80) & (r > 1.7 * g) & (r > 1.7 * b)).astype(np.uint8)
    out = []
    for k in range(8):
        n, lab, st, _ = cv2.connectedComponentsWithStats((C == k).astype(np.uint8))
        area = (lab == 1 + int(np.argmax(st[1:, 4]))).astype(np.uint8)
        m = cv2.morphologyEx(red * cv2.dilate(area, np.ones((5, 5), np.uint8)), cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
        n, lab, st, _ = cv2.connectedComponentsWithStats(m)
        i = 1 + int(np.argmax(st[1:, 4]))
        # fill the box interior (letter included)
        mk = (lab == i).astype(np.uint8)
        hull = np.zeros_like(mk); cv2.fillConvexPoly(hull, cv2.convexHull(np.argwhere(mk)[:, ::-1].astype(np.int32)), 1)
        out.append((hull, tuple(int(v) for v in st[i][:4])))
    return out


def ink_cover(gray):
    """Dark strokes against the local paper tone, 0..1."""
    bh = cv2.morphologyEx(gray.astype(np.float32), cv2.MORPH_BLACKHAT, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (17, 17)))
    return np.clip((bh - 12) / 90, 0, 1)


def corners(m):
    """Corners TL, TR, BR, BL of a box mask (4-point fit of its outline)."""
    cnt = cv2.findContours(m, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)[0]
    cnt = max(cnt, key=cv2.contourArea)
    hull = cv2.convexHull(cnt)
    peri = cv2.arcLength(hull, True)
    for e in np.linspace(0.01, 0.2, 60):
        ap = cv2.approxPolyDP(hull, e * peri, True)
        if len(ap) <= 4:
            break
    if len(ap) != 4:
        bx = cv2.boxPoints(cv2.minAreaRect(cnt)); ap = bx.reshape(-1, 1, 2)
    p = ap.reshape(-1, 2).astype(np.float64)
    c = p.mean(0); ang = np.arctan2(p[:, 1] - c[1], p[:, 0] - c[0])
    p = p[np.argsort(ang)]          # clockwise from the top-left in image coordinates
    i = int(np.argmin(p.sum(1)))
    p = np.roll(p, -i, 0)
    return p                        # TL, TR, BR, BL


def line_matrix(cn, g, b, f, fx=1.0, k=0.0):
    """Template -> image for a word next to the box with corners cn.

    Page units: box = 1x1. Template point (u, v) lies at page (1 + g + (u - OX) * f, b + (v - OY) * f)."""
    tl, tr, br, bl = cn
    ex = ((tr - tl) + (br - bl)) / 2; ey = ((bl - tl) + (br - tr)) / 2
    # page (px, py) = (1 + g + (u-OX)*f*fx, b + (v-OY)*f + (u-OX)*f*k)
    A = np.stack([ex * f * fx + ey * f * k, ey * f], 1)
    t = tl + ex * (1 + g) + ey * b - A @ np.array([OX, OY])
    return np.hstack([A, t[:, None]]).astype(np.float32)


def fit_global(cov, regions, cns):
    """g, b, f shared by all lines, maximising the summed correlation of the English words."""
    tm = [render(w[1:]) for w in EN]
    tg = [cv2.GaussianBlur(cov * cv2.dilate(r, np.ones((5, 5), np.uint8)), (0, 0), 1.0) for r in regions]
    def score(p):
        tot = 0
        for k in range(8):
            wv = cv2.GaussianBlur(cv2.warpAffine(tm[k], line_matrix(cns[k], *p), (800, 600), flags=cv2.INTER_AREA), (0, 0), 1.0)
            n = np.linalg.norm(wv) * np.linalg.norm(tg[k])
            tot += (wv * tg[k]).sum() / n if n else 0
        return tot / 8
    best, p = -1, None
    for g in (0.0, 0.15, 0.3):
        for bb in (0.7, 0.85, 1.0):
            for f in (0.008, 0.011, 0.014):
                q = np.array([g, bb, f, 1.0, 0.0]); v = score(q)
                if v > best:
                    best, p = v, q
    steps = np.array([0.05, 0.05, 0.001, 0.04, 0.03])
    for _ in range(14):
        imp = True
        while imp:
            imp = False
            for i in range(5):
                for sg in (1, -1):
                    q = p.copy(); q[i] += sg * steps[i]; v = score(q)
                    if v > best + 1e-6:
                        best, p, imp = v, q, True
        steps /= 2
    return p, best


def line_adjust(M, dy, th):
    """M turned by th degrees about the start of the word and moved dy pixels down."""
    p = M @ np.array([OX, OY, 1.0])
    c, s = np.cos(np.radians(th)), np.sin(np.radians(th))
    R = np.array([[c, -s], [s, c]])
    return np.hstack([R @ M[:, :2], (R @ (M[:, 2] - p) + p + np.array([0.0, dy]))[:, None]]).astype(np.float32)


def fit_lines(cov, regions, cns, gp):
    """Per line height and turn of the English word against the shared fit (the page bends)."""
    out = []
    for k in range(8):
        t = render(EN[k][1:])
        tg = cv2.GaussianBlur(cov * cv2.dilate(regions[k], np.ones((5, 5), np.uint8)), (0, 0), 1.0)
        M0 = line_matrix(cns[k], *gp)
        def score(q):
            M = line_adjust(M0, q[1], q[2])
            M[:, 2] += np.float32([q[0], 0])
            wv = cv2.GaussianBlur(cv2.warpAffine(t, M, (800, 600), flags=cv2.INTER_AREA), (0, 0), 1.0)
            n = np.linalg.norm(wv) * np.linalg.norm(tg)
            return (wv * tg).sum() / n if n else 0
        q, best, st = np.zeros(3), score(np.zeros(3)), np.ones(3)
        for _ in range(10):
            imp = True
            while imp:
                imp = False
                for i in range(3):
                    for sg in (1, -1):
                        r = q.copy(); r[i] += sg * st[i]; v = score(r)
                        if v > best + 1e-6:
                            best, q, imp = v, r, True
            st /= 2
        out.append([q[1], q[2]])    # dx belongs to the English gap, not kept
    # one smooth bend over the page, not a correction per word
    k, out = np.arange(8), np.array(out)
    return [[float(np.polyval(np.polyfit(k, out[:, j], 2), i)) for j in range(2)] for i in k]


Q, MG = 56, 10         # canonical box: red square Q x Q, margin for the gold frame


def smooth_corners(cns):
    """Corners as a quadratic of the line number: same size and slant change from line to line."""
    k = np.arange(8)
    flat = np.array([c.reshape(-1) for c in cns])
    out = np.zeros_like(flat)
    for j in range(flat.shape[1]):
        out[:, j] = np.polyval(np.polyfit(k, flat[:, j], 2), k)
    return [o.reshape(4, 2) for o in out]


def box_h(cn):
    """Canonical box coordinates -> image."""
    src = np.float32([[0, 0], [Q, 0], [Q, Q], [0, Q]]) + MG
    return cv2.getPerspectiveTransform(src, np.float32(cn))


def fill_paper(B, hole):
    """Paper under the hole from the surrounding paper: matched tone and grain, no smear."""
    keep = (1 - hole).astype(np.float32)
    est = np.zeros_like(B)
    wsum = np.zeros(hole.shape, np.float32)
    for sig in (3, 6, 12, 24):
        w = cv2.GaussianBlur(keep, (0, 0), sig)
        v = cv2.GaussianBlur(B * keep[..., None], (0, 0), sig)
        ok = (w > 0.15) & (wsum == 0)
        est[ok] = v[ok] / w[ok, None]
        wsum[ok] = 1
    resid = (B - cv2.GaussianBlur(B, (0, 0), 1.5)) * keep[..., None]
    sd = np.sqrt(cv2.GaussianBlur(resid ** 2, (0, 0), 10) / np.maximum(cv2.GaussianBlur(keep, (0, 0), 10), 1e-3)[..., None])
    rng = np.random.default_rng(7)
    grain = cv2.GaussianBlur(rng.standard_normal(B.shape[:2]).astype(np.float32), (0, 0), 0.7)
    grain /= grain.std()
    est = est + sd * grain[..., None]
    a = cv2.GaussianBlur(hole.astype(np.float32), (0, 0), 1.2)
    a = np.maximum(a, hole)
    return B * (1 - a[..., None]) + est * a[..., None]


def build_box(folder):
    """Clean initial box (no letter) from the eight German boxes, aligned box by box."""
    size = Q + 2 * MG
    DB = np.asarray(Image.open(os.path.join(folder, 'Hauptmenu.jpg')).convert('RGB')).astype(np.float32)
    DC = np.asarray(Image.open(os.path.join(folder, 'hmenu_colormap.bmp')))
    dbx = boxes(DB, DC)
    dr, dg, dbb = DB[..., 0], DB[..., 1], DB[..., 2]
    dred = np.zeros(DB.shape[:2], np.uint8)
    for m, _ in dbx:
        dred |= m
    dgold = ((dr > 120) & (dg > 80) & (dbb < 0.75 * dg) & (dr > dbb + 40)).astype(np.uint8)
    dfull = cv2.morphologyEx(dred | (dgold & cv2.dilate(dred, np.ones((11, 11), np.uint8))), cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
    inner = np.zeros((size, size), bool); inner[MG + 3:MG + Q - 3, MG + 3:MG + Q - 3] = True
    tiles, masks, alph = [], [], []
    for k in range(8):
        Hk = box_h(corners(dbx[k][0]))
        cb = cv2.warpPerspective(DB, Hk, (size, size), flags=cv2.WARP_INVERSE_MAP | cv2.INTER_CUBIC)
        ca = cv2.warpPerspective(dfull.astype(np.float32), Hk, (size, size), flags=cv2.WARP_INVERSE_MAP | cv2.INTER_LINEAR)
        # letter plus its gold outline: dark pixels and yellow pixels inside the box
        yel = (cb[..., 0] > 150) & (cb[..., 1] > 120) & (cb[..., 2] < 110)
        let = ((cb.max(2) < 95) | yel) & inner
        let = cv2.dilate(let.astype(np.uint8), np.ones((3, 3), np.uint8)) > 0
        let &= inner
        tiles.append(cb); masks.append(let); alph.append(ca)
    ref = int(np.argmax([np.ptp(corners(m)[:, 0]) for m, _ in dbx]))
    crit = (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT, 200, 1e-6)
    stack = []
    for k in range(8):
        cb, let = tiles[k], masks[k]
        if k != ref:
            M = np.eye(3, dtype=np.float32)
            try:
                tm = (~(masks[ref] | let)).astype(np.uint8)
                _, M = cv2.findTransformECC(tiles[ref][..., 0], cb[..., 0], M, cv2.MOTION_HOMOGRAPHY, crit, tm, 3)
                cb = cv2.warpPerspective(cb, M, (size, size), flags=cv2.WARP_INVERSE_MAP | cv2.INTER_CUBIC)
                let = cv2.warpPerspective(let.astype(np.uint8), M, (size, size), flags=cv2.WARP_INVERSE_MAP | cv2.INTER_NEAREST) > 0
            except cv2.error:
                pass
        t = cb.copy(); t[let] = np.nan
        stack.append(t)
    canon = np.nanmedian(np.stack(stack), 0)
    nanm = np.isnan(canon).any(2).astype(np.uint8)
    canon = np.nan_to_num(canon)
    if nanm.any():
        canon = cv2.inpaint(np.clip(canon, 0, 255).astype(np.uint8), nanm, 3, cv2.INPAINT_TELEA).astype(np.float32)
    calpha = np.clip(np.median(np.stack(alph), 0), 0, 1)
    calpha = cv2.GaussianBlur(calpha, (0, 0), 0.4)
    lk = np.array([12.0, 8.0, 6.0])
    goldc = np.array([235.0, 200.0, 70.0])
    try:
        yy = np.concatenate([tiles[k][masks[k] & (tiles[k][..., 1] > 130) & (tiles[k][..., 2] < 110)] for k in range(8)])
        if len(yy) > 20:
            goldc = np.median(yy, 0)
    except ValueError:
        pass

    rgba = np.dstack([np.clip(canon, 0, 255), np.clip(calpha * 255, 0, 255)]).astype(np.uint8)
    Image.fromarray(rgba, 'RGBA').save(os.path.join(HERE, 'box.png'))
    with open(os.path.join(HERE, 'box.json'), 'w', newline='\n') as f:
        json.dump({'ink': [12.0, 8.0, 6.0], 'gold': [float(v) for v in goldc]}, f)
        f.write('\n')


def build(lang, out):
    os.makedirs(out, exist_ok=True)
    words = WORDS[lang]
    B, C, palette = load()
    gray = B.mean(2)
    bx = boxes(B, C)
    cov = ink_cover(gray)
    r, g, b = B[..., 0], B[..., 1], B[..., 2]

    # 1. what to remove: English words and the boxes with their gold frame
    redm = np.zeros(gray.shape, np.uint8)
    for m, _ in bx:
        redm |= m
    gold = ((r > 120) & (g > 80) & (b < 0.75 * g) & (r > b + 40)).astype(np.uint8)
    boxfull = cv2.morphologyEx(redm | (gold & cv2.dilate(redm, np.ones((11, 11), np.uint8))), cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
    boxfull = cv2.dilate(boxfull, np.ones((3, 3), np.uint8))
    boxzone = cv2.dilate(boxfull, np.ones((5, 5), np.uint8))
    regions, textmask = [], np.zeros(gray.shape, np.uint8)
    for k in range(8):
        n, lab, st, _ = cv2.connectedComponentsWithStats((C == k).astype(np.uint8))
        rr = cv2.dilate((lab == 1 + int(np.argmax(st[1:, 4]))).astype(np.uint8), np.ones((7, 7), np.uint8))
        regions.append(rr * (1 - boxzone))
        textmask |= ((cov > 0.08) & (rr > 0) & (boxzone == 0)).astype(np.uint8)
    hole = cv2.dilate(textmask, np.ones((5, 5), np.uint8)) | boxzone
    clean = fill_paper(B, hole)
    ink = np.median(B[(cov > 0.85) & (textmask > 0)], 0)

    # 2. canonical box from the German boxes (sharper), aligned box by box, letters masked
    cns_raw = [corners(m) for m, _ in bx]
    cns = smooth_corners(cns_raw)
    size = Q + 2 * MG
    box = Image.open(os.path.join(HERE, 'box.png'))
    canon = np.asarray(box.convert('RGB')).astype(np.float32)
    calpha = np.asarray(box.getchannel('A')).astype(np.float32) / 255
    meta = json.load(open(os.path.join(HERE, 'box.json')))
    lk, goldc = np.array(meta['ink']), np.array(meta['gold'])

    # 3. compose: paper, words, boxes with initials
    newimg = clean.copy()
    fit = os.path.join(HERE, 'fit.json')
    if not os.path.exists(fit):
        # Measure once how the English words sit on the lines of the book.
        gp, sc = fit_global(cov, regions, cns_raw)
        with open(fit, 'w', newline='\n') as f:
            json.dump([[float(v) for v in gp], float(sc)], f)
            f.write('\n')
    gp, sc = json.load(open(fit))
    lfit = os.path.join(HERE, 'lines.json')
    if not os.path.exists(lfit):
        with open(lfit, 'w', newline='\n') as f:
            json.dump(fit_lines(cov, regions, cns, gp), f)
            f.write('\n')
    adj = json.load(open(lfit))
    gp = [GAP] + list(gp[1:])
    covmaps = []
    for k in range(8):
        Wk = line_adjust(line_matrix(cns[k], *gp), *adj[k])
        # same stroke weight on screen in every line: the extra weight shrinks as the line scale grows
        sk = np.sqrt(abs(np.linalg.det(line_matrix(cns[k], *gp)[:, :2])))
        s0 = np.sqrt(abs(np.linalg.det(line_matrix(cns[0], *gp)[:, :2])))
        wk = WEIGHT * (s0 / sk) ** 3
        t = wk * render(words[k][1:], stroke=1) + (1 - wk) * render(words[k][1:], stroke=0)
        c = cv2.warpAffine(t, Wk, (800, 600), flags=cv2.INTER_AREA)
        c = cv2.GaussianBlur(c, (0, 0), 0.4)
        covmaps.append(c)
        newimg = newimg * (1 - c[..., None]) + ink * c[..., None]
    letter_cov = []
    big = ImageFont.truetype(FONT, 200)
    for k in range(8):
        Hk = box_h(cns[k])
        # initial in canonical space: same height and weight in every box
        sc4 = 4
        L = Image.new('L', (size * sc4, size * sc4), 0)
        d = ImageDraw.Draw(L)
        ch = words[k][0]
        bb = d.textbbox((0, 0), 'H', font=big, stroke_width=8)
        capH = bb[3] - bb[1]
        fs = int(200 * (Q * 0.80 * sc4) / capH)
        fnt = ImageFont.truetype(FONT, fs)
        sw = max(1, int(fs * 0.016))
        cb = d.textbbox((0, 0), ch, font=fnt, stroke_width=sw)
        cx = (size * sc4 - (cb[2] - cb[0])) / 2 - cb[0]
        hb = d.textbbox((0, 0), 'H', font=fnt, stroke_width=sw)
        cy = (size * sc4 - (hb[3] - hb[1])) / 2 - hb[1]
        d.text((cx, cy), ch, font=fnt, fill=255, stroke_width=sw, stroke_fill=255)
        G = Image.new('L', L.size, 0)
        ImageDraw.Draw(G).text((cx, cy), ch, font=fnt, fill=255, stroke_width=sw + max(1, int(fs * 0.026)), stroke_fill=255)
        lc = np.asarray(L.resize((size, size), Image.LANCZOS), np.float32) / 255
        gc = np.asarray(G.resize((size, size), Image.LANCZOS), np.float32) / 255
        tile = canon * (1 - gc[..., None]) + goldc * gc[..., None]
        tile = tile * (1 - lc[..., None]) + lk * lc[..., None]
        wb = cv2.warpPerspective(tile, Hk, (800, 600), flags=cv2.INTER_CUBIC)
        wa = cv2.warpPerspective(calpha, Hk, (800, 600), flags=cv2.INTER_LINEAR)
        newimg = newimg * (1 - wa[..., None]) + wb * wa[..., None]
        letter_cov.append(wa)
    final = np.clip(newimg, 0, 255)
    Image.fromarray(final.astype(np.uint8)).save(f'{out}/Hauptmenu.jpg', quality=95, subsampling=0)

    # 4. click areas
    cm = np.full((600, 800), 255, np.uint8)
    for k in range(8):
        band = ((covmaps[k] > 0.15) | (letter_cov[k] > 0.5)).astype(np.uint8)
        band = cv2.morphologyEx(band, cv2.MORPH_CLOSE, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (25, 25)))
        band = cv2.dilate(band, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (9, 9)))
        cm[(band > 0) & (cm == 255)] = k
    pal = palette
    ci = Image.fromarray(cm, 'P'); ci.putpalette(pal); ci.save(f'{out}/hmenu_colormap.bmp')

    # 5. hover sprites: the background itself, only the word glows (no tinted edge)
    anchors = [(235, 185), (208, 211), (180, 239), (148, 268), (115, 300), (80, 334), (41, 371), (0, 412)]
    sprites = []
    for k in range(8):
        ax, ay = anchors[k]
        area = (cm == k).astype(np.uint8)
        ys, xs = np.nonzero(area)
        x1, y1 = xs.max() + 1, ys.max() + 1
        c = covmaps[k]
        halo = np.clip(cv2.GaussianBlur(c, (0, 0), 2.4) * 1.6, 0, 1)
        nt = final * (1 - c[..., None]) + clean * c[..., None]      # page without this word
        glow = nt + (np.array([255, 236, 150]) - nt) * (halo * 0.8)[..., None]
        core = glow * (1 - c[..., None]) + np.array([255, 250, 212]) * c[..., None]
        # fade the glow to the plain background before the edge of the area
        edge = cv2.GaussianBlur(cv2.erode(area, np.ones((5, 5), np.uint8)).astype(np.float32), (0, 0), 2)
        spr = final * (1 - edge[..., None]) + core * edge[..., None]
        spr = np.clip(spr, 0, 255).astype(np.uint8)[ay:y1, ax:x1].copy()
        sprites.append((spr, area[ay:y1, ax:x1]))
    sw_ = max(s.shape[1] for s, _ in sprites); sh_ = sum(s.shape[0] for s, _ in sprites)
    sheet = np.zeros((sh_, sw_, 3), np.uint8); key = np.ones((sh_, sw_), bool)
    y = 0; rects = []
    for s_, a_ in sprites:
        h_, w_ = a_.shape
        sheet[y:y + h_, :w_] = s_; key[y:y + h_, :w_] = a_ == 0
        rects.append((y, w_, h_)); y += h_
    filled = cv2.inpaint(sheet, key.astype(np.uint8), 3, cv2.INPAINT_TELEA)
    q = Image.fromarray(filled).quantize(255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.FLOYDSTEINBERG)
    qa = np.asarray(q).copy(); qa[key] = 255
    p = (q.getpalette() + [0] * 768)[:765] + [255, 0, 255]
    Image.fromarray(qa, 'P').putpalette(p) if False else None
    qi = Image.fromarray(qa, 'P'); qi.putpalette(p); qi.save(f'{out}/p_hmenu01.bmp')
    with open(f'{out}/textures.tsv', 'w', newline='\n') as f:
        f.write('# Sprite rectangles of p_hmenu01.bmp (tools/gfx/gen.py): texture file<TAB>x y width height\n')
        for k, (y, w, h) in enumerate(rects):
            f.write(f'{SHEET}\t0 {y} {w} {h}\n')
    print(lang, 'ok', rects)


def main():
    global ENGLISH, FONT, font
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--archive', required=True)
    ap.add_argument('--font', required=True)
    ap.add_argument('--out', default=os.path.join(HERE, '..', '..', 'data', 'gfx'))
    ap.add_argument('--box-from')
    ap.add_argument('langs', nargs='*', default=[l for l in WORDS if l != 'en'])
    a = ap.parse_args()
    FONT = a.font
    font = ImageFont.truetype(FONT, SIZE)
    ENGLISH = read_archive(a.archive, ['images/options/Hauptmenu.jpg', 'images/options/hmenu_colormap.bmp'])
    if a.box_from:
        build_box(a.box_from)
    for lang in a.langs:
        build(lang, os.path.join(a.out, lang))


if __name__ == '__main__':
    main()
