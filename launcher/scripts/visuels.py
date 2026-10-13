# -*- coding: utf-8 -*-
"""Genere les visuels neutres fournis avec le depot (a remplacer par les votres).

    python launcher/scripts/visuels.py          (Pillow requis : pip install pillow)
    cd launcher && npx tauri icon app-icon.png
    python launcher/scripts/visuels.py --fond   (Pillow et numpy : pip install pillow numpy)

Sans option : ecrit launcher/app-icon.png (1024 px, source des icones) et launcher/public/logo.png
(256 px, affiche en 24 px dans la barre de titre), dans les couleurs "surface" et "accent" de launcher.config.json.
--fond : ecrit seulement launcher/public/fond.webp, image d'ambiance neutre (crepuscule sur la mer,
ile, brume, grain) dessinee par calcul, a remplacer par une capture de votre serveur.
"""
import json
import math
import os
import sys

from PIL import Image, ImageDraw

ICI = os.path.dirname(os.path.abspath(__file__))
LAUNCHER = os.path.dirname(ICI)
CONFIG = os.path.join(os.path.dirname(LAUNCHER), "launcher.config.json")


def rvb(hexa):
    hexa = hexa.lstrip("#")
    return tuple(int(hexa[i:i + 2], 16) for i in (0, 2, 4))


def couleurs():
    with open(CONFIG, encoding="utf-8") as f:
        c = json.load(f).get("couleurs", {})
    return rvb(c.get("surface", "#141824")), rvb(c.get("accent", "#ffb547"))


def embleme(taille, surface, accent):
    k = 4
    t = taille * k
    im = Image.new("RGBA", (t, t), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    marge = t * 0.06
    d.rounded_rectangle([marge, marge, t - marge, t - marge], radius=t * 0.07, fill=surface + (255,), outline=accent + (255,), width=int(t * 0.025))
    cx, cy = t / 2, t / 2
    r = t * 0.27
    epais = int(t * 0.045)
    d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=accent + (255,), width=epais)
    for a in range(4):
        ang = a * math.pi / 2
        x1, y1 = cx + math.cos(ang) * r * 0.55, cy + math.sin(ang) * r * 0.55
        x2, y2 = cx + math.cos(ang) * r * 1.35, cy + math.sin(ang) * r * 1.35
        d.line([x1, y1, x2, y2], fill=accent + (255,), width=epais)
    p = t * 0.035
    d.ellipse([cx - p, cy - p, cx + p, cy + p], fill=accent + (255,))
    return im.resize((taille, taille), Image.LANCZOS)


def image_fond(chemin, largeur=1650, hauteur=1020):
    import numpy as np
    from PIL import ImageFilter

    W, H = largeur, hauteur
    rng = np.random.default_rng(7)
    y = np.linspace(0, 1, H)[:, None]
    x = np.linspace(0, 1, W)[None, :]

    def melange(a, b, t):
        t = np.clip(t, 0, 1)[..., None]
        return np.array(a, float) * (1 - t) + np.array(b, float) * t

    def bruit(lignes, colonnes, flou):
        b = Image.fromarray((rng.random((lignes, colonnes)) * 255).astype(np.uint8))
        return np.asarray(b.resize((W, H), Image.BICUBIC).filter(ImageFilter.GaussianBlur(flou * W / 2200)), float) / 255

    horizon = 0.60
    ciel = melange((14, 22, 40), (58, 52, 78), (y / horizon) ** 1.4)
    ciel = melange(ciel, (214, 112, 58), np.clip((y - horizon * 0.55) / (horizon * 0.45), 0, 1) ** 2.2)
    ciel = melange(ciel, (255, 196, 120), np.clip((y - horizon * 0.9) / (horizon * 0.1), 0, 1) ** 1.5)
    img = np.broadcast_to(ciel, (H, W, 3)).copy()

    sx, sy = 0.66, horizon - 0.015
    d = np.sqrt(((x - sx) * 1.6) ** 2 + (y - sy) ** 2)
    halo = np.exp(-(d / 0.20) ** 2) * 0.85 + np.exp(-(d / 0.035) ** 2)
    img = img + halo[..., None] * np.array([255, 170, 90]) * 0.55
    soleil = np.broadcast_to(d < 0.022, (H, W))
    img[soleil] = img[soleil] * 0.2 + np.array([255, 214, 160]) * 0.8

    dens = np.clip((bruit(40, 18, 30) * 0.7 + bruit(90, 30, 8) * 0.3 - 0.48) * 3.2, 0, 1) * np.clip(1 - np.abs(y - 0.36) / 0.22, 0, 1)
    bord = np.exp(-(d / 0.35) ** 2)
    nuage = melange(np.broadcast_to(np.array([40, 34, 56], float), (H, W, 3)), (230, 120, 80), np.broadcast_to(bord * 1.3 + (y / horizon) * 0.3, (H, W)))
    img = img * (1 - dens[..., None] * 0.85) + nuage * dens[..., None] * 0.85

    prof = np.clip((y - horizon) / (1 - horizon), 0, 1)
    eau = melange((70, 52, 60), (10, 16, 26), prof ** 0.6)
    reflet = np.exp(-((x - sx) / (0.03 + prof * 0.10)) ** 2) * (1 - prof) ** 1.2
    vague = 0.6 + 0.4 * np.sin(y * 900 + np.sin(x * 40) * 3) * np.sin(x * 200 + y * 50)
    eau = eau + (reflet * vague)[..., None] * np.array([255, 160, 80]) * 0.8
    img = np.where((y > horizon)[..., None], np.broadcast_to(eau, (H, W, 3)), img)

    xs = np.linspace(0, 1, W)
    yy = np.arange(H)[:, None] / H
    couches = [
        (horizon + 0.001, 0.035, 11, 0.02, 0.40, (96, 70, 86), 0.8),
        (horizon + 0.001, 0.022, 21, 0.84, 1.10, (92, 68, 84), 0.8),
        (horizon + 0.012, 0.13, 31, -0.05, 0.42, (40, 32, 48), 0.3),
        (horizon + 0.30, 0.30, 41, 0.70, 1.25, (14, 14, 22), 0.0),
        (1.03, 0.16, 51, -0.25, 0.30, (9, 10, 15), 0.0),
    ]
    for base, amp, graine, debut, fin, couleur, brume in couches:
        r = np.random.default_rng(graine)
        p = sum(a * np.sin(xs * k * np.pi + r.uniform(0, 6.28)) for k, a in [(2, 1.0), (5, 0.5), (11, 0.25), (23, 0.12), (61, 0.04), (140, 0.02)])
        p = (p - p.min()) / (p.max() - p.min())
        env = np.clip((xs - debut) / 0.12, 0, 1) * np.clip((fin - xs) / 0.12, 0, 1)
        crete = base - amp * p * env
        masque = (yy > crete[None, :]) & (env[None, :] > 0.002)
        if base < 0.7:
            masque &= yy < base + 0.006
        teinte = np.array(couleur, float)
        rempli = melange(teinte * 1.25 + np.array([30, 14, 0]) * brume, teinte, np.clip((yy - crete[None, :]) / 0.15, 0, 1))
        img = np.where(masque[..., None], rempli, img)
        voile = np.exp(-((yy - crete[None, :] - 0.01) / 0.03) ** 2) * 0.35
        img = img + voile[..., None] * np.array([200, 130, 100]) * 0.25

    brume = np.exp(-((y - (horizon + 0.03)) / 0.05) ** 2) * (0.6 + 0.4 * np.sin(x * 9 + 1.3))
    img = img + brume[..., None] * np.array([230, 150, 110]) * 0.22

    im = Image.fromarray(np.clip(img, 0, 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.6))
    a = np.asarray(im).astype(float) + rng.normal(0, 6, (H, W, 1))
    a *= np.clip(1 - 0.45 * (((x - 0.5) * 1.3) ** 2 + ((y - 0.45) * 1.1) ** 2), 0, 1)[..., None]
    Image.fromarray(np.clip(a, 0, 255).astype(np.uint8)).save(chemin, "WEBP", quality=74, method=6)


def main():
    if "--fond" in sys.argv[1:]:
        chemin = os.path.join(LAUNCHER, "public", "fond.webp")
        image_fond(chemin)
        print("Image de fond ecrite : %s (%d Ko)" % (chemin, os.path.getsize(chemin) // 1024))
        return
    surface, accent = couleurs()
    embleme(1024, surface, accent).save(os.path.join(LAUNCHER, "app-icon.png"))
    embleme(256, surface, accent).save(os.path.join(LAUNCHER, "public", "logo.png"))
    print("Visuels ecrits. Icones : cd launcher && npx tauri icon app-icon.png")


if __name__ == "__main__":
    main()
