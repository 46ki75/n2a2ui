import type { RichTextItemResponse } from "@notionhq/client";

import type { Component, ComponentId, RichText } from "../a2ui.js";
import { childId } from "../id.js";
import { mapColor } from "./color.js";

export interface RichTextConversion {
  ids: ComponentId[];
  components: Component[];
}

type Decoration = NonNullable<RichText["decoration"]>[number];

export function convertRichTexts(
  parentId: ComponentId,
  slot: string,
  items: readonly RichTextItemResponse[],
): RichTextConversion {
  const ids: ComponentId[] = [];
  const components: Component[] = [];

  for (const [index, item] of items.entries()) {
    const id = childId(parentId, slot, index);
    ids.push(id);
    components.push(convertRichText(id, item));
  }

  return { ids, components };
}

export function richTextPlain(item: RichTextItemResponse): string {
  return item.plain_text;
}

function convertRichText(
  id: ComponentId,
  item: RichTextItemResponse,
): Component {
  if (item.type === "mention" && item.mention.type === "custom_emoji") {
    return {
      id,
      component: "Icon",
      src: item.mention.custom_emoji.url,
      alt: item.mention.custom_emoji.name,
    };
  }

  const text =
    item.type === "equation" ? item.equation.expression : item.plain_text;

  if (item.href !== null) {
    return {
      id,
      component: "LinkText",
      text,
      href: item.href,
    };
  }

  const decoration: Decoration[] = [];
  if (item.annotations.bold) decoration.push("bold");
  if (item.annotations.italic) decoration.push("italic");
  if (item.annotations.underline) decoration.push("underline");
  if (item.annotations.strikethrough) decoration.push("strikethrough");
  if (item.annotations.code) decoration.push("code");
  if (item.type === "equation") decoration.push("katex");

  const color = mapColor(item.annotations.color);
  return {
    id,
    component: "RichText",
    text,
    ...(decoration.length === 0 ? {} : { decoration }),
    ...(color === undefined ? {} : { color }),
  };
}
