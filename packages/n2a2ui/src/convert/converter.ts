import {
  collectPaginatedAPI,
  isFullBlock,
  type ApiColor,
  type BlockObjectResponse,
  type CalloutBlockObjectResponse,
  type Client,
  type PartialBlockObjectResponse,
  type RichTextItemResponse,
  type TableRowBlockObjectResponse,
} from "@notionhq/client";

import type { Component, ComponentId, NotionCallout } from "../a2ui.js";
import { childId } from "../id.js";
import { mapBackgroundColor, mapColor } from "./color.js";
import { fetchBookmarkMetadata, fetchImageDimensions } from "./metadata.js";
import { convertRichTexts, richTextPlain } from "./rich-text.js";

export type NotionBlock = BlockObjectResponse | PartialBlockObjectResponse;
export type ListStyle = "unordered" | "ordered";
export type ConversionResult = [ComponentId[], Component[]];
export type ConversionChunk = [ComponentId, Component[]];

export type SiblingGroup =
  | { kind: "single"; block: NotionBlock }
  | { kind: "list"; blocks: NotionBlock[]; style: ListStyle };

export interface ConverterOptions {
  notion: Client;
  fetch: typeof globalThis.fetch;
  enableUnsupportedBlock: boolean;
  enableFetchImageMeta: boolean;
  enableFetchBookmarkMeta: boolean;
  enableHtmlEmbed: boolean;
}

type CalloutIcon = NonNullable<CalloutBlockObjectResponse["callout"]["icon"]>;
type NotionCalloutIcon = NonNullable<NotionCallout["icon"]>;
type NotionCalloutColor = NonNullable<NotionCallout["color"]>;
type NotionCalloutVariant = NonNullable<NotionCallout["variant"]>;

export function listStyle(block: NotionBlock): ListStyle | undefined {
  if (!("type" in block)) return undefined;
  if (block.type === "bulleted_list_item" || block.type === "to_do") {
    return "unordered";
  }
  return block.type === "numbered_list_item" ? "ordered" : undefined;
}

export function topLevelGroups(blocks: readonly NotionBlock[]): SiblingGroup[] {
  const groups: SiblingGroup[] = [];
  let index = 0;

  while (index < blocks.length) {
    const block = blocks[index];
    if (block === undefined) break;
    const style = listStyle(block);
    if (style === undefined) {
      groups.push({ kind: "single", block });
      index += 1;
      continue;
    }

    let end = index + 1;
    while (end < blocks.length && listStyle(blocks[end]!) === style) {
      end += 1;
    }
    groups.push({ kind: "list", blocks: blocks.slice(index, end), style });
    index = end;
  }

  return groups;
}

export function isHtmlFileUrl(url: string): boolean {
  const [path = url] = url.split(/[?#]/, 1);
  return path.toLowerCase().endsWith(".html");
}

export class Converter {
  readonly notion: Client;
  readonly fetch: typeof globalThis.fetch;
  readonly enableUnsupportedBlock: boolean;
  readonly enableFetchImageMeta: boolean;
  readonly enableFetchBookmarkMeta: boolean;
  readonly enableHtmlEmbed: boolean;

  constructor(options: ConverterOptions) {
    this.notion = options.notion;
    this.fetch = options.fetch;
    this.enableUnsupportedBlock = options.enableUnsupportedBlock;
    this.enableFetchImageMeta = options.enableFetchImageMeta;
    this.enableFetchBookmarkMeta = options.enableFetchBookmarkMeta;
    this.enableHtmlEmbed = options.enableHtmlEmbed;
  }

  async convertChildren(parentId: ComponentId): Promise<ConversionResult> {
    const blocks = await this.fetchChildren(parentId);
    const components: Component[] = [];
    const ids = await this.convertSiblings(blocks, components);
    return [ids, components];
  }

  async fetchChildren(parentId: ComponentId): Promise<NotionBlock[]> {
    return collectPaginatedAPI(this.notion.blocks.children.list, {
      block_id: parentId,
    });
  }

  async convertSingleBlockToChunk(
    block: NotionBlock,
  ): Promise<ConversionChunk | undefined> {
    const components: Component[] = [];
    const id = await this.convertBlock(block, components);
    return id === undefined ? undefined : [id, components];
  }

  async convertListGroupToChunk(
    items: readonly NotionBlock[],
    style: ListStyle,
  ): Promise<ConversionChunk> {
    const components: Component[] = [];
    const id = await this.emitList(items, style, components);
    return [id, components];
  }

  async convertSiblings(
    blocks: readonly NotionBlock[],
    bag: Component[],
  ): Promise<ComponentId[]> {
    const ids: ComponentId[] = [];
    for (const group of topLevelGroups(blocks)) {
      if (group.kind === "list") {
        ids.push(await this.emitList(group.blocks, group.style, bag));
        continue;
      }
      const id = await this.convertBlock(group.block, bag);
      if (id !== undefined) ids.push(id);
    }
    return ids;
  }

  classifyTableRows(
    rows: readonly NotionBlock[],
    hasColumnHeader: boolean,
    hasRowHeader: boolean,
    bag: Component[],
  ): [ComponentId[], ComponentId[]] {
    const tableRows = rows.filter(
      (row): row is TableRowBlockObjectResponse =>
        isFullBlock(row) && row.type === "table_row",
    );
    const headerIds: ComponentId[] = [];
    const bodyIds: ComponentId[] = [];

    for (const [rowIndex, row] of tableRows.entries()) {
      const cellIds = this.tableRowCells(
        row.id,
        row.table_row.cells,
        hasRowHeader,
        bag,
      );
      bag.push({
        id: row.id,
        component: "TableRow",
        children: cellIds,
      });
      if (hasColumnHeader && rowIndex === 0) headerIds.push(row.id);
      else bodyIds.push(row.id);
    }

    return [headerIds, bodyIds];
  }

  private async convertBlock(
    notion: NotionBlock,
    bag: Component[],
  ): Promise<ComponentId | undefined> {
    const id = notion.id;
    if (!isFullBlock(notion)) {
      if (!this.enableUnsupportedBlock) return undefined;
      bag.push(unsupported(id, "partial_block"));
      return id;
    }

    let component: Component;
    switch (notion.type) {
      case "paragraph":
        component = await this.paragraph(
          id,
          notion.paragraph.rich_text,
          notion.paragraph.color,
          notion.has_children,
          bag,
        );
        break;
      case "heading_1":
        component = this.heading(id, 1, notion.heading_1.rich_text, bag);
        break;
      case "heading_2":
        component = this.heading(id, 2, notion.heading_2.rich_text, bag);
        break;
      case "heading_3":
        component = this.heading(id, 3, notion.heading_3.rich_text, bag);
        break;
      case "heading_4":
        component = this.heading(id, 4, notion.heading_4.rich_text, bag);
        break;
      case "quote":
        component = await this.quote(
          id,
          notion.quote.rich_text,
          notion.has_children,
          bag,
        );
        break;
      case "callout":
        component = await this.callout(
          id,
          notion.callout,
          notion.has_children,
          bag,
        );
        break;
      case "toggle":
        component = await this.toggle(
          id,
          notion.toggle.rich_text,
          notion.has_children,
          bag,
        );
        break;
      case "divider":
        component = { id, component: "Divider" };
        break;
      case "code": {
        const code = notion.code.rich_text.map(richTextPlain).join("");
        if (notion.code.language === "mermaid") {
          component = { id, component: "Mermaid", code };
        } else {
          component = {
            id,
            component: "CodeBlock",
            code,
            language: notion.code.language,
            ...(notion.code.caption.length === 0
              ? {}
              : {
                  caption: notion.code.caption.map(richTextPlain).join(""),
                }),
          };
        }
        break;
      }
      case "equation":
        component = {
          id,
          component: "Katex",
          expression: notion.equation.expression,
        };
        break;
      case "image":
        component = await this.image(id, notion.image);
        break;
      case "file":
        component = fileComponent(id, notion.file, "file");
        break;
      case "pdf":
        component = fileComponent(id, notion.pdf, "file");
        break;
      case "audio":
        component = audioComponent(id, notion.audio);
        break;
      case "video":
        component = videoComponent(id, notion.video);
        break;
      case "bookmark":
        component = await this.bookmarkFromUrl(id, notion.bookmark.url);
        break;
      case "embed":
        component = await this.embedFromUrl(id, notion.embed.url);
        break;
      case "link_preview":
        component = await this.bookmarkFromUrl(id, notion.link_preview.url);
        break;
      case "child_page":
        component = {
          id,
          component: "Bookmark",
          url: notionPageUrl(id),
          title: notion.child_page.title,
        };
        break;
      case "child_database":
        component = {
          id,
          component: "Bookmark",
          url: notionPageUrl(id),
          title: notion.child_database.title,
        };
        break;
      case "column_list":
        component = await this.columnList(id, notion.has_children, bag);
        break;
      case "column": {
        let children: ComponentId[] = [];
        if (notion.has_children) {
          const [ids, components] = await this.convertChildren(id);
          bag.push(...components);
          children = ids;
        }
        component = {
          id,
          component: "Column",
          children,
          ...(notion.column.width_ratio === undefined
            ? {}
            : { widthRatio: notion.column.width_ratio }),
        };
        break;
      }
      case "table":
        component = await this.table(
          id,
          notion.table.has_column_header,
          notion.table.has_row_header,
          bag,
        );
        break;
      case "tab":
        component = await this.tab(id, notion.has_children, bag);
        break;
      case "synced_block": {
        let children: ComponentId[] = [];
        if (notion.has_children) {
          const [ids, components] = await this.convertChildren(id);
          bag.push(...components);
          children = ids;
        }
        component = { id, component: "Column", children };
        break;
      }
      default:
        if (!this.enableUnsupportedBlock) return undefined;
        component = unsupported(id, unsupportedLabel(notion));
        break;
    }

    bag.push(component);
    return id;
  }

  private async paragraph(
    id: ComponentId,
    richText: readonly RichTextItemResponse[],
    notionColor: ApiColor,
    hasChildren: boolean,
    bag: Component[],
  ): Promise<Component> {
    const converted = convertRichTexts(id, "rich_text", richText);
    bag.push(...converted.components);
    const children = [...converted.ids];
    if (hasChildren) {
      const [ids, components] = await this.convertChildren(id);
      bag.push(...components);
      children.push(...ids);
    }
    const color = mapColor(notionColor);
    const backgroundColor = mapBackgroundColor(notionColor);
    return {
      id,
      component: "Paragraph",
      children,
      ...(color === undefined ? {} : { color }),
      ...(backgroundColor === undefined ? {} : { backgroundColor }),
    };
  }

  private heading(
    id: ComponentId,
    level: 1 | 2 | 3 | 4,
    richText: readonly RichTextItemResponse[],
    bag: Component[],
  ): Component {
    const converted = convertRichTexts(id, "rich_text", richText);
    bag.push(...converted.components);
    return { id, component: "Heading", level, children: converted.ids };
  }

  private async quote(
    id: ComponentId,
    richText: readonly RichTextItemResponse[],
    hasChildren: boolean,
    bag: Component[],
  ): Promise<Component> {
    const converted = convertRichTexts(id, "rich_text", richText);
    bag.push(...converted.components);
    const children = [...converted.ids];
    if (hasChildren) {
      const [ids, components] = await this.convertChildren(id);
      bag.push(...components);
      children.push(...ids);
    }
    return { id, component: "BlockQuote", children };
  }

  private async callout(
    id: ComponentId,
    callout: CalloutBlockObjectResponse["callout"],
    hasChildren: boolean,
    bag: Component[],
  ): Promise<Component> {
    const converted = convertRichTexts(id, "rich_text", callout.rich_text);
    bag.push(...converted.components);
    const children = [...converted.ids];
    if (hasChildren) {
      const [ids, components] = await this.convertChildren(id);
      bag.push(...components);
      children.push(...ids);
    }
    const [color, variant] = notionCalloutColorAndVariant(callout.color);
    const icon = notionCalloutIcon(callout.icon);
    return {
      id,
      component: "NotionCallout",
      children,
      color,
      ...(icon === undefined ? {} : { icon }),
      ...(variant === undefined ? {} : { variant }),
    };
  }

  private async toggle(
    id: ComponentId,
    richText: readonly RichTextItemResponse[],
    hasChildren: boolean,
    bag: Component[],
  ): Promise<Component> {
    const summary = convertRichTexts(id, "summary", richText);
    bag.push(...summary.components);
    let children: ComponentId[] = [];
    if (hasChildren) {
      const [ids, components] = await this.convertChildren(id);
      bag.push(...components);
      children = ids;
    }
    return {
      id,
      component: "Toggle",
      summary: summary.ids,
      children,
    };
  }

  private async image(id: ComponentId, media: unknown): Promise<Component> {
    const src = fileUrl(media);
    if (src === undefined) return unsupported(id, "image:api_uploaded");

    const dimensions = this.enableFetchImageMeta
      ? await fetchImageDimensions(this.fetch, src)
      : {};
    const name = fileName(media);
    const caption = mediaCaption(media);
    return {
      id,
      component: "BlockImage",
      src,
      ...(name === undefined ? {} : { alt: name }),
      ...(dimensions.width === undefined ? {} : { width: dimensions.width }),
      ...(dimensions.height === undefined ? {} : { height: dimensions.height }),
      ...(caption === undefined ? {} : { caption }),
    };
  }

  private async bookmarkFromUrl(
    id: ComponentId,
    url: string,
  ): Promise<Component> {
    const metadata = this.enableFetchBookmarkMeta
      ? await fetchBookmarkMetadata(this.fetch, url)
      : {};
    return { id, component: "Bookmark", url, ...metadata };
  }

  private async embedFromUrl(id: ComponentId, url: string): Promise<Component> {
    if (this.enableHtmlEmbed && isHtmlFileUrl(url)) {
      return { id, component: "Html", src: url };
    }
    return this.bookmarkFromUrl(id, url);
  }

  private async columnList(
    id: ComponentId,
    hasChildren: boolean,
    bag: Component[],
  ): Promise<Component> {
    let children: ComponentId[] = [];
    if (hasChildren) {
      const [ids, components] = await this.convertChildren(id);
      bag.push(...components);
      children = ids;
    }
    return { id, component: "ColumnList", children };
  }

  private async table(
    id: ComponentId,
    hasColumnHeader: boolean,
    hasRowHeader: boolean,
    bag: Component[],
  ): Promise<Component> {
    const rows = await this.fetchChildren(id);
    const [header, body] = this.classifyTableRows(
      rows,
      hasColumnHeader,
      hasRowHeader,
      bag,
    );
    return {
      id,
      component: "Table",
      body,
      ...(header.length === 0 ? {} : { header }),
      hasColumnHeader,
      hasRowHeader,
    };
  }

  private tableRowCells(
    rowId: ComponentId,
    cells: readonly (readonly RichTextItemResponse[])[],
    hasRowHeader: boolean,
    bag: Component[],
  ): ComponentId[] {
    const cellIds: ComponentId[] = [];
    for (const [columnIndex, cell] of cells.entries()) {
      const cellId = childId(rowId, "cell", columnIndex);
      const converted = convertRichTexts(cellId, "rich_text", cell);
      bag.push(...converted.components);
      bag.push({
        id: cellId,
        component: "TableCell",
        children: converted.ids,
        ...(hasRowHeader && columnIndex === 0 ? { isHeader: true } : {}),
      });
      cellIds.push(cellId);
    }
    return cellIds;
  }

  private async tab(
    id: ComponentId,
    hasChildren: boolean,
    bag: Component[],
  ): Promise<Component> {
    const tabIds: ComponentId[] = [];
    if (hasChildren) {
      const children = await this.fetchChildren(id);
      for (const child of children) {
        if (!isFullBlock(child) || child.type !== "paragraph") continue;
        const label = convertRichTexts(
          child.id,
          "label",
          child.paragraph.rich_text,
        );
        bag.push(...label.components);
        let content: ComponentId[] = [];
        if (child.has_children) {
          const [ids, components] = await this.convertChildren(child.id);
          bag.push(...components);
          content = ids;
        }
        bag.push({
          id: child.id,
          component: "ContentTab",
          label: label.ids,
          content,
        });
        tabIds.push(child.id);
      }
    }
    return { id, component: "ContentTabs", children: tabIds };
  }

  private async emitList(
    items: readonly NotionBlock[],
    style: ListStyle,
    bag: Component[],
  ): Promise<ComponentId> {
    const first = items[0];
    if (first === undefined) {
      throw new Error("cannot convert an empty list group");
    }
    const listId: ComponentId = `${first.id}::list`;
    const children: ComponentId[] = [];
    for (const item of items) {
      children.push(await this.emitListItem(item, bag));
    }
    bag.push({ id: listId, component: "List", children, style });
    return listId;
  }

  private async emitListItem(
    item: NotionBlock,
    bag: Component[],
  ): Promise<ComponentId> {
    if (!isFullBlock(item)) {
      throw new Error("list group contains a partial block");
    }

    let richText: readonly RichTextItemResponse[];
    let todoMark: string | undefined;
    switch (item.type) {
      case "bulleted_list_item":
        richText = item.bulleted_list_item.rich_text;
        break;
      case "numbered_list_item":
        richText = item.numbered_list_item.rich_text;
        break;
      case "to_do":
        richText = item.to_do.rich_text;
        todoMark = item.to_do.checked ? "☑ " : "☐ ";
        break;
      default:
        throw new Error("list group contains a non-list block");
    }

    const children: ComponentId[] = [];
    if (todoMark !== undefined) {
      const markId = childId(item.id, "todo_mark", 0);
      bag.push({
        id: markId,
        component: "RichText",
        text: todoMark,
      });
      children.push(markId);
    }

    const converted = convertRichTexts(item.id, "rich_text", richText);
    bag.push(...converted.components);
    children.push(...converted.ids);
    if (item.has_children) {
      const [ids, components] = await this.convertChildren(item.id);
      bag.push(...components);
      children.push(...ids);
    }
    bag.push({ id: item.id, component: "ListItem", children });
    return item.id;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function fileUrl(media: unknown): string | undefined {
  if (!isRecord(media) || typeof media.type !== "string") return undefined;
  if (media.type !== "external" && media.type !== "file") return undefined;
  const value = media[media.type];
  return isRecord(value) && typeof value.url === "string"
    ? value.url
    : undefined;
}

function fileName(media: unknown): string | undefined {
  return isRecord(media) && typeof media.name === "string"
    ? media.name
    : undefined;
}

function mediaCaption(media: unknown): string | undefined {
  if (!isRecord(media) || !Array.isArray(media.caption)) return undefined;
  return (media.caption as RichTextItemResponse[]).map(richTextPlain).join("");
}

function fileComponent(
  id: ComponentId,
  media: unknown,
  label: "file",
): Component {
  const src = fileUrl(media);
  if (src === undefined) return unsupported(id, `${label}:api_uploaded`);
  const name = fileName(media);
  return {
    id,
    component: "File",
    src,
    ...(name === undefined ? {} : { name }),
  };
}

function audioComponent(id: ComponentId, media: unknown): Component {
  const src = fileUrl(media);
  if (src === undefined) return unsupported(id, "audio:api_uploaded");
  const title = fileName(media);
  return {
    id,
    component: "Audio",
    src,
    ...(title === undefined ? {} : { title }),
  };
}

function videoComponent(id: ComponentId, media: unknown): Component {
  const src = fileUrl(media);
  if (src === undefined) return unsupported(id, "video:api_uploaded");
  const title = fileName(media);
  const caption = mediaCaption(media);
  return {
    id,
    component: "Video",
    src,
    ...(title === undefined ? {} : { title }),
    ...(caption === undefined ? {} : { caption }),
  };
}

function notionPageUrl(blockId: string): string {
  return `https://www.notion.so/${blockId.replaceAll("-", "")}`;
}

function notionCalloutIcon(
  icon: CalloutIcon | null,
): NotionCalloutIcon | undefined {
  if (icon === null) return undefined;
  if (icon.type === "emoji") {
    return { kind: "emoji", emoji: icon.emoji };
  }
  if (icon.type === "custom_emoji") {
    return {
      kind: "image",
      src: icon.custom_emoji.url,
      alt: icon.custom_emoji.name,
    };
  }
  if (icon.type === "icon") {
    const runtimeColor = (icon.icon as { color: string }).color;
    const color = runtimeColor === "light_gray" ? "lightgray" : runtimeColor;
    return {
      kind: "image",
      src: `https://app.notion.com/icons/${icon.icon.name}_${color}.svg`,
      alt: icon.icon.name,
    };
  }
  const src = fileUrl(icon);
  return src === undefined ? undefined : { kind: "image", src };
}

function notionCalloutColorAndVariant(
  color: ApiColor,
): [NotionCalloutColor, NotionCalloutVariant | undefined] {
  const filled = color.endsWith("_background");
  const base = color.replace(/_background$/, "");
  const mapped = (
    {
      default: "default",
      blue: "blue",
      brown: "gray",
      gray: "gray",
      green: "green",
      orange: "orange",
      pink: "magenta",
      purple: "purple",
      red: "red",
      yellow: "yellow",
    } satisfies Record<string, NotionCalloutColor>
  )[base];
  if (mapped === undefined) return ["default", undefined];
  if (color === "default") return [mapped, undefined];
  return [mapped, filled ? "filled" : "outlined"];
}

function unsupported(id: ComponentId, details: string): Component {
  return { id, component: "Unsupported", details };
}

function unsupportedLabel(block: BlockObjectResponse): string {
  return block.type === "unsupported"
    ? `unsupported:${block.unsupported.block_type}`
    : block.type;
}
