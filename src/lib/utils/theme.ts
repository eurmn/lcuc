export interface RuneTheme {
  styleId: number;
  name: string;
  textAccent: string;
  textDim: string;
  borderPrimary: string;
  borderSubtle: string;
  borderHover: string;
  ringActive: string;
  ringFocus: string;
  bgTint: string;
  bgCard: string;
  bgBadge: string;
  badgeClasses: string;
  iconBoxClasses: string;
  activeBar: string;
  shadowGlow: string;
  cardGradient: string;
}

export const THEMES: Record<number, RuneTheme> = {
  8000: {
    styleId: 8000,
    name: "Precision",
    textAccent: "text-yellow-400",
    textDim: "text-yellow-300",
    borderPrimary: "border-yellow-500",
    borderSubtle: "border-yellow-500/30",
    borderHover: "hover:border-yellow-400",
    ringActive: "ring-yellow-400",
    ringFocus: "ring-yellow-400/80",
    bgTint: "bg-yellow-500/10",
    bgCard: "bg-yellow-950/40",
    bgBadge: "bg-yellow-400 text-gray-950",
    badgeClasses: "border border-yellow-500/40 bg-yellow-500/10 text-yellow-300",
    iconBoxClasses: "border-yellow-500/40 bg-yellow-950/30",
    activeBar: "bg-yellow-400 shadow-yellow-400",
    shadowGlow: "shadow-yellow-950/50",
    cardGradient: "from-yellow-950/30 via-gray-900/90 to-gray-900",
  },
  8100: {
    styleId: 8100,
    name: "Domination",
    textAccent: "text-red-400",
    textDim: "text-red-300",
    borderPrimary: "border-red-500",
    borderSubtle: "border-red-500/30",
    borderHover: "hover:border-red-400",
    ringActive: "ring-red-500",
    ringFocus: "ring-red-400/80",
    bgTint: "bg-red-500/10",
    bgCard: "bg-red-950/40",
    bgBadge: "bg-red-500 text-white",
    badgeClasses: "border border-red-500/40 bg-red-500/10 text-red-300",
    iconBoxClasses: "border-red-500/40 bg-red-950/30",
    activeBar: "bg-red-500 shadow-red-500",
    shadowGlow: "shadow-red-950/50",
    cardGradient: "from-red-950/30 via-gray-900/90 to-gray-900",
  },
  8200: {
    styleId: 8200,
    name: "Sorcery",
    textAccent: "text-purple-400",
    textDim: "text-purple-300",
    borderPrimary: "border-purple-500",
    borderSubtle: "border-purple-500/30",
    borderHover: "hover:border-purple-400",
    ringActive: "ring-purple-400",
    ringFocus: "ring-purple-400/80",
    bgTint: "bg-purple-500/10",
    bgCard: "bg-purple-950/40",
    bgBadge: "bg-purple-500 text-white",
    badgeClasses: "border border-purple-500/40 bg-purple-500/10 text-purple-300",
    iconBoxClasses: "border-purple-500/40 bg-purple-950/30",
    activeBar: "bg-purple-400 shadow-purple-400",
    shadowGlow: "shadow-purple-950/50",
    cardGradient: "from-purple-950/30 via-gray-900/90 to-gray-900",
  },
  8300: {
    styleId: 8300,
    name: "Inspiration",
    textAccent: "text-cyan-400",
    textDim: "text-cyan-300",
    borderPrimary: "border-cyan-400",
    borderSubtle: "border-cyan-500/30",
    borderHover: "hover:border-cyan-400",
    ringActive: "ring-cyan-400",
    ringFocus: "ring-cyan-400/80",
    bgTint: "bg-cyan-500/10",
    bgCard: "bg-cyan-950/40",
    bgBadge: "bg-cyan-400 text-gray-950",
    badgeClasses: "border border-cyan-500/40 bg-cyan-500/10 text-cyan-300",
    iconBoxClasses: "border-cyan-500/40 bg-cyan-950/30",
    activeBar: "bg-cyan-400 shadow-cyan-400",
    shadowGlow: "shadow-cyan-950/50",
    cardGradient: "from-cyan-950/30 via-gray-900/90 to-gray-900",
  },
  8400: {
    styleId: 8400,
    name: "Resolve",
    textAccent: "text-emerald-400",
    textDim: "text-emerald-300",
    borderPrimary: "border-emerald-500",
    borderSubtle: "border-emerald-500/30",
    borderHover: "hover:border-emerald-400",
    ringActive: "ring-emerald-400",
    ringFocus: "ring-emerald-400/80",
    bgTint: "bg-emerald-500/10",
    bgCard: "bg-emerald-950/40",
    bgBadge: "bg-emerald-500 text-white",
    badgeClasses: "border border-emerald-500/40 bg-emerald-500/10 text-emerald-300",
    iconBoxClasses: "border-emerald-500/40 bg-emerald-950/30",
    activeBar: "bg-emerald-400 shadow-emerald-400",
    shadowGlow: "shadow-emerald-950/50",
    cardGradient: "from-emerald-950/30 via-gray-900/90 to-gray-900",
  },
};

const DEFAULT_THEME: RuneTheme = {
  styleId: 0,
  name: "Default",
  textAccent: "text-purple-400",
  textDim: "text-purple-300",
  borderPrimary: "border-purple-500",
  borderSubtle: "border-purple-500/30",
  borderHover: "hover:border-purple-400",
  ringActive: "ring-purple-400",
  ringFocus: "ring-purple-400/80",
  bgTint: "bg-purple-500/10",
  bgCard: "bg-purple-950/40",
  bgBadge: "bg-purple-500 text-white",
  badgeClasses: "border border-purple-500/40 bg-purple-500/10 text-purple-300",
  iconBoxClasses: "border-purple-500/40 bg-purple-950/30",
  activeBar: "bg-purple-400 shadow-purple-400",
  shadowGlow: "shadow-purple-950/50",
  cardGradient: "from-purple-950/30 via-gray-900/90 to-gray-900",
};

export function getRuneTheme(styleId: number | null | undefined): RuneTheme {
  if (!styleId) return DEFAULT_THEME;
  return THEMES[styleId] || DEFAULT_THEME;
}

export const RUNE_PATH_ROW_NAMES: Record<number, string[]> = {
  8000: ["Heroism", "Legend", "Combat"],
  8100: ["Malice", "Tracking", "Hunter"],
  8200: ["Artifact", "Excellence", "Power"],
  8300: ["Contraptions", "Tomorrow", "Beyond"],
  8400: ["Strength", "Resistance", "Vitality"],
};

export function getStyleRowName(styleId: number, rowIndex: number): string {
  const names = RUNE_PATH_ROW_NAMES[styleId];
  if (names && names[rowIndex]) {
    return names[rowIndex];
  }
  return `Tier ${rowIndex + 1}`;
}

