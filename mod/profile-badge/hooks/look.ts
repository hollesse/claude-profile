import type { ThemeSetArgs } from 'claude-code'

import { makeColors } from './make-colors.js'

// Look of the desktop app for a profile: all surfaces tinted in the badge color, with a
// gradient behind them that starts in the full color at the top. Terminal and editors draw no
// look – there the badge above the prompt remains.

type Rgb = [number, number, number]

const rgbOf = (hex: string): Rgb => {
  const value = Number.parseInt(hex.slice(1), 16)

  return [(value >> 16) & 255, (value >> 8) & 255, value & 255]
}

const hexOf = (rgb: Rgb): string =>
  `#${rgb.map(channel => Math.round(channel).toString(16).padStart(2, '0')).join('')}`

// Mixes `amount` (0 to 1) of `tint` into `base`.
const mix = (base: string, tint: string, amount: number): string => {
  const from = rgbOf(base)
  const to = rgbOf(tint)

  return hexOf([0, 1, 2].map(i => from[i]! + (to[i]! - from[i]!) * amount) as Rgb)
}

// How strongly the profile color tints the neutral surfaces.
const LIGHT_TINT = 0.2
const DARK_TINT = 0.16

// The lines that would make the desktop app not paint the whole color scheme.
export const unpainted = (problems: readonly string[]): string[] =>
  problems.filter(line => line.startsWith("Desktop won't paint"))

// Color scheme with the profile color at strength `strength` (1 = full tint).
const tinted = (label: string, color: string, strength: number) =>
  makeColors({
    label: `Profile ${label}`,
    roles: { accent: color },
    light: {
      gray: {
        anchors: {
          'surface-0': mix('#eeece4', color, LIGHT_TINT * strength),
          'surface-1': mix('#f5f4ee', color, LIGHT_TINT * strength * 0.7),
          'surface-2': mix('#fcfbf8', color, LIGHT_TINT * strength * 0.4),
          'text-primary': mix('#161614', color, 0.08),
          'text-secondary': mix('#3b3a37', color, 0.08),
          'text-muted': mix('#56554f', color, 0.08),
        },
      },
    },
    dark: {
      gray: {
        anchors: {
          'surface-0': mix('#151514', color, DARK_TINT * strength),
          'surface-1': mix('#1d1d1b', color, DARK_TINT * strength),
          'surface-2': mix('#252523', color, DARK_TINT * strength),
          'surface-3': mix('#2b2b29', color, DARK_TINT * strength),
          'text-primary': mix('#f4f3ee', color, 0.06),
          'text-secondary': mix('#d6d4ca', color, 0.06),
          'text-muted': mix('#bab8ae', color, 0.06),
        },
      },
    },
  })

// The look for a profile with the badge color `color` (`#rrggbb`); null if the color
// yields none that the desktop app paints.
export const profileLook = (label: string, color: string): ThemeSetArgs | null => {
  // Very light colors make the dark surfaces too light for the text – tint more weakly then.
  for (const strength of [1, 0.6, 0.3]) {
    try {
      const { theme, problems } = tinted(label, color, strength)

      if (unpainted(problems).length > 0) {
        continue
      }

      return {
        colors: theme,
        // The profile color at the top, fading into the page color towards the bottom.
        backdrop: {
          gradient: {
            stops: [
              { at: 0, color: color as `#${string}` },
              { at: 0.45, color: 'surface-0' },
              { at: 1, color: 'surface-0' },
            ],
          },
        },
      }
    } catch {
      // This strength yields no valid color scheme.
    }
  }

  return null
}
