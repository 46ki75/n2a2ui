import type { Client as NotionClient } from "@notionhq/client";

import {
  NOTION_BLOCK_CATALOG_ID,
  Surface,
  createSurface,
  updateComponents,
  type Column,
  type ComponentId,
  type Message,
} from "./a2ui.js";
import { Converter, topLevelGroups } from "./convert/converter.js";
import { ROOT_ID } from "./id.js";

export interface N2A2UIClientOptions {
  notion: NotionClient;
  fetch?: typeof globalThis.fetch;
  enableUnsupportedBlock?: boolean;
  enableFetchImageMeta?: boolean;
  enableFetchBookmarkMeta?: boolean;
  enableHtmlEmbed?: boolean;
}

export class N2A2UIClient {
  readonly notion: NotionClient;
  readonly fetch: typeof globalThis.fetch;
  readonly options: Readonly<{
    enableUnsupportedBlock: boolean;
    enableFetchImageMeta: boolean;
    enableFetchBookmarkMeta: boolean;
    enableHtmlEmbed: boolean;
  }>;

  constructor(options: N2A2UIClientOptions) {
    this.notion = options.notion;
    this.fetch = options.fetch ?? globalThis.fetch;
    this.options = Object.freeze({
      enableUnsupportedBlock: options.enableUnsupportedBlock ?? false,
      enableFetchImageMeta: options.enableFetchImageMeta ?? false,
      enableFetchBookmarkMeta: options.enableFetchBookmarkMeta ?? false,
      enableHtmlEmbed: options.enableHtmlEmbed ?? false,
    });
  }

  async convertBlock(blockId: string): Promise<Surface> {
    const [rootChildren, components] =
      await this.converter().convertChildren(blockId);
    const surface = new Surface(ROOT_ID);
    surface.insert(rootColumn(rootChildren));
    for (const component of components) surface.insert(component);
    return surface;
  }

  async *convertBlockStream(
    blockId: string,
    surfaceId: string,
  ): AsyncGenerator<Message, void, void> {
    const converter = this.converter();
    const blocks = await converter.fetchChildren(blockId);

    yield createSurface({
      surfaceId,
      catalogId: NOTION_BLOCK_CATALOG_ID,
    });

    const accumulated: ComponentId[] = [];
    yield updateComponents({
      surfaceId,
      components: [rootColumn(accumulated)],
    });

    for (const group of topLevelGroups(blocks)) {
      const chunk =
        group.kind === "list"
          ? await converter.convertListGroupToChunk(group.blocks, group.style)
          : await converter.convertSingleBlockToChunk(group.block);
      if (chunk === undefined) continue;

      const [chunkId, components] = chunk;
      accumulated.push(chunkId);
      yield updateComponents({
        surfaceId,
        components: [...components, rootColumn(accumulated)],
      });
    }
  }

  async convertBlockToMessages(
    blockId: string,
    surfaceId: string,
  ): Promise<Message[]> {
    const messages: Message[] = [];
    for await (const message of this.convertBlockStream(blockId, surfaceId)) {
      messages.push(message);
    }
    return messages;
  }

  private converter(): Converter {
    return new Converter({
      notion: this.notion,
      fetch: this.fetch,
      ...this.options,
    });
  }
}

function rootColumn(children: readonly ComponentId[]): Column {
  return {
    id: ROOT_ID,
    component: "Column",
    children: [...children],
  };
}
