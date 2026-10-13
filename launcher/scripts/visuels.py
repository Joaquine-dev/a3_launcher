# -*- coding: utf-8 -*-
"""Genere les visuels neutres fournis avec le depot (a remplacer par les votres).

    python launcher/scripts/visuels.py        (Pillow requis : pip install pillow)
    cd launcher && npx tauri icon app-icon.png

Ecrit launcher/app-icon.png (1024 px, source des icones) et launcher/public/logo.png
(256 px, barre laterale), dans les couleurs "surface" et "accent" de launcher.config.json.
"""
import json
import math
import os

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
    return rvb(c.get("fond", "#141618")), rvb(c.get("surface", "#1c1f22")), rvb(c.get("accent", "#8a9a5b"))


def embleme(taille, fond, surface, accent):
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


def main():
    fond, surface, accent = couleurs()
    embleme(1024, fond, surface, accent).save(os.path.join(LAUNCHER, "app-icon.png"))
    embleme(256, fond, surface, accent).save(os.path.join(LAUNCHER, "public", "logo.png"))
    print("Visuels ecrits. Icones : cd launcher && npx tauri icon app-icon.png")


if __name__ == "__main__":
    main()
