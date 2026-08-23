import { Client } from "@notionhq/client";
import { loadEnvFile } from "node:process";
import { describe, expect, it } from "vitest";

import {
  N2A2UIClient,
  NOTION_BLOCK_CATALOG_ID,
  ROOT_ID,
  type Column,
  type ComponentId,
  type Message,
} from "../src/index.js";

try {
  loadEnvFile(new URL("../../../.env", import.meta.url));
} catch (error) {
  if (
    typeof error !== "object" ||
    error === null ||
    !("code" in error) ||
    error.code !== "ENOENT"
  ) {
    throw error;
  }
}

const notionApiKey = process.env.NOTION_API_KEY;
const blockId = process.env.BLOCK_ID;
const hasCredentials = notionApiKey !== undefined && blockId !== undefined;

function rootFrom(message: Message): Column {
  if (!("updateComponents" in message)) {
    throw new Error("expected updateComponents message");
  }
  const root = message.updateComponents.components.find(
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

describe("live Notion conversion", () => {
  it.skipIf(!hasCredentials)(
    "converts eager and streaming surfaces without writing artifacts",
    async () => {
      if (notionApiKey === undefined || blockId === undefined) return;
      const notion = new Client({ auth: notionApiKey });
      const client = new N2A2UIClient({
        notion,
        enableUnsupportedBlock: true,
        enableFetchImageMeta: true,
        enableFetchBookmarkMeta: true,
        enableHtmlEmbed: true,
      });

      const eager = await client.convertBlock(blockId);
      const eagerRoot = eager.components.get(ROOT_ID);
      expect(eager.root).toBe(ROOT_ID);
      expect(eagerRoot?.component).toBe("Column");

      const messages = await collectMessages(
        client.convertBlockStream(blockId, "notion-page"),
      );
      expect(messages.length).toBeGreaterThanOrEqual(2);
      expect(messages[0]).toEqual({
        version: "v0.9",
        createSurface: {
          surfaceId: "notion-page",
          catalogId: NOTION_BLOCK_CATALOG_ID,
        },
      });

      let previousChildren: ComponentId[] = [];
      for (const [index, message] of messages.slice(1).entries()) {
        const children = componentIds(rootFrom(message).children);
        expect(children).toHaveLength(index);
        if (index > 0) {
          expect(children.slice(0, -1)).toEqual(previousChildren);
        }
        previousChildren = children;
      }
      if (eagerRoot?.component !== "Column") {
        throw new Error("eager surface is missing its root Column");
      }
      expect(previousChildren).toEqual(componentIds(eagerRoot.children));
    },
  );
});
