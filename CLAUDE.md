# CLAUDE.md

This file provides repository guidance for coding agents.

## Package

`n2a2ui` is a strict TypeScript package under `packages/n2a2ui`, targeting
Node.js 22. It converts Notion block trees into A2UI v0.9 `Surface` objects or
progressive message sequences. It builds ESM, CommonJS, and declarations from
`packages/n2a2ui/src/index.ts` into `packages/n2a2ui/dist/`.

Use pnpm 9 as declared by `packageManager`. The `Justfile` is the command source
of truth; automation and contributors should invoke its recipes rather than
duplicating underlying commands.

## Commands

```bash
just fmt          # Prettier write
just fmt-check    # Prettier check
just lint         # ESLint
just typecheck    # tsc --noEmit
just build        # ESM, CommonJS, and declarations
just test         # Hermetic Vitest suite; excludes live tests
just ci           # Format, lint, typecheck, test, and build
just test-live    # Real Notion API test; NOTION_API_KEY + BLOCK_ID
just ci-live      # Hermetic CI plus live test
just coverage     # Text, HTML, and LCOV reports under coverage/
just coverage-ci  # Root lcov.info for Codecov
```

`.env` at the repository root is supported for live test variables. The live
test skips when either variable is absent.

## Layout

- `packages/n2a2ui/src/client.ts`: public `N2A2UIClient`, eager conversion, async-generator
  streaming, and message collection.
- `packages/n2a2ui/src/convert/converter.ts`: recursive Notion traversal, pagination, sibling
  grouping, block mappings, and chunk construction.
- `packages/n2a2ui/src/convert/rich-text.ts`: one deterministic A2UI component per Notion
  rich-text run.
- `packages/n2a2ui/src/convert/metadata.ts`: nonfatal image dimension and bookmark metadata
  fetching.
- `packages/n2a2ui/src/convert/color.ts`: Notion foreground/background color mapping.
- `packages/n2a2ui/src/a2ui.ts`: catalog-derived component types, v0.9 message helpers, and the
  ordered `Surface` adjacency list.
- `packages/n2a2ui/src/id.ts`: stable synthesized ID rules.
- `packages/n2a2ui/src/index.ts`: package public exports.
- `packages/n2a2ui/tests/*.test.ts`: hermetic tests; `*.live.test.ts`:
  approval-tier tests.

## Dependencies

- `@notionhq/client`: official API client, generated response types,
  `collectPaginatedAPI`, and `isFullBlock`.
- `@a2ui/web_core/v0_9`: protocol message, dynamic value, child-list, and base
  component types.
- `@elmethis/core`: owner of the Notion Block Catalog component APIs and
  `NOTION_BLOCK_CATALOG_ID`.
- `cheerio` and `image-size`: optional bookmark and image metadata extraction.
- `zod`: declared runtime dependency; do not assume it is part of the converter
  path without checking imports.
- TypeScript, tsdown, ESLint, Prettier, and Vitest are development tooling.

The catalog is not owned or vendored here. Component schema and catalog-ID
changes belong in `@elmethis/core`; update this package's dependency and inferred
types afterward. Do not duplicate catalog definitions locally.

## Notion API Model

`N2A2UIClient` requires an official `Client` instance. `fetch` defaults to
`globalThis.fetch`, and all four feature flags default to `false`:
`enableUnsupportedBlock`, `enableFetchImageMeta`,
`enableFetchBookmarkMeta`, and `enableHtmlEmbed`. Normalized flags are frozen in
`client.options`.

Children are always fetched with:

```ts
collectPaginatedAPI(notion.blocks.children.list, { block_id: parentId });
```

This collects every page before grouping siblings, including recursively, so a
list can span Notion response pages. Keep the bound SDK method and let the helper
manage `start_cursor`.

The SDK's children response is modeled as
`BlockObjectResponse | PartialBlockObjectResponse`. Preserve the `NotionBlock`
union and use `isFullBlock` before accessing `type` or type-specific payloads.
Partial blocks are skipped by default or emitted as `Unsupported` when enabled.
Do not replace current SDK types with handwritten block interfaces.

## Surface And Messages

`Surface` is a flat adjacency list with `root: ComponentId` and an insertion-
ordered `Map<ComponentId, Component>`. `insert` keys by `component.id`, `toJSON`
serializes the map as an object, and `Surface.toMessages` emits one
`createSurface` plus one full `updateComponents`.

`convertBlock(blockId)` recursively converts the whole tree. It inserts the
`root` `Column` first, followed by converter output in deterministic order.

`convertBlockStream(blockId, surfaceId)` has different, load-bearing semantics:

1. Collect every page of the requested block's direct children before the first
   yield. A failure here delivers no messages.
2. Yield `createSurface` with `NOTION_BLOCK_CATALOG_ID`.
3. Yield `updateComponents` with an empty `root` `Column` so the surface mounts
   and an all-skipped page remains valid.
4. Group top-level siblings. Each non-list block is one group. Consecutive
   bullets and to-dos form one unordered group; consecutive numbered items form
   one ordered group. Style changes and non-list blocks split groups.
5. Convert one complete group at a time. Each yielded update contains that
   group's components in converter order and the updated `root` last. Root
   children are a growing snapshot. Skipped single groups emit no update.
6. Recursive fetches and metadata complete before their group is yielded. Later
   failures end the generator without retracting prior messages.

`updateComponents` is upsert-by-ID, making repeated root snapshots replay-safe.
This is chunked conversion, not page-by-page Notion streaming.

`convertBlockToMessages` iterates and collects exactly the generator sequence.
Its shape is `createSurface + empty-root update + N group updates`, not the
two-message `Surface.toMessages` shape.

## Conversion Invariants

- Block-derived components reuse Notion IDs. `ROOT_ID` is `root`.
- `childId(parent, slot, index)` is exactly `<parent>::<slot>/<index>` and is
  used for rich-text runs, table cells, and to-do markers.
- List IDs are exactly `<first-item-id>::list`. Do not introduce random IDs.
- Child components are appended before their parent. Eager and reconstructed
  streaming surfaces must preserve identical `Map` order.
- A sibling list group contains one style: bullets and to-dos are unordered;
  numbered items are ordered. Grouping must work across pagination boundaries.
- To-dos synthesize `RichText` prefixes `"☐ "` or `"☑ "` before their text.
- Tabs use only direct paragraph children. Paragraph rich text becomes
  `ContentTab.label`; paragraph children become `ContentTab.content`.
- Rich-text custom emoji mentions become `Icon`; linked runs become `LinkText`;
  equations become `RichText` with `katex`; other runs become `RichText`.
- Callouts become `NotionCallout` with structured emoji/image icons and mapped
  color/variant, not synthesized leading children.
- Tables ignore non-row children before selecting a column-header row. First
  cells are marked as row headers when requested.
- `code` with language `mermaid` becomes `Mermaid`; other code becomes
  `CodeBlock`. Block equations become `Katex`.
- Bookmark captions never populate metadata descriptions. Child pages and
  databases become titled Notion bookmark URLs.
- Missing `file_upload` URLs become `Unsupported` even when general unsupported
  block emission is disabled.

## Metadata And HTML

All metadata options are opt-in and add one fetch per applicable component.
Image bodies are parsed by `image-size`. Bookmark HTML is parsed by `cheerio`
with priority Open Graph, Twitter, then native title/description. Metadata errors
return empty fields and never fail conversion. Bookmark metadata applies to
bookmark, link-preview, and non-HTML embed URLs.

With `enableHtmlEmbed`, only an embed URL whose path ends in `.html`
case-insensitively, ignoring query and fragment, becomes `Html { src }`. It is
not fetched by the converter. All other embeds use bookmark behavior.

## Conventions

- Keep source imports ESM-compatible with `.js` suffixes.
- Preserve strict typing, including `noUncheckedIndexedAccess` and
  `exactOptionalPropertyTypes`; omit absent optional fields rather than assigning
  `undefined`.
- Export public API through `packages/n2a2ui/src/index.ts` and verify both ESM
  and CommonJS builds.
- Add hermetic tests for conversion changes and live coverage only when real API
  behavior is necessary.
- PRs target `develop` or `release/*` per the pull request template; use `main`
  only when those branches do not exist.
