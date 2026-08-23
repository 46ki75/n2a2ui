import { notionBlockCatalogJson } from "@elmethis/core";
import { describe, expect, it } from "vitest";
import {
  NOTION_BLOCK_CATALOG_ID,
  Surface,
  VERSION,
  createSurface,
  deleteSurface,
  updateComponents,
  updateDataModel,
} from "../src/a2ui";
import type {
  ChildList,
  Component,
  DynamicString,
  SurfaceJSON,
} from "../src/a2ui";
import { ROOT_ID, childId } from "../src/id";

const components = {
  RichText: {
    id: "rich-text",
    component: "RichText",
    text: "hello",
  },
  LinkText: {
    id: "link-text",
    component: "LinkText",
    text: "Elmethis",
    href: "https://example.com",
  },
  Icon: {
    id: "icon",
    component: "Icon",
    src: "https://example.com/icon.svg",
  },
  Row: { id: "row", component: "Row", children: [] },
  Column: { id: "column", component: "Column", children: [] },
  ColumnList: { id: "column-list", component: "ColumnList", children: [] },
  Heading: {
    id: "heading",
    component: "Heading",
    level: 2,
    children: [],
  },
  Paragraph: { id: "paragraph", component: "Paragraph", children: [] },
  List: { id: "list", component: "List", children: [] },
  ListItem: { id: "list-item", component: "ListItem", children: [] },
  BlockQuote: {
    id: "block-quote",
    component: "BlockQuote",
    children: [],
  },
  Callout: { id: "callout", component: "Callout", children: [] },
  NotionCallout: {
    id: "notion-callout",
    component: "NotionCallout",
    children: [],
  },
  Divider: { id: "divider", component: "Divider" },
  Toggle: {
    id: "toggle",
    component: "Toggle",
    summary: [],
    children: [],
  },
  Bookmark: {
    id: "bookmark",
    component: "Bookmark",
    url: "https://example.com",
  },
  File: {
    id: "file",
    component: "File",
    src: "https://example.com/file.pdf",
  },
  Audio: {
    id: "audio",
    component: "Audio",
    src: "https://example.com/track.mp3",
  },
  Video: {
    id: "video",
    component: "Video",
    src: "https://example.com/clip.mp4",
  },
  BlockImage: {
    id: "image",
    component: "BlockImage",
    src: "https://example.com/image.png",
  },
  Html: { id: "html", component: "Html" },
  CodeBlock: { id: "code", component: "CodeBlock", code: "const x = 1;" },
  Katex: { id: "katex", component: "Katex", expression: "x^2" },
  Mermaid: { id: "mermaid", component: "Mermaid", code: "graph TD" },
  ContentTab: {
    id: "content-tab",
    component: "ContentTab",
    label: [],
    content: [],
  },
  ContentTabs: {
    id: "content-tabs",
    component: "ContentTabs",
    children: [],
  },
  Table: { id: "table", component: "Table", body: [] },
  TableRow: { id: "table-row", component: "TableRow", children: [] },
  TableCell: { id: "table-cell", component: "TableCell", children: [] },
  Unsupported: { id: "unsupported", component: "Unsupported" },
} satisfies {
  [Name in Component["component"]]: Extract<Component, { component: Name }>;
};

describe("catalog component types", () => {
  it("covers all 30 catalog discriminators", () => {
    const allComponents: Component[] = Object.values(components);

    expect(allComponents).toHaveLength(30);
    expect(allComponents.map((component) => component.component)).toEqual(
      Object.keys(components),
    );
  });

  it("uses the flat v0.9 component shape", () => {
    const paragraph: Component = {
      id: "p1",
      component: "Paragraph",
      children: ["t1"],
    };
    const richText: Component = {
      id: "t1",
      component: "RichText",
      text: "hello",
      decoration: ["bold"],
    };

    expect(JSON.parse(JSON.stringify(paragraph))).toEqual(paragraph);
    expect(JSON.parse(JSON.stringify(richText))).toEqual(richText);
  });

  it("preserves catalog-specific wire field names", () => {
    const audio: Component = {
      id: "a1",
      component: "Audio",
      src: "https://example.com/track.mp3",
      title: "Track",
      artist: "Artist",
      seekStep: 5,
      loop: true,
      autoPlay: false,
    };
    const callout: Component = {
      id: "c1",
      component: "Callout",
      children: ["p1"],
      type: "warning",
    };
    const notionCallout: Component = {
      id: "nc1",
      component: "NotionCallout",
      children: ["p1"],
      icon: { kind: "emoji", emoji: "idea" },
      color: "blue",
      variant: "filled",
    };
    const html: Component = {
      id: "h1",
      component: "Html",
      src: "https://example.com/doc.html",
      allowScripts: true,
      height: 600,
    };
    const tab: Component = {
      id: "tab1",
      component: "ContentTab",
      label: ["tab1-label"],
      content: ["tab1-paragraph"],
    };

    expect(audio).toMatchObject({ loop: true, autoPlay: false });
    expect(callout).toHaveProperty("type", "warning");
    expect(callout).not.toHaveProperty("calloutType");
    expect(notionCallout.icon).toEqual({ kind: "emoji", emoji: "idea" });
    expect(html).not.toHaveProperty("html");
    expect(html).not.toHaveProperty("autoHeight");
    expect(tab).toMatchObject({
      label: ["tab1-label"],
      content: ["tab1-paragraph"],
    });
    expect(tab).not.toHaveProperty("labels");
    expect(tab).not.toHaveProperty("contents");
  });

  it("re-exports the official dynamic and child-list types", () => {
    const children: ChildList = { componentId: "row-template", path: "/rows" };
    const literal: DynamicString = "hello";
    const binding: DynamicString = { path: "/user/name" };
    const call: DynamicString = {
      call: "trim",
      args: {},
      returnType: "string",
    };

    expect([children, literal, binding, call]).toEqual([
      { componentId: "row-template", path: "/rows" },
      "hello",
      { path: "/user/name" },
      { call: "trim", args: {}, returnType: "string" },
    ]);
  });
});

describe("catalog identity", () => {
  it("uses the canonical Elmethis catalog ID", () => {
    expect(NOTION_BLOCK_CATALOG_ID).toBe(
      "https://46ki75.github.io/elmethis/a2ui/v0_9/notion_block_catalog.json",
    );
    expect(notionBlockCatalogJson.$id).toBe(NOTION_BLOCK_CATALOG_ID);
    expect(notionBlockCatalogJson.catalogId).toBe(NOTION_BLOCK_CATALOG_ID);
  });
});

describe("message constructors", () => {
  it("uses exactly the v0.9 protocol version", () => {
    expect(VERSION).toBe("v0.9");
  });

  it("constructs createSurface with the flattened wire shape", () => {
    expect(
      createSurface({
        surfaceId: "s1",
        catalogId: NOTION_BLOCK_CATALOG_ID,
        theme: { mode: "dark" },
        sendDataModel: true,
      }),
    ).toEqual({
      version: "v0.9",
      createSurface: {
        surfaceId: "s1",
        catalogId: NOTION_BLOCK_CATALOG_ID,
        theme: { mode: "dark" },
        sendDataModel: true,
      },
    });
  });

  it("constructs updateComponents with catalog components", () => {
    expect(
      updateComponents({
        surfaceId: "s1",
        components: [components.Column, components.Paragraph],
      }),
    ).toEqual({
      version: "v0.9",
      updateComponents: {
        surfaceId: "s1",
        components: [components.Column, components.Paragraph],
      },
    });
  });

  it("constructs updateDataModel without adding omitted fields", () => {
    expect(updateDataModel({ surfaceId: "s1" })).toEqual({
      version: "v0.9",
      updateDataModel: { surfaceId: "s1" },
    });
    expect(
      updateDataModel({
        surfaceId: "s1",
        path: "/user",
        value: { name: "Ada", enabled: true },
      }),
    ).toEqual({
      version: "v0.9",
      updateDataModel: {
        surfaceId: "s1",
        path: "/user",
        value: { name: "Ada", enabled: true },
      },
    });
  });

  it("constructs deleteSurface with the flattened wire shape", () => {
    expect(deleteSurface({ surfaceId: "s1" })).toEqual({
      version: "v0.9",
      deleteSurface: { surfaceId: "s1" },
    });
  });
});

describe("Surface", () => {
  function makeSurface(): Surface {
    const surface = new Surface(ROOT_ID);
    surface.insert({
      id: ROOT_ID,
      component: "Column",
      children: ["p1"],
    });
    surface.insert({ id: "p1", component: "Paragraph", children: ["t1"] });
    surface.insert({ id: "t1", component: "RichText", text: "hello" });
    return surface;
  }

  it("serializes as root plus an insertion-ordered component object", () => {
    const surface = makeSurface();
    const json = surface.toJSON();

    expect(json.root).toBe(ROOT_ID);
    expect(Object.keys(json.components)).toEqual([ROOT_ID, "p1", "t1"]);
    expect(JSON.parse(JSON.stringify(surface))).toEqual(json);
  });

  it("restores its Map and order from JSON", () => {
    const original = makeSurface();
    const wireValue = JSON.parse(JSON.stringify(original)) as SurfaceJSON;
    const restored = Surface.fromJSON(wireValue);

    expect(restored.root).toBe(original.root);
    expect(restored.components).toBeInstanceOf(Map);
    expect([...restored.components.keys()]).toEqual([ROOT_ID, "p1", "t1"]);
    expect(restored.toJSON()).toEqual(original.toJSON());
  });

  it("upserts by component id without moving the existing entry", () => {
    const surface = makeSurface();

    surface.insert({ id: "p1", component: "Paragraph", children: [] });

    expect([...surface.components.keys()]).toEqual([ROOT_ID, "p1", "t1"]);
    expect(surface.components.get("p1")).toEqual({
      id: "p1",
      component: "Paragraph",
      children: [],
    });
  });

  it("emits one create followed by one ordered update using the default catalog", () => {
    const messages = makeSurface().toMessages("my-surface");

    expect(messages).toHaveLength(2);
    expect(messages[0]).toEqual(
      createSurface({
        surfaceId: "my-surface",
        catalogId: NOTION_BLOCK_CATALOG_ID,
      }),
    );
    expect(messages[1]).toEqual(
      updateComponents({
        surfaceId: "my-surface",
        components: [
          { id: ROOT_ID, component: "Column", children: ["p1"] },
          { id: "p1", component: "Paragraph", children: ["t1"] },
          { id: "t1", component: "RichText", text: "hello" },
        ],
      }),
    );
  });

  it("allows an explicit catalog override", () => {
    expect(new Surface(ROOT_ID).toMessages("s1", "catalog:test")[0]).toEqual(
      createSurface({ surfaceId: "s1", catalogId: "catalog:test" }),
    );
  });
});

describe("component IDs", () => {
  it("uses the stable root ID", () => {
    expect(ROOT_ID).toBe("root");
  });

  it("builds synthesized child IDs without normalization", () => {
    expect(childId("parent", "rich_text", 3)).toBe("parent::rich_text/3");
    expect(childId("block-id", "table cell", 0)).toBe("block-id::table cell/0");
  });
});
