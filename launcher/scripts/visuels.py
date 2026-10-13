# -*- coding: utf-8 -*-
"""Genere les visuels neutres fournis avec le depot (a remplacer par les votres).

    python launcher/scripts/visuels.py        (Pillow requis : pip install pillow)
    cd launcher && npx tauri icon app-icon.png

Ecrit launcher/app-icon.png (1024 px, source des icones), launcher/public/logo.png
(256 px, barre de titre) et launcher/public/entete.webp (1536 x 287, fond d'en-tete),
dans les couleurs "fond" et "accent" de launcher.config.json.
"""
import json
import math
import os

from PIL import Image, ImageDraw, ImageFilter

ICI = os.path.dirname(os.path.abspath(__file__))
LAUNCHER = os.path.dirname(ICI)
CONFIG = os.path.join(os.path.dirname(LAUNCHER), "launcher.config.json")


def rvb(hexa):
    hexa = hexa.lstrip("#")
    return tuple(int(hexa[i:i + 2], 16) for i in (0, 2, 4))


def couleurs():
    with open(CONFIG, encoding="utf-8") as f:
        c = json.load(f).get("couleurs", {})
    return rvb(c.get("fond", "#0b0f12")), rvb(c.get("surface", "#12181d")), rvb(c.get("accent", "#6cb8e6"))


def embleme(taille, fond, surface, accent):
    k = 4
    t = taille * k
    im = Image.new("RGBA", (t, t), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    marge = t * 0.06
    d.rounded_rectangle([marge, marge, t - marge, t - marge], radius=t * 0.2, fill=surface + (255,), outline=accent + (255,), width=int(t * 0.025))
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


def entete(fond, accent):
    l, h = 1536, 287
    im = Image.new("RGB", (l, h), fond)
    px = im.load()
    for y in range(h):
        for x in range(0, l):
            lueur = max(0.0, 1 - math.hypot((x - l * 0.78) / (l * 0.5), (y - h * 0.15) / (h * 1.2)))
            ombre = y / h * 0.35
            px[x, y] = tuple(int(f + (a - f) * 0.22 * lueur - f * ombre) for f, a in zip(fond, accent))
    d = ImageDraw.Draw(im)
    for i in range(7):
        base = h * (0.55 + i * 0.07)
        points = [(x, base - 30 * math.sin(x / 210 + i) * math.cos(x / 530 - i * 0.7)) for x in range(0, l + 8, 8)]
        teinte = tuple(int(f + (a - f) * (0.18 - i * 0.02)) for f, a in zip(fond, accent))
        d.line(points, fill=teinte, width=2)
    return im.filter(ImageFilter.GaussianBlur(0.6))


def main():
    fond, surface, accent = couleurs()
    embleme(1024, fond, surface, accent).save(os.path.join(LAUNCHER, "app-icon.png"))
    embleme(256, fond, surface, accent).save(os.path.join(LAUNCHER, "public", "logo.png"))
    entete(fond, accent).save(os.path.join(LAUNCHER, "public", "entete.webp"), quality=82)
    print("Visuels ecrits. Icones : cd launcher && npx tauri icon app-icon.png")


if __name__ == "__main__":
    main()
