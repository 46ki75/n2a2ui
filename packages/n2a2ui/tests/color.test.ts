import type { ApiColor } from "@notionhq/client";
import { describe, expect, it } from "vitest";

import { mapBackgroundColor, mapColor } from "../src/convert/color.js";

const foregroundCases = [
  ["blue", "#6987b8"],
  ["brown", "#8b4c3f"],
  ["gray", "#868e9c"],
  ["green", "#59b57c"],
  ["orange", "#bf7e71"],
  ["pink", "#c9699e"],
  ["purple", "#9771bd"],
  ["red", "#b36472"],
  ["yellow", "#b8a36e"],
] as const satisfies ReadonlyArray<readonly [ApiColor, string]>;

const backgroundCases = [
  ["blue_background", "#c0cce1"],
  ["brown_background", "#d0bdac"],
  ["gray_background", "#cccfd5"],
  ["green_background", "#b1dcc2"],
  ["orange_background", "#f1dbd2"],
  ["pink_background", "#ebc7db"],
  ["purple_background", "#d7c8e5"],
  ["red_background", "#e8c2c2"],
  ["yellow_background", "#f0e9d7"],
] as const satisfies ReadonlyArray<readonly [ApiColor, string]>;

describe("mapColor", () => {
  it.each(foregroundCases)("maps %s", (color, expected) => {
    expect(mapColor(color)).toBe(expected);
  });

  it.each(backgroundCases)("does not map %s", (color) => {
    expect(mapColor(color)).toBeUndefined();
  });

  it.each(["default", "default_background"] as const)(
    "does not map %s",
    (color) => {
      expect(mapColor(color)).toBeUndefined();
    },
  );
});

describe("mapBackgroundColor", () => {
  it.each(backgroundCases)("maps %s", (color, expected) => {
    expect(mapBackgroundColor(color)).toBe(expected);
  });

  it.each(foregroundCases)("does not map %s", (color) => {
    expect(mapBackgroundColor(color)).toBeUndefined();
  });

  it.each(["default", "default_background"] as const)(
    "does not map %s",
    (color) => {
      expect(mapBackgroundColor(color)).toBeUndefined();
    },
  );
});
