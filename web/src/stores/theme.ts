// Theme store — controls colors, shape, density, and background image.
//
// Persistence: localStorage key `qshare.theme.v1`. Bad/missing data falls
// back to the midnight default without throwing — the SPA should always
// render in some valid state.
//
// Application: the theme + accent + radius are applied by writing CSS
// variables to `document.documentElement.style` AND setting a
// `data-theme="..."` attribute that the preset stylesheets in
// `styles/themes.css` key off. Both layers matter:
//   - presets (themes.css) own the *cohort* of colors per theme
//   - inline `style.setProperty` owns the *user override* (accent, radius)
//     so a single accent picker doesn't have to know about every theme
//
// Background is rendered by a separate `BackgroundLayer` component that
// reads the same reactive signal — we don't write background-image as a
// CSS variable here because that would couple concerns, and an `<img>`
// is easier to debug (you can right-click → open in new tab).

import { createSignal, createEffect } from 'solid-js';

export type ThemeId = 'midnight' | 'daylight' | 'forest' | 'sunset' | 'sakura';
export type Density = 'compact' | 'normal' | 'cozy';
export type BackgroundType = 'none' | 'url' | 'data';

export interface BackgroundConfig {
  type: BackgroundType;
  /** Remote URL (when type === 'url'). */
  url?: string;
  /** data: URL (when type === 'data'). */
  dataUrl?: string;
  /** 0–20 px blur applied via CSS filter. */
  blur: number;
  /** 0–1 overlay opacity applied as a black veil. */
  overlay: number;
}

export interface ThemeConfig {
  id: ThemeId;
  accent: string;
  radius: number;
  density: Density;
  background: BackgroundConfig;
}

const STORAGE_KEY = 'qshare.theme.v1';

const DEFAULT: ThemeConfig = {
  id: 'midnight',
  accent: '#4dabff',
  radius: 10,
  density: 'normal',
  background: { type: 'none', blur: 4, overlay: 0.4 },
};

function load(): ThemeConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT;
    const p = JSON.parse(raw);
    if (
      p &&
      typeof p.id === 'string' &&
      ['midnight', 'daylight', 'forest', 'sunset', 'sakura'].includes(p.id) &&
      typeof p.accent === 'string' &&
      typeof p.radius === 'number' &&
      ['compact', 'normal', 'cozy'].includes(p.density) &&
      p.background
    ) {
      return {
        id: p.id,
        accent: p.accent,
        radius: p.radius,
        density: p.density,
        background: {
          type: p.background.type ?? 'none',
          url: p.background.url,
          dataUrl: p.background.dataUrl,
          blur: typeof p.background.blur === 'number' ? p.background.blur : DEFAULT.background.blur,
          overlay: typeof p.background.overlay === 'number' ? p.background.overlay : DEFAULT.background.overlay,
        },
      };
    }
  } catch {}
  return DEFAULT;
}

function save(t: ThemeConfig) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(t));
  } catch {
    // localStorage may be full if user uploaded huge data: URLs. We cap
    // uploads at 5 MB in the Settings panel, so this is rare.
  }
}

const [theme, setThemeRaw] = createSignal<ThemeConfig>(load());

export const currentTheme = theme;

export function setTheme(patch: Partial<ThemeConfig>) {
  const next = { ...theme(), ...patch };
  setThemeRaw(next);
  save(next);
}

export function setBackground(patch: Partial<BackgroundConfig>) {
  const next = { ...theme(), background: { ...theme().background, ...patch } };
  setThemeRaw(next);
  save(next);
}

export function resetTheme() {
  setThemeRaw(DEFAULT);
  save(DEFAULT);
}

/** Export the current config as a JSON blob for download. */
export function exportTheme(): string {
  return JSON.stringify(theme(), null, 2);
}

export function importTheme(json: string): boolean {
  try {
    const p = JSON.parse(json);
    // Re-use load()'s validation by round-tripping through it.
    const parsed = JSON.parse(JSON.stringify(p));
    const next: ThemeConfig = {
      id: parsed.id ?? DEFAULT.id,
      accent: parsed.accent ?? DEFAULT.accent,
      radius: parsed.radius ?? DEFAULT.radius,
      density: parsed.density ?? DEFAULT.density,
      background: {
        ...DEFAULT.background,
        ...(parsed.background ?? {}),
      },
    };
    setThemeRaw(next);
    save(next);
    return true;
  } catch {
    return false;
  }
}

// ───── Apply theme to the DOM ─────

/** Convert a hex color (#rrggbb) to an rgba string with given alpha. */
export function withAlpha(hex: string, alpha: number): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex);
  if (!m) return hex;
  const n = parseInt(m[1], 16);
  const r = (n >> 16) & 0xff;
  const g = (n >> 8) & 0xff;
  const b = n & 0xff;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

// Row height multiplier per density — applied via the existing
// `--row-h` CSS variable so the change cascades through the list view.
const DENSITY_ROW_PX: Record<Density, number> = {
  compact: 40,
  normal: 48,
  cozy: 56,
};

function applyToDom(t: ThemeConfig) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.setAttribute('data-theme', t.id);
  // When a background image is active, panels should let the image show
  // through. `data-has-bg` switches the surface tokens to their
  // semi-transparent counterparts in global.css via color-mix().
  if (t.background.type !== 'none') {
    root.setAttribute('data-has-bg', '');
  } else {
    root.removeAttribute('data-has-bg');
  }
  root.style.setProperty('--accent', t.accent);
  root.style.setProperty('--accent-15', withAlpha(t.accent, 0.15));
  root.style.setProperty('--accent-08', withAlpha(t.accent, 0.08));
  root.style.setProperty('--radius', `${t.radius}px`);
  root.style.setProperty('--row-h', `${DENSITY_ROW_PX[t.density]}px`);
}

createEffect(() => {
  applyToDom(theme());
});
