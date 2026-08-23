import {
  Client,
  type ApiColor,
  type BlockObjectResponse,
  type PartialBlockObjectResponse,
  type RichTextItemResponse,
} from "@notionhq/client";
import { describe, expect, it } from "vitest";

import type { Component, ComponentId } from "../src/a2ui.js";
import {
  Converter,
  isHtmlFileUrl,
  topLevelGroups,
  type ConverterOptions,
  type NotionBlock,
} from "../src/convert/converter.js";
import { convertRichTexts } from "../src/convert/rich-text.js";
import { ROOT_ID } from "../src/id.js";

const annotations = {
  bold: false,
  italic: false,
  strikethrough: false,
  underline: false,
  code: false,
  color: "default",
} satisfies RichTextItemResponse["annotations"];

function text(
  value: string,
  options: {
    href?: string | null;
    annotations?: Partial<RichTextItemResponse["annotations"]>;
  } = {},
): RichTextItemResponse {
  return {
    type: "text",
    text: { content: value, link: null },
    plain_text: value,
    href: options.href ?? null,
    annotations: { ...annotations, ...options.annotations },
  };
}

function equation(
  expression: string,
  plainText: string = expression,
): RichTextItemResponse {
  return {
    type: "equation",
    equation: { expression },
    plain_text: plainText,
    href: null,
    annotations: { ...annotations, bold: true },
  };
}

function customEmoji(name: string, url: string): RichTextItemResponse {
  return {
    type: "mention",
    mention: {
      type: "custom_emoji",
      custom_emoji: { id: `emoji-${name}`, name, url },
    },
    plain_text: `:${name}:`,
    href: null,
    annotations,
  };
}

function block(
  id: string,
  type: string,
  payload: unknown,
  hasChildren = false,
): BlockObjectResponse {
  return {
    object: "block",
    id,
    parent: { type: "page_id", page_id: "parent" },
    created_time: "2024-01-01T00:00:00.000Z",
    created_by: { object: "user", id: "user" },
    last_edited_time: "2024-01-01T00:00:00.000Z",
    last_edited_by: { object: "user", id: "user" },
    has_children: hasChildren,
    archived: false,
    in_trash: false,
    type,
    [type]: payload,
  } as unknown as BlockObjectResponse;
}

function paragraph(
  id: string,
  value: string,
  hasChildren = false,
  color: ApiColor = "default",
): BlockObjectResponse {
  return block(
    id,
    "paragraph",
    { rich_text: [text(value)], color, icon: null },
    hasChildren,
  );
}

function bulleted(
  id: string,
  value: string,
  hasChildren = false,
): BlockObjectResponse {
  return block(
    id,
    "bulleted_list_item",
    { rich_text: [text(value)], color: "default" },
    hasChildren,
  );
}

function numbered(id: string, value: string): BlockObjectResponse {
  return block(id, "numbered_list_item", {
    rich_text: [text(value)],
    color: "default",
  });
}

function todo(
  id: string,
  value: string,
  checked: boolean,
): BlockObjectResponse {
  return block(id, "to_do", {
    rich_text: [text(value)],
    color: "default",
    checked,
  });
}

function tableRow(id: string, values: readonly string[]): BlockObjectResponse {
  return block(id, "table_row", {
    cells: values.map((value) => [text(value)]),
  });
}

function bookmark(
  id: string,
  url: string,
  caption: readonly RichTextItemResponse[] = [],
): BlockObjectResponse {
  return block(id, "bookmark", { url, caption });
}

function embed(id: string, url: string): BlockObjectResponse {
  return block(id, "embed", { url, caption: [] });
}

type Pages = Record<string, readonly (readonly NotionBlock[])[]>;
type NotionFetch = NonNullable<
  NonNullable<ConstructorParameters<typeof Client>[0]>["fetch"]
>;

interface TestConverterOptions extends Partial<
  Omit<ConverterOptions, "notion" | "fetch">
> {
  pages?: Pages;
  metadataFetch?: typeof globalThis.fetch;
}

function makeConverter(options: TestConverterOptions = {}): {
  converter: Converter;
  requests: string[];
} {
  const requests: string[] = [];
  const pages = options.pages ?? {};
  const notionFetch: NotionFetch = async (input) => {
    const url = new URL(input);
    requests.push(url.toString());
    const segments = url.pathname.split("/");
    const blocksIndex = segments.lastIndexOf("blocks");
    const parentId = decodeURIComponent(segments[blocksIndex + 1] ?? "");
    const pageIndex = Number(url.searchParams.get("start_cursor") ?? "0");
    const parentPages = pages[parentId] ?? [[]];
    const results = parentPages[pageIndex] ?? [];
    const hasMore = pageIndex + 1 < parentPages.length;
    const body = JSON.stringify({
      object: "list",
      type: "block",
      block: {},
      results,
      has_more: hasMore,
      next_cursor: hasMore ? String(pageIndex + 1) : null,
    });
    return {
      body: null,
      headers: new Headers({ "content-type": "application/json" }),
      ok: true,
      status: 200,
      text: async () => body,
    };
  };
  const notion = new Client({
    auth: "dummy",
    baseUrl: "https://notion.test/v1",
    fetch: notionFetch,
  });
  const metadataFetch =
    options.metadataFetch ??
    (async () => {
      throw new Error("unexpected metadata fetch");
    });

  return {
    converter: new Converter({
      notion,
      fetch: metadataFetch,
      enableUnsupportedBlock: options.enableUnsupportedBlock ?? false,
      enableFetchImageMeta: options.enableFetchImageMeta ?? false,
      enableFetchBookmarkMeta: options.enableFetchBookmarkMeta ?? false,
      enableHtmlEmbed: options.enableHtmlEmbed ?? false,
    }),
    requests,
  };
}

async function convertOne(
  converter: Converter,
  notion: NotionBlock,
): Promise<[ComponentId, Component[]]> {
  const result = await converter.convertSingleBlockToChunk(notion);
  if (result === undefined) throw new Error("block was skipped");
  return result;
}

async function streamingChunks(
  converter: Converter,
  blocks: readonly NotionBlock[],
): Promise<[ComponentId, Component[]][]> {
  const chunks: [ComponentId, Component[]][] = [];
  for (const group of topLevelGroups(blocks)) {
    const chunk =
      group.kind === "list"
        ? await converter.convertListGroupToChunk(group.blocks, group.style)
        : await converter.convertSingleBlockToChunk(group.block);
    if (chunk !== undefined) chunks.push(chunk);
  }
  return chunks;
}

function byId(components: readonly Component[], id: string): Component {
  const found = components.find((component) => component.id === id);
  if (found === undefined) throw new Error(`missing component ${id}`);
  return found;
}

function png(width: number, height: number): ArrayBuffer {
  return new Uint8Array([
    0x89,
    0x50,
    0x4e,
    0x47,
    0x0d,
    0x0a,
    0x1a,
    0x0a,
    0,
    0,
    0,
    0x0d,
    0x49,
    0x48,
    0x44,
    0x52,
    (width >>> 24) & 0xff,
    (width >>> 16) & 0xff,
    (width >>> 8) & 0xff,
    width & 0xff,
    (height >>> 24) & 0xff,
    (height >>> 16) & 0xff,
    (height >>> 8) & 0xff,
    height & 0xff,
  ]).buffer;
}

describe("converter regression coverage", () => {
  it("does not route a bookmark caption into description", async () => {
    const { converter } = makeConverter();
    const [id, components] = await convertOne(
      converter,
      bookmark("bk-1", "https://example.com", [text("user caption")]),
    );

    expect(id).toBe("bk-1");
    expect(components).toEqual([
      {
        id: "bk-1",
        component: "Bookmark",
        url: "https://example.com",
      },
    ]);
  });

  it("keeps eager and streaming grouping and component order identical", async () => {
    const { converter } = makeConverter();
    const blocks = [
      paragraph("p-1", "intro"),
      bulleted("b-1", "first bullet"),
      bulleted("b-2", "second bullet"),
      paragraph("p-2", "middle"),
      numbered("n-1", "first item"),
      numbered("n-2", "second item"),
      paragraph("p-3", "outro"),
    ];
    const eagerComponents: Component[] = [];
    const eagerIds = await converter.convertSiblings(blocks, eagerComponents);
    const streamed = await streamingChunks(converter, blocks);

    expect(streamed.map(([id]) => id)).toEqual(eagerIds);
    expect(streamed.flatMap(([, components]) => components)).toEqual(
      eagerComponents,
    );
    expect(eagerIds).toEqual(["p-1", "b-1::list", "p-2", "n-1::list", "p-3"]);
  });

  it("preserves eager Map order when streaming updates upsert root", async () => {
    const { converter } = makeConverter();
    const blocks = [
      paragraph("p-1", "alpha"),
      bulleted("b-1", "bravo"),
      bulleted("b-2", "charlie"),
      paragraph("p-2", "delta"),
    ];
    const eagerComponents: Component[] = [];
    const eagerChildren = await converter.convertSiblings(
      blocks,
      eagerComponents,
    );
    const eager = new Map<ComponentId, Component>();
    eager.set(ROOT_ID, {
      id: ROOT_ID,
      component: "Column",
      children: eagerChildren,
    });
    for (const component of eagerComponents) eager.set(component.id, component);

    const streamed = new Map<ComponentId, Component>();
    streamed.set(ROOT_ID, {
      id: ROOT_ID,
      component: "Column",
      children: [],
    });
    const accumulated: ComponentId[] = [];
    for (const [id, components] of await streamingChunks(converter, blocks)) {
      accumulated.push(id);
      for (const component of components) streamed.set(component.id, component);
      streamed.set(ROOT_ID, {
        id: ROOT_ID,
        component: "Column",
        children: [...accumulated],
      });
    }

    expect([...streamed.keys()]).toEqual([...eager.keys()]);
  });

  it("maps a built-in callout icon to Notion's exact asset URL", async () => {
    const { converter } = makeConverter();
    const callout = block("co-1", "callout", {
      rich_text: [text("heads up")],
      icon: {
        type: "icon",
        icon: { name: "info-alternate", color: "blue" },
      },
      color: "default",
    });
    const [, components] = await convertOne(converter, callout);

    expect(components.at(-1)).toMatchObject({
      component: "NotionCallout",
      icon: {
        kind: "image",
        src: "https://app.notion.com/icons/info-alternate_blue.svg",
        alt: "info-alternate",
      },
    });
  });

  it("normalizes defensive light_gray icon responses to lightgray", async () => {
    const { converter } = makeConverter();
    const callout = block("co-2", "callout", {
      rich_text: [text("note")],
      icon: {
        type: "icon",
        icon: { name: "home", color: "light_gray" },
      },
      color: "default",
    });
    const [, components] = await convertOne(converter, callout);

    expect(components.at(-1)).toMatchObject({
      icon: {
        src: "https://app.notion.com/icons/home_lightgray.svg",
      },
    });
  });

  it("maps emoji and background callout color to filled", async () => {
    const { converter } = makeConverter();
    const callout = block("co-3", "callout", {
      rich_text: [text("note")],
      icon: { type: "emoji", emoji: "💡" },
      color: "blue_background",
    });
    const [, components] = await convertOne(converter, callout);

    expect(components.at(-1)).toMatchObject({
      component: "NotionCallout",
      icon: { kind: "emoji", emoji: "💡" },
      color: "blue",
      variant: "filled",
    });
  });

  it("maps foreground callout color to outlined", async () => {
    const { converter } = makeConverter();
    const callout = block("co-4", "callout", {
      rich_text: [text("note")],
      icon: { type: "emoji", emoji: "💡" },
      color: "blue",
    });
    const [, components] = await convertOne(converter, callout);

    expect(components.at(-1)).toMatchObject({
      color: "blue",
      variant: "outlined",
    });
  });

  it("filters non-row table children before selecting the header", () => {
    const { converter } = makeConverter();
    const components: Component[] = [];
    const [header, body] = converter.classifyTableRows(
      [
        paragraph("stray", "not a row"),
        tableRow("row-a", ["A1", "A2"]),
        tableRow("row-b", ["B1", "B2"]),
      ],
      true,
      false,
      components,
    );

    expect(header).toEqual(["row-a"]);
    expect(body).toEqual(["row-b"]);
  });

  it("detects HTML paths case-insensitively before query or fragment", () => {
    expect(
      isHtmlFileUrl("https://example.com/x/diagram.html?X-Amz-Date=1"),
    ).toBe(true);
    expect(isHtmlFileUrl("https://example.com/DIAGRAM.HTML")).toBe(true);
    expect(isHtmlFileUrl("https://example.com/a/b/c.html#section")).toBe(true);
  });

  it("rejects non-HTML paths and HTML text only in a query", () => {
    expect(isHtmlFileUrl("https://example.com/video.mp4")).toBe(false);
    expect(isHtmlFileUrl("https://example.com/page?redirect=foo.html")).toBe(
      false,
    );
    expect(isHtmlFileUrl("https://youtube.com/watch?v=abc123")).toBe(false);
  });

  it("emits Html with src for enabled HTML embeds", async () => {
    const { converter } = makeConverter({ enableHtmlEmbed: true });
    const url = "https://s3.example/x/diagram.html?X-Amz-Signature=abc";

    await expect(convertOne(converter, embed("e-1", url))).resolves.toEqual([
      "e-1",
      [{ id: "e-1", component: "Html", src: url }],
    ]);
  });

  it("falls back to Bookmark when HTML embeds are disabled", async () => {
    const { converter } = makeConverter();
    const url = "https://s3.example/x/diagram.html?sig=abc";

    await expect(convertOne(converter, embed("e-1", url))).resolves.toEqual([
      "e-1",
      [{ id: "e-1", component: "Bookmark", url }],
    ]);
  });

  it("falls back to Bookmark for enabled non-HTML embeds", async () => {
    const { converter } = makeConverter({ enableHtmlEmbed: true });
    const url = "https://youtube.com/watch?v=abc123";

    await expect(convertOne(converter, embed("e-1", url))).resolves.toEqual([
      "e-1",
      [{ id: "e-1", component: "Bookmark", url }],
    ]);
  });
});

describe("rich text", () => {
  it("emits one deterministic component per run with canonical decoration order", () => {
    const items: RichTextItemResponse[] = [
      text("decorated", {
        annotations: {
          bold: true,
          italic: true,
          underline: true,
          strikethrough: true,
          code: true,
          color: "red",
        },
      }),
      text("background", { annotations: { color: "blue_background" } }),
      equation("x^2", "wrong plain text"),
      text("linked", {
        href: "https://example.com",
        annotations: { bold: true },
      }),
      customEmoji("party", "https://example.com/party.png"),
    ];

    expect(convertRichTexts("parent", "rich_text", items)).toEqual({
      ids: [
        "parent::rich_text/0",
        "parent::rich_text/1",
        "parent::rich_text/2",
        "parent::rich_text/3",
        "parent::rich_text/4",
      ],
      components: [
        {
          id: "parent::rich_text/0",
          component: "RichText",
          text: "decorated",
          decoration: ["bold", "italic", "underline", "strikethrough", "code"],
          color: "#b36472",
        },
        {
          id: "parent::rich_text/1",
          component: "RichText",
          text: "background",
        },
        {
          id: "parent::rich_text/2",
          component: "RichText",
          text: "x^2",
          decoration: ["bold", "katex"],
        },
        {
          id: "parent::rich_text/3",
          component: "LinkText",
          text: "linked",
          href: "https://example.com",
        },
        {
          id: "parent::rich_text/4",
          component: "Icon",
          src: "https://example.com/party.png",
          alt: "party",
        },
      ],
    });
  });
});

describe("lists and pagination", () => {
  it("groups bullets and todos together and emits stable checkbox marks", async () => {
    const { converter } = makeConverter();
    const components: Component[] = [];
    const ids = await converter.convertSiblings(
      [
        bulleted("b-1", "bullet"),
        todo("t-1", "done", true),
        todo("t-2", "open", false),
        numbered("n-1", "one"),
      ],
      components,
    );

    expect(ids).toEqual(["b-1::list", "n-1::list"]);
    expect(byId(components, "t-1::todo_mark/0")).toEqual({
      id: "t-1::todo_mark/0",
      component: "RichText",
      text: "☑ ",
    });
    expect(byId(components, "t-2::todo_mark/0")).toEqual({
      id: "t-2::todo_mark/0",
      component: "RichText",
      text: "☐ ",
    });
    expect(byId(components, "b-1::list")).toEqual({
      id: "b-1::list",
      component: "List",
      children: ["b-1", "t-1", "t-2"],
      style: "unordered",
    });
  });

  it("uses official SDK pagination recursively and groups across pages", async () => {
    const nestedItems = Array.from({ length: 100 }, (_, index) =>
      bulleted(`b-${index}`, `item ${index}`),
    );
    const parent = paragraph("parent", "parent", true);
    const { converter, requests } = makeConverter({
      pages: {
        root: [[parent]],
        parent: [nestedItems, [todo("todo-100", "last", false)]],
      },
    });

    const [ids, components] = await converter.convertChildren("root");

    expect(ids).toEqual(["parent"]);
    expect(
      requests.filter((url) => url.includes("/parent/children")),
    ).toHaveLength(2);
    expect(requests[2]).toContain("start_cursor=1");
    expect(byId(components, "b-0::list")).toMatchObject({
      component: "List",
      children: [...nestedItems.map((item) => item.id), "todo-100"],
    });
    expect(byId(components, "parent")).toEqual({
      id: "parent",
      component: "Paragraph",
      children: ["parent::rich_text/0", "b-0::list"],
    });
    expect(components.at(-1)?.id).toBe("parent");
  });
});

describe("media and metadata", () => {
  it("maps external and hosted media, runtime names, and empty captions", async () => {
    const { converter } = makeConverter();
    const mediaBlocks = [
      block("image", "image", {
        type: "external",
        external: { url: "https://example.com/image.png" },
        name: "runtime image name",
        caption: [],
      }),
      block("file", "file", {
        type: "file",
        file: {
          url: "https://example.com/file.zip",
          expiry_time: "2027-01-01T00:00:00.000Z",
        },
        name: "file.zip",
        caption: [],
      }),
      block("pdf", "pdf", {
        type: "external",
        external: { url: "https://example.com/doc.pdf" },
        name: "runtime.pdf",
        caption: [],
      }),
      block("audio", "audio", {
        type: "external",
        external: { url: "https://example.com/audio.mp3" },
        name: "runtime audio",
        caption: [],
      }),
      block("video", "video", {
        type: "file",
        file: {
          url: "https://example.com/video.mp4",
          expiry_time: "2027-01-01T00:00:00.000Z",
        },
        name: "runtime video",
        caption: [],
      }),
      block("bad-image", "image", {
        type: "file_upload",
        file_upload: { id: "upload" },
        caption: [],
      }),
    ];
    const components: Component[] = [];
    await converter.convertSiblings(mediaBlocks, components);

    expect(byId(components, "image")).toEqual({
      id: "image",
      component: "BlockImage",
      src: "https://example.com/image.png",
      alt: "runtime image name",
      caption: "",
    });
    expect(byId(components, "file")).toMatchObject({
      component: "File",
      name: "file.zip",
    });
    expect(byId(components, "pdf")).toMatchObject({
      component: "File",
      name: "runtime.pdf",
    });
    expect(byId(components, "audio")).toMatchObject({
      component: "Audio",
      title: "runtime audio",
    });
    expect(byId(components, "video")).toEqual({
      id: "video",
      component: "Video",
      src: "https://example.com/video.mp4",
      title: "runtime video",
      caption: "",
    });
    expect(byId(components, "bad-image")).toEqual({
      id: "bad-image",
      component: "Unsupported",
      details: "image:api_uploaded",
    });
  });

  it("applies optional image and bookmark metadata", async () => {
    const metadataFetch: typeof globalThis.fetch = async (input) => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.href
            : input.url;
      if (url.endsWith("image.png")) return new Response(png(320, 180));
      return new Response(`
        <title>Native</title>
        <meta property="og:title" content="OG title">
        <meta property="og:description" content="OG description">
        <meta property="og:image" content="/og.png">
      `);
    };
    const { converter } = makeConverter({
      metadataFetch,
      enableFetchImageMeta: true,
      enableFetchBookmarkMeta: true,
    });
    const [, imageComponents] = await convertOne(
      converter,
      block("image", "image", {
        type: "external",
        external: { url: "https://example.com/image.png" },
        caption: [text("caption")],
      }),
    );
    const [, bookmarkComponents] = await convertOne(
      converter,
      bookmark("bookmark", "https://example.com/article"),
    );

    expect(imageComponents[0]).toMatchObject({
      width: 320,
      height: 180,
      caption: "caption",
    });
    expect(bookmarkComponents[0]).toMatchObject({
      title: "OG title",
      description: "OG description",
      image: "/og.png",
    });
  });

  it("keeps metadata failures nonfatal", async () => {
    const metadataFetch: typeof globalThis.fetch = async () => {
      throw new Error("network failed");
    };
    const { converter } = makeConverter({
      metadataFetch,
      enableFetchImageMeta: true,
      enableFetchBookmarkMeta: true,
    });

    await expect(
      convertOne(
        converter,
        block("image", "image", {
          type: "external",
          external: { url: "https://example.com/image.png" },
          caption: [],
        }),
      ),
    ).resolves.toEqual([
      "image",
      [
        {
          id: "image",
          component: "BlockImage",
          src: "https://example.com/image.png",
          caption: "",
        },
      ],
    ]);
    await expect(
      convertOne(converter, bookmark("bookmark", "https://example.com")),
    ).resolves.toEqual([
      "bookmark",
      [
        {
          id: "bookmark",
          component: "Bookmark",
          url: "https://example.com",
        },
      ],
    ]);
  });
});

describe("unsupported responses", () => {
  it("skips unsupported and partial blocks when disabled", async () => {
    const { converter } = makeConverter();
    const partial: PartialBlockObjectResponse = {
      object: "block",
      id: "partial",
    };

    await expect(
      converter.convertSingleBlockToChunk(block("crumb", "breadcrumb", {})),
    ).resolves.toBeUndefined();
    await expect(
      converter.convertSingleBlockToChunk(partial),
    ).resolves.toBeUndefined();
  });

  it("labels ordinary, explicit, and partial unsupported blocks", async () => {
    const { converter } = makeConverter({ enableUnsupportedBlock: true });
    const partial: PartialBlockObjectResponse = {
      object: "block",
      id: "partial",
    };

    await expect(
      convertOne(converter, block("crumb", "breadcrumb", {})),
    ).resolves.toEqual([
      "crumb",
      [
        {
          id: "crumb",
          component: "Unsupported",
          details: "breadcrumb",
        },
      ],
    ]);
    await expect(
      convertOne(
        converter,
        block("future", "unsupported", { block_type: "future_block" }),
      ),
    ).resolves.toEqual([
      "future",
      [
        {
          id: "future",
          component: "Unsupported",
          details: "unsupported:future_block",
        },
      ],
    ]);
    await expect(convertOne(converter, partial)).resolves.toEqual([
      "partial",
      [
        {
          id: "partial",
          component: "Unsupported",
          details: "partial_block",
        },
      ],
    ]);
  });
});

describe("tables and tabs", () => {
  it("fetches table children unconditionally and marks row headers", async () => {
    const table = block("table", "table", {
      table_width: 2,
      has_column_header: true,
      has_row_header: true,
    });
    const { converter, requests } = makeConverter({
      pages: {
        table: [
          [
            paragraph("stray", "ignored"),
            tableRow("header-row", ["Name", "Value"]),
            tableRow("body-row", ["First", "1"]),
          ],
        ],
      },
    });
    const [, components] = await convertOne(converter, table);

    expect(requests).toHaveLength(1);
    expect(byId(components, "table")).toEqual({
      id: "table",
      component: "Table",
      body: ["body-row"],
      header: ["header-row"],
      hasColumnHeader: true,
      hasRowHeader: true,
    });
    expect(byId(components, "header-row::cell/0")).toMatchObject({
      component: "TableCell",
      isHeader: true,
    });
    expect(byId(components, "header-row::cell/1")).not.toHaveProperty(
      "isHeader",
    );
    expect(components.at(-1)?.id).toBe("table");
  });

  it("uses direct paragraph children as tabs and ignores other blocks", async () => {
    const tab = block("tabs", "tab", {}, true);
    const tabA = paragraph("tab-a", "Alpha", true);
    const tabB = paragraph("tab-b", "Beta");
    const { converter } = makeConverter({
      pages: {
        tabs: [[tabA, block("ignored", "divider", {}), tabB]],
        "tab-a": [[paragraph("content-a", "Panel A")]],
      },
    });
    const [, components] = await convertOne(converter, tab);

    expect(byId(components, "tab-a")).toEqual({
      id: "tab-a",
      component: "ContentTab",
      label: ["tab-a::label/0"],
      content: ["content-a"],
    });
    expect(byId(components, "tab-b")).toEqual({
      id: "tab-b",
      component: "ContentTab",
      label: ["tab-b::label/0"],
      content: [],
    });
    expect(byId(components, "tabs")).toEqual({
      id: "tabs",
      component: "ContentTabs",
      children: ["tab-a", "tab-b"],
    });
    expect(components.some((component) => component.id === "ignored")).toBe(
      false,
    );
  });
});

describe("representative block mappings", () => {
  it("maps typography, code, links, columns, and synced blocks", async () => {
    const heading = block(
      "heading",
      "heading_4",
      {
        rich_text: [text("Heading")],
        color: "red",
        is_toggleable: true,
      },
      true,
    );
    const blocks = [
      paragraph("colored", "Color", false, "green_background"),
      heading,
      block("quote", "quote", {
        rich_text: [text("Quote")],
        color: "default",
      }),
      block("toggle", "toggle", {
        rich_text: [text("Summary")],
        color: "default",
      }),
      block("divider", "divider", {}),
      block("code", "code", {
        rich_text: [text("const x = 1;")],
        caption: [text("TypeScript")],
        language: "typescript",
      }),
      block("mermaid", "code", {
        rich_text: [text("graph TD")],
        caption: [text("ignored")],
        language: "mermaid",
      }),
      block("math", "equation", { expression: "x^2" }),
      block("preview", "link_preview", { url: "https://example.com/p" }),
      block("page-id", "child_page", { title: "Child page" }),
      block("db-id", "child_database", { title: "Child database" }),
      block("columns", "column_list", {}),
      block("column", "column", { width_ratio: 0.4 }),
      block("synced", "synced_block", { synced_from: null }),
    ];
    const { converter, requests } = makeConverter({
      pages: { heading: [[paragraph("ignored-child", "ignored")]] },
    });
    const components: Component[] = [];
    await converter.convertSiblings(blocks, components);

    expect(byId(components, "colored")).toEqual({
      id: "colored",
      component: "Paragraph",
      children: ["colored::rich_text/0"],
      backgroundColor: "#b1dcc2",
    });
    expect(byId(components, "heading")).toEqual({
      id: "heading",
      component: "Heading",
      level: 4,
      children: ["heading::rich_text/0"],
    });
    expect(requests).toEqual([]);
    expect(byId(components, "quote")).toMatchObject({
      component: "BlockQuote",
      children: ["quote::rich_text/0"],
    });
    expect(byId(components, "toggle")).toEqual({
      id: "toggle",
      component: "Toggle",
      summary: ["toggle::summary/0"],
      children: [],
    });
    expect(byId(components, "divider")).toEqual({
      id: "divider",
      component: "Divider",
    });
    expect(byId(components, "code")).toEqual({
      id: "code",
      component: "CodeBlock",
      code: "const x = 1;",
      language: "typescript",
      caption: "TypeScript",
    });
    expect(byId(components, "mermaid")).toEqual({
      id: "mermaid",
      component: "Mermaid",
      code: "graph TD",
    });
    expect(byId(components, "math")).toEqual({
      id: "math",
      component: "Katex",
      expression: "x^2",
    });
    expect(byId(components, "preview")).toMatchObject({
      component: "Bookmark",
      url: "https://example.com/p",
    });
    expect(byId(components, "page-id")).toEqual({
      id: "page-id",
      component: "Bookmark",
      url: "https://www.notion.so/pageid",
      title: "Child page",
    });
    expect(byId(components, "db-id")).toEqual({
      id: "db-id",
      component: "Bookmark",
      url: "https://www.notion.so/dbid",
      title: "Child database",
    });
    expect(byId(components, "columns")).toEqual({
      id: "columns",
      component: "ColumnList",
      children: [],
    });
    expect(byId(components, "column")).toEqual({
      id: "column",
      component: "Column",
      children: [],
      widthRatio: 0.4,
    });
    expect(byId(components, "synced")).toEqual({
      id: "synced",
      component: "Column",
      children: [],
    });
  });
});
