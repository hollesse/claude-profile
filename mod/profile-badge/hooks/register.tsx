import type { EngineInterface, Register } from 'claude-code'

import { profileLook } from './look.js'

// Colors for profiles without a stored color; each always gets the same one based on its name.
// Must match the first entries of PALETTE in src/color.rs.
const PALETTE = ['#4c9aff', '#f5a623', '#36b37e', '#c678dd', '#ff7eb6', '#00b8d9']

const colorOf = (name: string): string => {
  let hash = 0

  for (const ch of name) {
    hash = (hash * 31 + ch.charCodeAt(0)) >>> 0
  }

  return PALETTE[hash % PALETTE.length] ?? '#4c9aff'
}

type Colors = { bg: string; fg: string }

const HEX = /^#[0-9a-f]{6}$/i

// Colors stored with `claude-profile add` or `claude-profile badge`.
const storedColors = (registry: string, name: string): Colors | null => {
  try {
    const badge = JSON.parse(registry)?.profiles?.[name]?.badge
    const isValid =
      typeof badge?.bg === 'string' &&
      HEX.test(badge.bg) &&
      typeof badge?.fg === 'string' &&
      HEX.test(badge.fg)

    return isValid ? { bg: badge.bg, fg: badge.fg } : null
  } catch {
    return null
  }
}

// Display name of the profile (e.g. `ACME` for the profile `acme`).
// In profiles.json a profile is always stored under its lowercase ID.
const storedLabel = (registry: string, name: string): string | null => {
  try {
    const label = JSON.parse(registry)?.profiles?.[name]?.name

    return typeof label === 'string' && label.toLowerCase() === name ? label : null
  } catch {
    return null
  }
}

// Profile name from a config directory ~/.claude-<profile>.
const nameOf = (dir: string | undefined): string | null =>
  dir?.match(/\/\.claude-([a-z0-9-]+)\/?$/)?.[1] ?? null

type Profile = {
  // Display name of the profile.
  label: string
  colors: Colors
  // true: the session uses the default config ~/.claude, which points to the profile via symlink
  // (started bypassing claude-profile).
  isViaLink: boolean
}

// null = no profile recognizable. Determined once per session.
let profile: Promise<Profile | null> | undefined

const findProfile = ($: EngineInterface): Promise<Profile | null> => {
  profile ??= (async (): Promise<Profile | null> => {
    const home = await $.env.get('HOME')

    const withColors = async (given: string, isViaLink: boolean): Promise<Profile> => {
      const name = given.toLowerCase()
      const fallback = { bg: colorOf(name), fg: '#000000' }

      try {
        const registry = await $.fs.read(`${home}/.config/claude-profiles/profiles.json`)

        return {
          label: storedLabel(registry, name) ?? name,
          isViaLink,
          colors: storedColors(registry, name) ?? fallback,
        }
      } catch {
        return { label: name, isViaLink, colors: fallback }
      }
    }

    // claude-profile sets CLAUDE_PROFILE and CLAUDE_CONFIG_DIR on start.
    const named = await $.env.get('CLAUDE_PROFILE')
    const configDir = await $.env.get('CLAUDE_CONFIG_DIR')
    const direct = named || nameOf(configDir)

    if (direct) {
      return withColors(direct, false)
    }

    if (configDir) {
      return null
    }

    // Without CLAUDE_CONFIG_DIR, ~/.claude applies: where does the symlink point?
    try {
      const stat = await $.fs.stat(`${home}/.claude`, { resolve: true })
      const linked = nameOf(stat.realPath)

      return linked ? withColors(linked, true) : null
    } catch {
      return null
    }
  })()

  return profile
}

export const register: Register = on => {
  // Whether the session got the profile's look (only the desktop and mobile apps draw it).
  let hasLook = false

  // Desktop: tint the whole window in the profile color.
  on('session.start', async ($, e, next) => {
    try {
      const found = await findProfile($)
      const look = found === null ? null : profileLook(found.label, found.colors.bg)

      if (look !== null) {
        await $.theme.set(look)
        hasLook = true
      }
    } catch {
      // The look is a convenience: without it the badge remains.
    }

    return next(e)
  })

  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => {
    if (e.props.hasSurvey) {
      return next(e)
    }

    // In the desktop app the window color shows the profile; the badge is left to the terminal.
    if (e.surface === 'desktop' && hasLook) {
      return next(e)
    }

    const found = await findProfile($)
    const { Box, Text } = $.ui.resolve(e)

    if (found === null) {
      return (
        <Box>
          <Text color="warning">▲ no profile – default config (~/.claude)</Text>
        </Box>
      )
    }

    return (
      <Box>
        <Text backgroundColor={found.colors.bg} color={found.colors.fg} bold>
          {` ${found.label} `}
        </Text>
        {found.isViaLink ? <Text dimColor> via ~/.claude</Text> : null}
      </Box>
    )
  })
}
