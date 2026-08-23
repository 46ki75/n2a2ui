# n2a2ui

`n2a2ui` converts Notion block trees into [A2UI](https://a2ui.dev) v0.9
surfaces and messages using the
[Elmethis Notion Block Catalog](https://46ki75.github.io/elmethis/a2ui/v0_9/notion_block_catalog.json).
It is a TypeScript package for Node.js 22 or newer and supports ESM and CommonJS.

## Install

This repository uses Node.js 22 and pnpm 9. Install the converter and the
official Notion SDK in your application:

```bash
pnpm add n2a2ui @notionhq/client
```

Create a Notion integration, copy its token, and give the integration access to
the page or block you want to convert. See the
[Notion integration guide](https://developers.notion.com/docs/create-a-notion-integration)
for the workspace setup.

```ts
import { Client } from "@notionhq/client";
import { N2A2UIClient } from "n2a2ui";

const token = process.env.NOTION_API_KEY;
if (token === undefined) throw new Error("NOTION_API_KEY is required");

const notion = new Client({ auth: token });
const client = new N2A2UIClient({ notion });
```

The converter follows `blocks.children.list` pagination at every level, so
`blockId` may identify a page or any block whose children are readable by the
integration.

## Client Options

`N2A2UIClient` accepts these options:

| Option                    | Default            | Behavior                                                                                           |
| ------------------------- | ------------------ | -------------------------------------------------------------------------------------------------- |
| `notion`                  | Required           | An official `@notionhq/client` `Client`.                                                           |
| `fetch`                   | `globalThis.fetch` | Fetch implementation used for optional image and bookmark metadata requests.                       |
| `enableUnsupportedBlock`  | `false`            | Emit an `Unsupported` component for unknown and partial Notion blocks instead of skipping them.    |
| `enableFetchImageMeta`    | `false`            | Download each converted image once and add intrinsic `width` and `height` when detection succeeds. |
| `enableFetchBookmarkMeta` | `false`            | Fetch bookmark-like URLs once and add available title, description, and image metadata.            |
| `enableHtmlEmbed`         | `false`            | Convert an `embed` URL whose path ends in `.html` to `Html` instead of `Bookmark`.                 |

All metadata failures are nonfatal and leave the component's basic URL fields
intact. Each feature flag is normalized to a boolean and exposed through the
client's frozen `options` object.

## Convert A Surface

`convertBlock` eagerly converts the complete descendant tree into a `Surface`.
The surface is a flat adjacency list: components refer to child component IDs,
and the root is always a `Column` with ID `root`.

```ts
const surface = await client.convertBlock(process.env.BLOCK_ID!);

console.log(surface.root); // "root"
console.log(surface.components); // Map<ComponentId, Component>
console.log(JSON.stringify(surface)); // {"root":"root","components":{...}}
```

## Convert Messages

`convertBlockToMessages` returns the complete progressive-render sequence as an
array. It collects exactly the same messages produced by `convertBlockStream`.

```ts
const messages = await client.convertBlockToMessages(
  process.env.BLOCK_ID!,
  "notion-page",
);
```

The sequence is not the two-message output of `Surface.toMessages`. It is:

1. A `createSurface` using the catalog ID exported by `@elmethis/core`.
2. An `updateComponents` containing an empty root `Column`.
3. One `updateComponents` for each converted top-level sibling group.

## Stream Messages

`convertBlockStream` is an async generator. Consume it with `for await` to render
each top-level group without waiting for later recursive conversions:

```ts
for await (const message of client.convertBlockStream(
  process.env.BLOCK_ID!,
  "notion-page",
)) {
  sendToRenderer(message);
}
```

Its chunk semantics are exact and deterministic:

- Before yielding `createSurface`, it collects all pages of the requested
  block's direct children with the official SDK's `collectPaginatedAPI` helper.
- The first update contains `{ id: "root", component: "Column", children: [] }`,
  which mounts the surface even when every child is skipped.
- A sibling group is either one non-list block or one consecutive run of a
  single list style. Bullets and to-dos share an unordered group; numbered items
  form ordered groups. A style change or non-list block starts a new group.
- Each group is converted completely, including descendants and enabled
  metadata requests, before its message is yielded.
- A group update contains all components synthesized for that group followed by
  the root `Column`. The root's `children` is a growing snapshot with the new
  group ID appended. A2UI `updateComponents` upserts by ID, so resending `root`
  preserves a replayable adjacency list.
- A skipped unknown or partial block produces no group update. An all-skipped
  page therefore produces exactly the initial two messages.
- A direct-child fetch error occurs before any message is yielded. An error in a
  later group ends the generator but does not retract messages already yielded.

This streams conversion chunks, not Notion pagination pages: direct-child
pagination finishes before the first message.

## Converted Blocks

- `paragraph`, `heading_1` through `heading_4`, `quote`, `callout`, `toggle`, and
  `divider` become `Paragraph`, `Heading`, `BlockQuote`, `NotionCallout`,
  `Toggle`, and `Divider` components.
- Consecutive `bulleted_list_item` and `to_do` blocks become an unordered `List`;
  consecutive `numbered_list_item` blocks become an ordered `List`. Nested list
  item children are converted recursively, and to-dos receive a stable checked
  or unchecked text marker.
- `code` becomes `CodeBlock`, except the `mermaid` language becomes `Mermaid`.
  `equation` becomes `Katex`.
- `image`, `file`, `pdf`, `audio`, and `video` become `BlockImage`, `File`,
  `File`, `Audio`, and `Video`.
- `bookmark`, ordinary `embed`, and `link_preview` become `Bookmark`.
  `child_page` and `child_database` become titled Notion `Bookmark` links.
- `column_list`, `column`, and `synced_block` become `ColumnList` or `Column`.
- `table` and `table_row` become `Table`, `TableRow`, and `TableCell`, including
  column-header and row-header flags.
- `tab` becomes `ContentTabs`; each direct paragraph child becomes one
  `ContentTab`, using that paragraph's rich text as its label and its children as
  content. Other direct tab children are ignored.

Rich-text runs become `RichText` or `LinkText`; custom emoji mentions become
`Icon`, and inline equations use the `katex` decoration. Supported annotations
and Notion colors are mapped onto catalog fields. Callout emoji, built-in icons,
custom emoji, and file icons are represented by `NotionCallout.icon`.

With `enableUnsupportedBlock: false`, unrecognized and partial blocks are
skipped. With it enabled, they become `Unsupported` with a diagnostic `details`
string. Supported media returned as an unfinished `file_upload` also becomes
`Unsupported` because it has no usable URL.

## Metadata And HTML

Image metadata downloads the image body and uses `image-size` to detect its
dimensions. Bookmark metadata uses `cheerio` and prefers Open Graph fields,
then Twitter fields, then native `<title>` and description metadata. It applies
to `bookmark`, `embed`, and `link_preview` URLs that remain bookmarks. A Notion
bookmark caption is user-authored content and is not used as the metadata
description.

When `enableHtmlEmbed` is enabled, an `embed` URL is considered HTML when its URL
path ends in `.html`, case-insensitively and ignoring its query or fragment. It
becomes `{ component: "Html", src: url }` without a metadata fetch; other embeds
follow normal bookmark behavior. Notion-hosted URLs may be signed and
time-limited, so consumers should render them promptly.

## Stable IDs

Notion block IDs are reused for block-derived components. Synthesized IDs are
deterministic:

- The surface root is `root`.
- Rich-text runs, table cells, and to-do markers use
  `<parent>::<slot>/<index>`.
- A grouped list uses `<first-item-id>::list`.

Do not replace these IDs with random values. Stable IDs make repeated conversion
and streamed upserts predictable.

## Development And Tests

Use Node.js 22 and the pnpm version declared by `packageManager`:

```bash
pnpm install --frozen-lockfile
just ci
```

`just test` runs the hermetic Vitest suite and excludes `*.live.test.ts`.
`just coverage` writes text, HTML, and LCOV coverage reports. The approval-tier
live test uses the real Notion API:

```bash
cp .env.example .env
# Set NOTION_API_KEY and BLOCK_ID in .env, then:
just test-live
```

The live test skips when either variable is absent. `just ci-live` runs the
normal checks first and then the live test.
