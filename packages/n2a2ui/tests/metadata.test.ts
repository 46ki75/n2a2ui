import { describe, expect, it, vi } from "vitest";

import {
  fetchBookmarkMetadata,
  fetchImageDimensions,
} from "../src/convert/metadata.js";

function responseFetch(
  body: BodyInit | null,
  init?: ResponseInit,
): typeof globalThis.fetch {
  return async () => new Response(body, init);
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
    0x00,
    0x00,
    0x00,
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

describe("fetchImageDimensions", () => {
  it("buffers and parses a non-2xx image response", async () => {
    const result = await fetchImageDimensions(
      responseFetch(png(320, 180), { status: 404 }),
      "https://example.com/image.png",
    );

    expect(result).toEqual({ width: 320, height: 180 });
  });

  it("returns an empty object for a malformed image", async () => {
    const result = await fetchImageDimensions(
      responseFetch("not an image"),
      "https://example.com/image",
    );

    expect(result).toEqual({});
  });

  it("silently handles fetch and body errors", async () => {
    const failedFetch: typeof globalThis.fetch = async () => {
      throw new Error("network failed");
    };
    const failedBody = new Response();
    vi.spyOn(failedBody, "arrayBuffer").mockRejectedValue(
      new Error("body failed"),
    );
    const failedBodyFetch: typeof globalThis.fetch = async () => failedBody;

    await expect(
      fetchImageDimensions(failedFetch, "https://example.com/image"),
    ).resolves.toEqual({});
    await expect(
      fetchImageDimensions(failedBodyFetch, "https://example.com/image"),
    ).resolves.toEqual({});
  });
});

describe("fetchBookmarkMetadata", () => {
  it("prefers Open Graph metadata over Twitter and native metadata", async () => {
    const html = `
      <title>Native title</title>
      <meta name="description" content="Native description">
      <meta name="twitter:title" content="Twitter title">
      <meta name="twitter:description" content="Twitter description">
      <meta name="twitter:image" content="/twitter.jpg">
      <meta property="og:title" content="OG title">
      <meta property="og:description" content="OG description">
      <meta property="og:image" content="/og.jpg">
    `;

    const result = await fetchBookmarkMetadata(
      responseFetch(html),
      "https://example.com/article",
    );

    expect(result).toEqual({
      title: "OG title",
      description: "OG description",
      image: "/og.jpg",
    });
  });

  it("falls back through Twitter metadata to native metadata", async () => {
    const twitterHtml = `
      <title>Native title</title>
      <meta name="og:title" content="">
      <meta property="twitter:title" content="Twitter title">
      <meta property="twitter:description" content="Twitter description">
      <meta property="twitter:image" content="images/twitter.jpg">
    `;
    const nativeHtml = `
      <title>\n  Native title  \n</title>
      <meta name="description" content="Native description">
    `;

    await expect(
      fetchBookmarkMetadata(
        responseFetch(twitterHtml),
        "https://example.com/twitter",
      ),
    ).resolves.toEqual({
      title: "Twitter title",
      description: "Twitter description",
      image: "images/twitter.jpg",
    });
    await expect(
      fetchBookmarkMetadata(
        responseFetch(nativeHtml),
        "https://example.com/native",
      ),
    ).resolves.toEqual({
      title: "Native title",
      description: "Native description",
    });
  });

  it("preserves non-empty meta content without trimming it", async () => {
    const result = await fetchBookmarkMetadata(
      responseFetch('<meta property="og:title" content="  OG title  ">'),
      "https://example.com/article",
    );

    expect(result).toEqual({ title: "  OG title  " });
  });

  it("parses non-2xx bodies and sends the converter user-agent", async () => {
    const url = "https://example.com/unavailable";
    let requestedInput: RequestInfo | URL | undefined;
    let requestedInit: RequestInit | undefined;
    const fetch: typeof globalThis.fetch = async (input, init) => {
      requestedInput = input;
      requestedInit = init;
      return new Response("<title>Error page</title>", { status: 503 });
    };

    await expect(fetchBookmarkMetadata(fetch, url)).resolves.toEqual({
      title: "Error page",
    });
    expect(requestedInput).toBe(url);
    expect(requestedInit?.method).toBe("GET");
    expect(new Headers(requestedInit?.headers).get("user-agent")).toBe(
      "n2a2ui",
    );
  });

  it("silently handles fetch and body errors", async () => {
    const failedFetch: typeof globalThis.fetch = async () => {
      throw new Error("network failed");
    };
    const failedBody = new Response();
    vi.spyOn(failedBody, "text").mockRejectedValue(new Error("body failed"));
    const failedBodyFetch: typeof globalThis.fetch = async () => failedBody;

    await expect(
      fetchBookmarkMetadata(failedFetch, "https://example.com/article"),
    ).resolves.toEqual({});
    await expect(
      fetchBookmarkMetadata(failedBodyFetch, "https://example.com/article"),
    ).resolves.toEqual({});
  });
});
