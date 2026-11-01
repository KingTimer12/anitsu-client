/**
 * Deriva a paleta de destaque a partir de `coverImage.color`.
 * A cor bruta costuma ser escura (ex.: `#780d0d`); para texto sobre o fundo
 * escuro a luminosidade é elevada mantendo o matiz.
 */

const BRAND = "#e8283c";

function hexToHsl(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h =
    max === r ? ((g - b) / d + (g < b ? 6 : 0)) / 6 : max === g ? ((b - r) / d + 2) / 6 : ((r - g) / d + 4) / 6;
  return [h * 360, s, l];
}

const hsl = (h: number, s: number, l: number, a = 1) =>
  `hsl(${h.toFixed(1)} ${(s * 100).toFixed(1)}% ${(l * 100).toFixed(1)}% / ${a})`;

export interface AccentVars {
  "--accent": string;
  "--accent-ink": string;
  "--accent-soft": string;
  "--accent-glow": string;
}

export function accentVars(color: string | null | undefined): AccentVars {
  const parsed = hexToHsl(color ?? BRAND) ?? hexToHsl(BRAND)!;
  const [h, s0, l0] = parsed;
  // Capas quase cinza: mantém neutro, sem inventar saturação.
  const s = s0 < 0.12 ? s0 : Math.max(s0, 0.45);
  const base = Math.min(Math.max(l0, 0.42), 0.6);
  return {
    "--accent": hsl(h, s, base),
    "--accent-ink": hsl(h, Math.min(s, 0.85), Math.max(l0, 0.72)),
    "--accent-soft": hsl(h, s, base, 0.16),
    "--accent-glow": hsl(h, s, Math.min(l0, 0.35), 0.55),
  };
}
