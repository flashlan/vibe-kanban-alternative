// Predefined color palette for projects and tags (HSL format)
// Modern, vibrant colors with good differentiation
export const PRESET_COLORS = [
  '0 84% 60%', // Coral Red - vibrant, warm
  '24 95% 53%', // Tangerine - energetic orange
  '45 93% 58%', // Golden Yellow - bright, optimistic
  '158 64% 52%', // Mint Green - fresh, modern
  '200 98% 39%', // Ocean Blue - professional, calm
  '271 81% 56%', // Vivid Purple - creative, modern
  '330 81% 60%', // Hot Pink - bold, playful
  '183 74% 44%', // Teal - sophisticated
  '262 52% 47%', // Indigo - deep, elegant
  '142 71% 45%', // Emerald - nature, growth
  '17 88% 40%', // Rust - warm, earthy
  '231 48% 48%', // Slate Blue - professional
] as const;

export type PresetColor = (typeof PRESET_COLORS)[number];

/**
 * High-luminance palette tuned for the dark theme — every swatch reads as
 * row tint/text on dark backgrounds and stays distinguishable on light.
 * HSL triples matching the project-tint format (`hsl(H S% L% / alpha)`).
 * Used by the project/workspace color editors where the operator re-picks
 * colors that look wrong on dark backgrounds.
 */
export const DARK_THEME_PRESET_COLORS = [
  '8 90% 72%', // Coral
  '25 100% 70%', // Apricot
  '45 100% 66%', // Amber
  '90 65% 62%', // Pistachio
  '152 62% 60%', // Mint
  '174 62% 56%', // Teal
  '197 88% 64%', // Sky
  '222 85% 70%', // Periwinkle
  '262 88% 72%', // Lavender
  '292 78% 68%', // Orchid
  '328 88% 70%', // Rose
  '348 85% 66%', // Raspberry
] as const;

/**
 * Get a random color from the preset palette
 */
export function getRandomPresetColor(): string {
  return PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)];
}
