import { NOTION_BLOCK_CATALOG_ID } from "@elmethis/core";
import type {
  AudioApi,
  BlockImageApi,
  BlockQuoteApi,
  BookmarkApi,
  CalloutApi,
  CodeBlockApi,
  ColumnApi,
  ColumnListApi,
  ContentTabApi,
  ContentTabsApi,
  DividerApi,
  FileApi,
  HeadingApi,
  HtmlApi,
  IconApi,
  KatexApi,
  LinkTextApi,
  ListApi,
  ListItemApi,
  MermaidApi,
  NotionCalloutApi,
  ParagraphApi,
  RichTextApi,
  RowApi,
  TableApi,
  TableCellApi,
  TableRowApi,
  ToggleApi,
  UnsupportedApi,
  VideoApi,
} from "@elmethis/core";
import type {
  ComponentApi,
  ComponentId,
  CreateSurfaceMessage as A2uiCreateSurfaceMessage,
  DeleteSurfaceMessage as A2uiDeleteSurfaceMessage,
  InferredComponentApiSchemaType,
  UpdateComponentsMessage as A2uiUpdateComponentsMessage,
  UpdateDataModelMessage as A2uiUpdateDataModelMessage,
} from "@a2ui/web_core/v0_9";

export { NOTION_BLOCK_CATALOG_ID };
export type {
  A2uiMessage,
  ChildList,
  ComponentId,
  CreateSurfaceMessage,
  DeleteSurfaceMessage,
  DynamicString,
  UpdateComponentsMessage,
  UpdateDataModelMessage,
} from "@a2ui/web_core/v0_9";

export const VERSION = "v0.9" as const;

export type ComponentFromApi<Name extends string, Api extends ComponentApi> = {
  id: ComponentId;
  component: Name;
} & Omit<InferredComponentApiSchemaType<Api>, "component" | "id">;

export type RichText = ComponentFromApi<"RichText", typeof RichTextApi>;
export type LinkText = ComponentFromApi<"LinkText", typeof LinkTextApi>;
export type Icon = ComponentFromApi<"Icon", typeof IconApi>;
export type Row = ComponentFromApi<"Row", typeof RowApi>;
export type Column = ComponentFromApi<"Column", typeof ColumnApi>;
export type ColumnList = ComponentFromApi<"ColumnList", typeof ColumnListApi>;
export type Heading = ComponentFromApi<"Heading", typeof HeadingApi>;
export type Paragraph = ComponentFromApi<"Paragraph", typeof ParagraphApi>;
export type List = ComponentFromApi<"List", typeof ListApi>;
export type ListItem = ComponentFromApi<"ListItem", typeof ListItemApi>;
export type BlockQuote = ComponentFromApi<"BlockQuote", typeof BlockQuoteApi>;
export type Callout = ComponentFromApi<"Callout", typeof CalloutApi>;
export type NotionCallout = ComponentFromApi<
  "NotionCallout",
  typeof NotionCalloutApi
>;
export type Divider = ComponentFromApi<"Divider", typeof DividerApi>;
export type Toggle = ComponentFromApi<"Toggle", typeof ToggleApi>;
export type Bookmark = ComponentFromApi<"Bookmark", typeof BookmarkApi>;
export type File = ComponentFromApi<"File", typeof FileApi>;
export type Audio = ComponentFromApi<"Audio", typeof AudioApi>;
export type Video = ComponentFromApi<"Video", typeof VideoApi>;
export type BlockImage = ComponentFromApi<"BlockImage", typeof BlockImageApi>;
export type Html = ComponentFromApi<"Html", typeof HtmlApi>;
export type CodeBlock = ComponentFromApi<"CodeBlock", typeof CodeBlockApi>;
export type Katex = ComponentFromApi<"Katex", typeof KatexApi>;
export type Mermaid = ComponentFromApi<"Mermaid", typeof MermaidApi>;
export type ContentTab = ComponentFromApi<"ContentTab", typeof ContentTabApi>;
export type ContentTabs = ComponentFromApi<
  "ContentTabs",
  typeof ContentTabsApi
>;
export type Table = ComponentFromApi<"Table", typeof TableApi>;
export type TableRow = ComponentFromApi<"TableRow", typeof TableRowApi>;
export type TableCell = ComponentFromApi<"TableCell", typeof TableCellApi>;
export type Unsupported = ComponentFromApi<
  "Unsupported",
  typeof UnsupportedApi
>;

export type Component =
  | RichText
  | LinkText
  | Icon
  | Row
  | Column
  | ColumnList
  | Heading
  | Paragraph
  | List
  | ListItem
  | BlockQuote
  | Callout
  | NotionCallout
  | Divider
  | Toggle
  | Bookmark
  | File
  | Audio
  | Video
  | BlockImage
  | Html
  | CodeBlock
  | Katex
  | Mermaid
  | ContentTab
  | ContentTabs
  | Table
  | TableRow
  | TableCell
  | Unsupported;

export type JsonValue =
  null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export type CreateSurfacePayload = Omit<
  A2uiCreateSurfaceMessage["createSurface"],
  "theme"
> & {
  theme?: JsonValue;
};

export type UpdateComponentsPayload = Omit<
  A2uiUpdateComponentsMessage["updateComponents"],
  "components"
> & {
  components: Component[];
};

export type UpdateDataModelPayload = Omit<
  A2uiUpdateDataModelMessage["updateDataModel"],
  "value"
> & {
  value?: JsonValue;
};

export type DeleteSurfacePayload = A2uiDeleteSurfaceMessage["deleteSurface"];

export type V09CreateSurfaceMessage = {
  version: typeof VERSION;
  createSurface: CreateSurfacePayload;
};

export type V09UpdateComponentsMessage = {
  version: typeof VERSION;
  updateComponents: UpdateComponentsPayload;
};

export type V09UpdateDataModelMessage = {
  version: typeof VERSION;
  updateDataModel: UpdateDataModelPayload;
};

export type V09DeleteSurfaceMessage = {
  version: typeof VERSION;
  deleteSurface: DeleteSurfacePayload;
};

export type Message =
  | V09CreateSurfaceMessage
  | V09UpdateComponentsMessage
  | V09UpdateDataModelMessage
  | V09DeleteSurfaceMessage;

export function createSurface(
  payload: CreateSurfacePayload,
): V09CreateSurfaceMessage {
  return { version: VERSION, createSurface: payload };
}

export function updateComponents(
  payload: UpdateComponentsPayload,
): V09UpdateComponentsMessage {
  return { version: VERSION, updateComponents: payload };
}

export function updateDataModel(
  payload: UpdateDataModelPayload,
): V09UpdateDataModelMessage {
  return { version: VERSION, updateDataModel: payload };
}

export function deleteSurface(
  payload: DeleteSurfacePayload,
): V09DeleteSurfaceMessage {
  return { version: VERSION, deleteSurface: payload };
}

export interface SurfaceJSON {
  root: ComponentId;
  components: Record<ComponentId, Component>;
}

export class Surface {
  root: ComponentId;
  readonly components: Map<ComponentId, Component>;

  constructor(
    root: ComponentId,
    components?: Iterable<readonly [ComponentId, Component]>,
  ) {
    this.root = root;
    this.components = new Map(components);
  }

  insert(component: Component): void {
    this.components.set(component.id, component);
  }

  toJSON(): SurfaceJSON {
    return {
      root: this.root,
      components: Object.fromEntries(this.components),
    };
  }

  static fromJSON(value: SurfaceJSON): Surface {
    return new Surface(value.root, Object.entries(value.components));
  }

  toMessages(
    surfaceId: string,
    catalogId: string = NOTION_BLOCK_CATALOG_ID,
  ): [V09CreateSurfaceMessage, V09UpdateComponentsMessage] {
    return [
      createSurface({ surfaceId, catalogId }),
      updateComponents({
        surfaceId,
        components: [...this.components.values()],
      }),
    ];
  }
}
