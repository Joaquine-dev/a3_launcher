/*
 * Fond topographique : courbes de niveau d'un relief imaginaire (marching squares
 * sur une grille de 20 px, meme terrain que le site) et coordonnee de grille facon
 * Arma. Seul element decoratif du launcher, pose derriere l'en-tete.
 */
const LARGEUR = 1600;
const HAUTEUR = 480;
const VUE = 1200;
const PAS = 20;
const DECALAGE = 260;
const COORDONNEE = "GR 142·087";
const SVG = "http://www.w3.org/2000/svg";

function altitude(x: number, y: number): number {
  return (
    Math.exp(-((x - 250) ** 2 + (y - 150) ** 2) / 50000) +
    0.85 * Math.exp(-((x - 800) ** 2 + (y - 40) ** 2) / 75000) +
    0.95 * Math.exp(-((x - 1340) ** 2 + (y - 250) ** 2) / 60000) +
    0.5 * Math.exp(-((x - 560) ** 2 + (y - 420) ** 2) / 38000) +
    0.07 * Math.sin(x / 97 + y / 61) +
    0.05 * Math.cos(x / 53 - y / 89)
  );
}

function courbes(): { fines: string; maitresses: string } {
  const nx = LARGEUR / PAS;
  const ny = HAUTEUR / PAS;
  const v: number[][] = [];
  for (let j = 0; j <= ny; j++) {
    v.push([]);
    for (let i = 0; i <= nx; i++) v[j].push(altitude(i * PAS, j * PAS));
  }
  const fines: string[] = [];
  const maitresses: string[] = [];

  for (let n = 1; n < 11; n++) {
    const seuil = n / 10;
    const segments: [string, string][] = [];
    const croise = (x0: number, y0: number, a: number, x1: number, y1: number, b: number) => {
      const t = (seuil - a) / (b - a);
      return `${Math.round(x0 + (x1 - x0) * t)},${Math.round(y0 + (y1 - y0) * t)}`;
    };
    for (let j = 0; j < ny; j++) {
      for (let i = 0; i < nx; i++) {
        const x0 = i * PAS;
        const y0 = j * PAS;
        const x1 = x0 + PAS;
        const y1 = y0 + PAS;
        const a = v[j][i];
        const b = v[j][i + 1];
        const c = v[j + 1][i + 1];
        const d = v[j + 1][i];
        const p: string[] = [];
        if (a < seuil !== b < seuil) p.push(croise(x0, y0, a, x1, y0, b));
        if (b < seuil !== c < seuil) p.push(croise(x1, y0, b, x1, y1, c));
        if (d < seuil !== c < seuil) p.push(croise(x0, y1, d, x1, y1, c));
        if (a < seuil !== d < seuil) p.push(croise(x0, y0, a, x0, y1, d));
        if (p.length >= 2) segments.push([p[0], p[1]]);
        if (p.length === 4) segments.push([p[2], p[3]]);
      }
    }

    const voisins = new Map<string, number[]>();
    segments.forEach(([u, w], k) => {
      voisins.set(u, [...(voisins.get(u) ?? []), k]);
      voisins.set(w, [...(voisins.get(w) ?? []), k]);
    });
    const pris = new Set<number>();
    const suivre = (depart: string, trace: string[]) => {
      let bout = depart;
      for (;;) {
        const k = (voisins.get(bout) ?? []).find((s) => !pris.has(s));
        if (k === undefined) return;
        pris.add(k);
        bout = segments[k][0] === bout ? segments[k][1] : segments[k][0];
        trace.push(bout);
      }
    };
    const lignes: string[] = [];
    segments.forEach(([u, w], k) => {
      if (pris.has(k)) return;
      pris.add(k);
      const avant: string[] = [];
      const apres: string[] = [u, w];
      suivre(w, apres);
      suivre(u, avant);
      const trace = [...avant.reverse(), ...apres];
      if (trace.length < 4) return;
      lignes.push(`M${trace[0]}L${trace.slice(1).join(" ")}`);
    });
    (n % 5 === 0 ? maitresses : fines).push(lignes.join(""));
  }
  return { fines: fines.join(""), maitresses: maitresses.join("") };
}

export function poserTopo(parent: HTMLElement) {
  const trace = courbes();
  const topo = document.createElement("div");
  topo.className = "topo";
  topo.setAttribute("aria-hidden", "true");
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", `${DECALAGE} 0 ${VUE} ${HAUTEUR}`);
  svg.setAttribute("preserveAspectRatio", "xMidYMin slice");
  for (const [classe, d] of [["fines", trace.fines], ["maitresses", trace.maitresses]]) {
    const chemin = document.createElementNS(SVG, "path");
    chemin.setAttribute("class", classe);
    chemin.setAttribute("d", d);
    svg.append(chemin);
  }
  const coord = document.createElement("span");
  coord.className = "topo-coord";
  coord.textContent = COORDONNEE;
  topo.append(svg, coord);
  parent.prepend(topo);
}
