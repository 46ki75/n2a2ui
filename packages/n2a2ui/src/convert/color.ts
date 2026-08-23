import type { ApiColor } from "@notionhq/client";

const foregroundColors: Partial<Record<ApiColor, string>> = {
  blue: "#6987b8",
  brown: "#8b4c3f",
  gray: "#868e9c",
  green: "#59b57c",
  orange: "#bf7e71",
  pink: "#c9699e",
  purple: "#9771bd",
  red: "#b36472",
  yellow: "#b8a36e",
};

const backgroundColors: Partial<Record<ApiColor, string>> = {
  blue_background: "#c0cce1",
  brown_background: "#d0bdac",
  gray_background: "#cccfd5",
  green_background: "#b1dcc2",
  orange_background: "#f1dbd2",
  pink_background: "#ebc7db",
  purple_background: "#d7c8e5",
  red_background: "#e8c2c2",
  yellow_background: "#f0e9d7",
};

export function mapColor(color: ApiColor): string | undefined {
  return foregroundColors[color];
}

export function mapBackgroundColor(color: ApiColor): string | undefined {
  return backgroundColors[color];
}
