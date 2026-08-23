import {
  Client,
  type BlockObjectResponse,
  type PartialBlockObjectResponse,
  type RichTextItemResponse,
} from "@notionhq/client";
import { describe, expect, it } from "vitest";

import {
  NOTION_BLOCK_CATALOG_ID,
  Surface,
  type Column,
  type ComponentId,
  type Message,
} from "../src/a2ui.js";
import { N2A2UIClient, type N2A2UIClientOptions } from "../src/client.js";
import type { NotionBlock } from "../src/convert/converter.js";
import { ROOT_ID } from "../src/id.js";

const annotations = {
  bold: false,
  italic: false,
  strikethrough: false,
  underline: false,
  code: false,
  color: "default",
} satisfies RichTextItemResponse["annotations"];

function text(value: string): RichTextItemResponse {
  return {
    type: "text",
    text: { content: value, link: null },
    plain_text: value,
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
): BlockObjectResponse {
  return block(
    id,
    "paragraph",
    { rich_text: [text(value)], color: "default", icon: null },
    hasChildren,
  );
}

function bulleted(id: string, value: string): BlockObjectResponse {
  return block(id, "bulleted_list_item", {
    rich_text: [text(value)],
    color: "default",
  });
}

function numbered(id: string, value: string): BlockObjectResponse {
  return block(id, "numbered_list_item", {
    rich_text: [text(value)],
    color: "default",
  });
}

function todo(id: string, value: string): BlockObjectResponse {
  return block(id, "to_do", {
    rich_text: [text(value)],
    color: "default",
    checked: false,
  });
}

function partial(id: string): PartialBlockObjectResponse {
  return { object: "block", id };
}

type NotionFetch = NonNullable<
  NonNullable<ConstructorParameters<typeof Client>[0]>["fetch"]
>;
type Pages = Readonly<Record<string, readonly (readonly NotionBlock[])[]>>;

interface HarnessOptions {
  pages?: Pages;
  failures?: ReadonlyMap<string, string>;
  metadataFetch?: typeof globalThis.fetch;
  clientOptions?: Omit<N2A2UIClientOptions, "notion" | "fetch">;
}

function makeHarness(options: HarnessOptions = {}): {
  notion: Client;
  client: N2A2UIClient;
  requests: string[];
} {
  const requests: string[] = [];
  const notionFetch: NotionFetch = async (input) => {
    const url = new URL(input);
    requests.push(url.toString());
    const segments = url.pathname.split("/");
    const blocksIndex = segments.lastIndexOf("blocks");
    const parentId = decodeURIComponent(segments[blocksIndex + 1] ?? "");
    const failure = options.failures?.get(parentId);
    if (failure !== undefined) {
      const body = JSON.stringify({
        object: "error",
        status: 400,
        code: "validation_error",
        message: failure,
        request_id: `request-${parentId}`,
      });
      return {
        body: null,
        headers: new Headers({ "content-type": "application/json" }),
        ok: false,
        status: 400,
        text: async () => body,
      };
    }

    const pageIndex = Number(url.searchParams.get("start_cursor") ?? "0");
    const pages = options.pages?.[parentId] ?? [[]];
    const results = pages[pageIndex] ?? [];
    const hasMore = pageIndex + 1 < pages.length;
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
  const client = new N2A2UIClient({
    notion,
    fetch: metadataFetch,
    ...options.clientOptions,
  });
  return { notion, client, requests };
}

function updateFrom(message: Message) {
  if (!("updateComponents" in message)) {
    throw new Error("expected updateComponents message");
  }
  return message.updateComponents;
}

function rootFrom(message: Message): Column {
  const root = updateFrom(message).components.find(
    (component) => component.id === ROOT_ID,
  );
  if (root?.component !== "Column") throw new Error("missing root Column");
  return root;
}

function componentIds(value: unknown): ComponentId[] {
  if (
    !Array.isArray(value) ||
    !value.every((item) => typeof item === "string")
  ) {
    throw new Error("expected a static component id list");
  }
  return value;
}

async function collectMessages(
  stream: AsyncIterable<Message>,
): Promise<Message[]> {
  const messages: Message[] = [];
  for await (const message of stream) messages.push(message);
  return messages;
}

async function nextMessage(iterator: AsyncIterator<Message>): Promise<Message> {
  const result = await iterator.next();
  if (result.done) throw new Error("stream ended unexpectedly");
  return result.value;
}

describe("N2A2UIClient", () => {
  it("normalizes options and exposes the official client and fetch", () => {
    const metadataFetch: typeof globalThis.fetch = async () => new Response();
    const { notion, client } = makeHarness({
      metadataFetch,
      clientOptions: {
        enableUnsupportedBlock: true,
        enableFetchImageMeta: true,
        enableFetchBookmarkMeta: true,
        enableHtmlEmbed: true,
      },
    });

    expect(client.notion).toBe(notion);
    expect(client.fetch).toBe(metadataFetch);
    expect(client.options).toEqual({
      enableUnsupportedBlock: true,
      enableFetchImageMeta: true,
      enableFetchBookmarkMeta: true,
      enableHtmlEmbed: true,
    });
    expect(Object.isFrozen(client.options)).toBe(true);

    const defaults = makeHarness().client.options;
    expect(defaults).toEqual({
      enableUnsupportedBlock: false,
      enableFetchImageMeta: false,
      enableFetchBookmarkMeta: false,
      enableHtmlEmbed: false,
    });
  });

  it("builds an eager root-first surface without changing converter order", async () => {
    const { client } = makeHarness({
      pages: {
        page: [[paragraph("p-1", "hello"), block("d-1", "divider", {})]],
      },
    });

    const surface = await client.convertBlock("page");

    expect(surface.root).toBe(ROOT_ID);
    expect(surface.components.get(ROOT_ID)).toEqual({
      id: ROOT_ID,
      component: "Column",
      children: ["p-1", "d-1"],
    });
    expect([...surface.components.keys()]).toEqual([
      ROOT_ID,
      "p-1::rich_text/0",
      "p-1",
      "d-1",
    ]);
  });

  it("streams protocol messages with grouped chunks and snapshot roots", async () => {
    const { client } = makeHarness({
      pages: {
        page: [
          [
            paragraph("p-1", "intro"),
            bulleted("b-1", "bullet"),
            todo("t-1", "todo"),
            numbered("n-1", "one"),
            numbered("n-2", "two"),
            block("skip", "breadcrumb", {}),
            block("d-1", "divider", {}),
          ],
        ],
      },
    });

    const messages = await collectMessages(
      client.convertBlockStream("page", "surface-1"),
    );

    expect(messages[0]).toEqual({
      version: "v0.9",
      createSurface: {
        surfaceId: "surface-1",
        catalogId: NOTION_BLOCK_CATALOG_ID,
      },
    });
    expect(messages).toHaveLength(6);
    expect(
      messages
        .slice(1)
        .map(rootFrom)
        .map((root) => componentIds(root.children)),
    ).toEqual([
      [],
      ["p-1"],
      ["p-1", "b-1::list"],
      ["p-1", "b-1::list", "n-1::list"],
      ["p-1", "b-1::list", "n-1::list", "d-1"],
    ]);

    for (const message of messages.slice(2)) {
      const components = updateFrom(message).components;
      expect(components.at(-1)).toEqual(rootFrom(message));
    }
    const unordered = updateFrom(messages[3]!).components.find(
      (component) => component.id === "b-1::list",
    );
    expect(unordered).toEqual({
      id: "b-1::list",
      component: "List",
      children: ["b-1", "t-1"],
      style: "unordered",
    });
    const ordered = updateFrom(messages[4]!).components.find(
      (component) => component.id === "n-1::list",
    );
    expect(ordered).toMatchObject({
      children: ["n-1", "n-2"],
      style: "ordered",
    });
  });

  it("reconstructs the eager surface with identical component order", async () => {
    const { client } = makeHarness({
      pages: {
        page: [
          [
            paragraph("p-1", "alpha"),
            bulleted("b-1", "bravo"),
            bulleted("b-2", "charlie"),
            paragraph("p-2", "delta"),
          ],
        ],
      },
    });
    const eager = await client.convertBlock("page");
    const messages = await client.convertBlockToMessages("page", "surface-1");
    const reconstructed = new Surface(ROOT_ID);

    for (const message of messages.slice(1)) {
      for (const component of updateFrom(message).components) {
        reconstructed.insert(component);
      }
    }

    expect([...reconstructed.components.keys()]).toEqual([
      ...eager.components.keys(),
    ]);
    expect(reconstructed.toJSON()).toEqual(eager.toJSON());
  });

  it("keeps an all-skipped stream valid with an empty root", async () => {
    const { client } = makeHarness({
      pages: {
        page: [[block("skip", "breadcrumb", {}), partial("partial")]],
      },
    });

    const messages = await collectMessages(
      client.convertBlockStream("page", "surface-1"),
    );

    expect(messages).toHaveLength(2);
    expect(updateFrom(messages[1]!).components).toEqual([
      { id: ROOT_ID, component: "Column", children: [] },
    ]);
  });

  it("collects exactly the convertBlockStream sequence", async () => {
    const { client } = makeHarness({
      pages: {
        page: [[paragraph("p-1", "hello"), block("d-1", "divider", {})]],
      },
    });
    const streamed = await collectMessages(
      client.convertBlockStream("page", "surface-1"),
    );

    await expect(
      client.convertBlockToMessages("page", "surface-1"),
    ).resolves.toEqual(streamed);
  });

  it("finishes direct-child pagination before yielding createSurface", async () => {
    const { client, requests } = makeHarness({
      pages: {
        page: [[block("d-1", "divider", {})], [block("d-2", "divider", {})]],
      },
    });
    const iterator = client.convertBlockStream("page", "surface-1");

    expect(requests).toEqual([]);
    const first = await nextMessage(iterator);

    expect(first).toHaveProperty("createSurface");
    expect(requests).toHaveLength(2);
    expect(requests[1]).toContain("start_cursor=1");
  });

  it("throws a direct-child fetch error before delivering any message", async () => {
    const { client } = makeHarness({
      failures: new Map([["page", "top-level failed"]]),
    });
    const iterator = client.convertBlockStream("page", "surface-1");

    await expect(iterator.next()).rejects.toThrow("top-level failed");
    await expect(iterator.next()).resolves.toEqual({
      done: true,
      value: undefined,
    });
  });

  it("preserves delivered chunks when a later recursive conversion fails", async () => {
    const { client } = makeHarness({
      pages: {
        page: [
          [
            block("d-1", "divider", {}),
            paragraph("nested-parent", "later", true),
          ],
        ],
      },
      failures: new Map([["nested-parent", "nested failed"]]),
    });
    const iterator = client.convertBlockStream("page", "surface-1");
    const delivered: Message[] = [];

    delivered.push(await nextMessage(iterator));
    delivered.push(await nextMessage(iterator));
    delivered.push(await nextMessage(iterator));

    expect(delivered[0]).toHaveProperty("createSurface");
    expect(componentIds(rootFrom(delivered[1]!).children)).toEqual([]);
    expect(componentIds(rootFrom(delivered[2]!).children)).toEqual(["d-1"]);
    expect(updateFrom(delivered[2]!).components[0]).toEqual({
      id: "d-1",
      component: "Divider",
    });
    await expect(iterator.next()).rejects.toThrow("nested failed");
  });
});
