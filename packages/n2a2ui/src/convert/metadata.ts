import { load } from "cheerio";
import { imageSize } from "image-size";

export interface ImageDimensions {
  width?: number;
  height?: number;
}

export interface BookmarkMetadata {
  title?: string;
  description?: string;
  image?: string;
}

export async function fetchImageDimensions(
  fetch: typeof globalThis.fetch,
  url: string,
): Promise<ImageDimensions> {
  try {
    const response = await fetch(url);
    const body = new Uint8Array(await response.arrayBuffer());
    const { width, height } = imageSize(body);
    return { width, height };
  } catch {
    return {};
  }
}

export async function fetchBookmarkMetadata(
  fetch: typeof globalThis.fetch,
  url: string,
): Promise<BookmarkMetadata> {
  try {
    const response = await fetch(url, {
      method: "GET",
      headers: { "user-agent": "n2a2ui" },
    });
    const $ = load(await response.text());
    const metaContent = (selector: string): string | undefined => {
      const content = $(selector).first().attr("content");
      return content === "" ? undefined : content;
    };

    const nativeTitle = $("title").first().text().trim() || undefined;
    const title =
      metaContent("meta[property='og:title'], meta[name='og:title']") ??
      metaContent(
        "meta[name='twitter:title'], meta[property='twitter:title']",
      ) ??
      nativeTitle;
    const description =
      metaContent(
        "meta[property='og:description'], meta[name='og:description']",
      ) ??
      metaContent(
        "meta[name='twitter:description'], meta[property='twitter:description']",
      ) ??
      metaContent("meta[name='description']");
    const image =
      metaContent("meta[property='og:image'], meta[name='og:image']") ??
      metaContent("meta[name='twitter:image'], meta[property='twitter:image']");

    const metadata: BookmarkMetadata = {};
    if (title !== undefined) metadata.title = title;
    if (description !== undefined) metadata.description = description;
    if (image !== undefined) metadata.image = image;
    return metadata;
  } catch {
    return {};
  }
}
