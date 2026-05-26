/**
 * @typedef {Object} MediaResult
 * @property {string} url
 * @property {Uint8Array} [data]
 * @property {string|null} [mimeType]
 * @property {string} [error]
 */

/**
 * @typedef {Object} BooruPostData
 * @property {string} pageUrl
 * @property {string|null} sourceUrl
 * @property {string|null} artistName
 * @property {string|null} artistUrl
 * @property {MediaResult|null} image
 */

/**
 * @typedef {Object} SiteConfig
 * @property {string} name
 * @property {(hostname: string) => boolean} match
 * @property {string[]} sidebarSelectors
 * @property {string[]} tagListSelectors
 * @property {string[]} sourceSelectors
 * @property {string[]} originalImageSelectors
 * @property {string|null} sourceDataAttribute
 * @property {string|null} sourceTextPattern
 * @property {"direct"|"background"} fetchMethod
 */

import { create_capture_button } from "./content2.js";
/// <reference path="extension_config.d.ts" />

/** @type {SiteConfig[]} */
const siteConfigs = [
  {
    name: "gelbooru",
    match: (hostname) => hostname.includes("gelbooru.com"),
    sidebarSelectors: [".aside", "aside"],
    tagListSelectors: ["ul#tag-list"],
    sourceSelectors: ["[data-source]"],
    originalImageSelectors: [
      'a[href*="/images/"]',
      'meta[property="og:image"]',
      'meta[name="twitter:image"]',
    ],
    sourceDataAttribute: "data-source",
    sourceTextPattern: null,
    fetchMethod: "background",
  },
  {
    name: "safebooru",
    match: (hostname) => hostname.includes("safebooru.org"),
    sidebarSelectors: ["div.sidebar"],
    tagListSelectors: ["ul#tag-sidebar"],
    sourceSelectors: ["#stats"],
    originalImageSelectors: ['a[href*="/images/"]'],
    sourceDataAttribute: null,
    sourceTextPattern: "Source:",
    fetchMethod: "direct",
  },
  {
    name: "danbooru",
    match: (hostname) => hostname.includes("danbooru.donmai.us"),
    sidebarSelectors: ["aside", "section.sidebar", "#sidebar"],
    tagListSelectors: ["ul.tag-list", "section.tag-list"],
    sourceSelectors: [".image-container", "#post-info"],
    originalImageSelectors: ['a[href*="/original/"]', 'a.image-view-original-link'],
    sourceDataAttribute: "data-source",
    sourceTextPattern: "Source:",
    fetchMethod: "direct",
  },
  {
    name: "rule34",
    match: (hostname) => hostname.includes("rule34.xxx"),
    sidebarSelectors: [".aside", "aside"],
    tagListSelectors: ["ul#tag-list", "ul#tag-sidebar"],
    sourceSelectors: ["[data-source]", "#stats"],
    originalImageSelectors: [
      'meta[property="og:image"]',
      'a[href*="/images/"]',
    ],
    sourceDataAttribute: "data-source",
    sourceTextPattern: "Source:",
    fetchMethod: "direct",
  },
  {
    name: "generic",
    match: (hostname) => hostname.includes("booru"),
    sidebarSelectors: [".aside", "div.sidebar", "aside", "#sidebar"],
    tagListSelectors: ["ul#tag-list", "ul#tag-sidebar", "ul.tag-list"],
    sourceSelectors: ["[data-source]", "#stats", "#post-info"],
    originalImageSelectors: [
      'a[href*="/images/"]',
      'meta[property="og:image"]',
      'meta[name="twitter:image"]',
    ],
    sourceDataAttribute: "data-source",
    sourceTextPattern: "Source:",
    fetchMethod: "direct",
  },
];

/** @type {() => void} */
export const do_booru = () => {
  const BUTTON_MARKER = "sunwet-booru-capture-btn";

  /**
   * Get the site config for the current hostname
   * @type {() => SiteConfig|null}
   */
  const getSiteConfig = () => {
    const hostname = window.location.hostname;
    for (const config of siteConfigs) {
      if (config.match(hostname)) {
        return config;
      }
    }
    return null;
  };

  const siteConfig = getSiteConfig();
  if (!siteConfig) {
    console.log("Booru: No matching site config found");
    return;
  }

  console.log(`Booru: Using config for ${siteConfig.name}`);

  /**
   * Download media via direct fetch
   * @type {(url: string) => Promise<MediaResult>}
   */
  const downloadMediaDirect = async (url) => {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const mimeType = response.headers.get("Content-Type");
    const buffer = await response.arrayBuffer();
    return { url, data: new Uint8Array(buffer), mimeType };
  };

  /**
   * Download media via background script (bypasses CORS)
   * @type {(url: string) => Promise<MediaResult>}
   */
  const downloadMediaBackground = async (url) => {
    /** @type {{runtime: {sendMessage: (msg: any) => Promise<any>}}} */
    const browser = /** @type {any} */ (globalThis).browser;
    const resp = await browser.runtime.sendMessage({ type: "fetch_media", url, referer: window.location.href });
    if (resp.error) throw new Error(resp.error);
    return { url, data: new Uint8Array(resp.data), mimeType: resp.mimeType };
  };

  /**
   * Download media using the site-configured method
   * @type {(url: string) => Promise<MediaResult>}
   */
  const downloadMedia = async (url) => {
    try {
      if (siteConfig.fetchMethod === "background") {
        return await downloadMediaBackground(url);
      }
      return await downloadMediaDirect(url);
    } catch (err) {
      console.error("[sunwet] downloadMedia failed:", url, /** @type {Error} */ (err).message);
      return { url, error: /** @type {Error} */ (err).message };
    }
  };

  /**
   * Check if this is an image/post page
   * @type {() => boolean}
   */
  const isImagePage = () => {
    const mainImage = document.querySelector("img#image");
    if (mainImage) return true;
    const url = window.location.href;
    return url.includes("page=post") && url.includes("s=view");
  };

  /**
   * Get element by trying multiple selectors
   * @type {(selectors: string[]) => Element|null}
   */
  const queryFirst = (selectors) => {
    for (const selector of selectors) {
      const el = document.querySelector(selector);
      if (el) return el;
    }
    return null;
  };

  /**
   * Get the sidebar element using site-specific selectors
   * @type {() => Element|null}
   */
  const getSidebar = () => {
    return queryFirst(siteConfig.sidebarSelectors);
  };

  /**
   * Get the tag list element using site-specific selectors
   * @type {() => Element|null}
   */
  const getTagList = () => {
    return queryFirst(siteConfig.tagListSelectors);
  };

  /**
   * Extract the first artist tag name and its booru link URL.
   * @type {() => { name: string|null, url: string|null }}
   */
  const extractArtist = () => {
    const tagList = getTagList();
    if (!tagList) return { name: null, url: null };

    const artistItems = tagList.querySelectorAll("li.tag-type-artist");
    if (artistItems.length === 0) return { name: null, url: null };

    const item = artistItems[0];
    const links = item.querySelectorAll("a");
    for (const link of links) {
      const href = link.getAttribute("href") || "";
      if (href.includes("page=post") && href.includes("tags=")) {
        const name = link.textContent?.trim() ?? null;
        const url = link.href;
        return { name, url };
      }
    }

    return { name: null, url: null };
  };

  /**
   * Get the source URL for the image using site-specific methods
   * @type {() => string|null}
   */
  const getSourceUrl = () => {
    if (siteConfig.sourceDataAttribute) {
      const el = document.querySelector(`[${siteConfig.sourceDataAttribute}]`);
      if (el) {
        const source = el.getAttribute(siteConfig.sourceDataAttribute);
        if (source && source.trim()) return source.trim();
      }
    }

    for (const selector of siteConfig.sourceSelectors) {
      const container = document.querySelector(selector);
      if (!container) continue;

      if (siteConfig.sourceTextPattern) {
        const walker = document.createTreeWalker(
          container,
          NodeFilter.SHOW_TEXT,
          null,
        );
        let node;
        while ((node = walker.nextNode())) {
          if (node.textContent?.includes(siteConfig.sourceTextPattern)) {
            const parent = node.parentElement;
            if (parent) {
              const link = parent.querySelector("a[href]");
              if (link) {
                const href = link.getAttribute("href");
                if (
                  href &&
                  !href.startsWith("#") &&
                  !href.startsWith("javascript:")
                ) {
                  return href;
                }
              }
              const nextEl = parent.nextElementSibling;
              if (nextEl?.tagName === "A") {
                const href = nextEl.getAttribute("href");
                if (href && !href.startsWith("#")) return href;
              }
            }
          }
        }
      }

      const links = container.querySelectorAll('a[href^="http"]');
      for (const link of links) {
        const href = link.getAttribute("href");
        if (
          href &&
          !href.includes("saucenao") &&
          !href.includes("iqdb") &&
          !href.includes("waifu2x")
        ) {
          const text = link.textContent?.toLowerCase() || "";
          if (
            !text.includes("previous") &&
            !text.includes("next") &&
            !text.includes("edit")
          ) {
            return href;
          }
        }
      }
    }

    return null;
  };

  /**
   * Get the original quality image URL using site-specific methods
   * @type {() => string|null}
   */
  const getOriginalImageUrl = () => {
    for (const selector of siteConfig.originalImageSelectors) {
      const el = document.querySelector(selector);
      if (!el) continue;

      if (el.tagName === "META") {
        const content = el.getAttribute("content");
        if (content && content.includes("/images/")) {
          return content;
        }
        continue;
      }

      if (el.tagName === "A") {
        const href = el.getAttribute("href");
        const text = el.textContent?.toLowerCase().trim() || "";

        if (text === "original image" || text === "original") {
          if (href) {
            if (href.startsWith("//")) return "https:" + href;
            if (href.startsWith("/")) return window.location.origin + href;
            return href;
          }
        }

        if (href?.includes("/images/")) {
          if (href.startsWith("//")) return "https:" + href;
          if (href.startsWith("/")) return window.location.origin + href;
          return href;
        }
      }
    }

    const links = document.querySelectorAll("a");
    for (const link of links) {
      const text = link.textContent?.toLowerCase().trim();
      if (text === "original image" || text === "original") {
        const href = link.getAttribute("href");
        if (href && (href.includes("/images/") || href.includes("/original/"))) {
          if (href.startsWith("//")) return "https:" + href;
          if (href.startsWith("/")) return window.location.origin + href;
          return href;
        }
      }
    }

    const ogImage = document.querySelector('meta[property="og:image"]');
    if (ogImage) {
      const content = ogImage.getAttribute("content");
      if (content && (content.includes("/images/") || content.includes("/original/"))) {
        return content;
      }
    }

    return null;
  };

  /**
   * Extract all post data
   * @type {() => Promise<BooruPostData>}
   */
  const extractPostData = async () => {
    const pageUrl = window.location.href;
    const sourceUrl = getSourceUrl();
    const artist = extractArtist();
    const originalImageUrl = getOriginalImageUrl();

    /** @type {MediaResult|null} */
    let image = null;

    if (originalImageUrl) {
      image = await downloadMedia(originalImageUrl);
    } else {
      console.warn("[sunwet] no original image URL found");
    }

    return {
      pageUrl,
      sourceUrl,
      artistName: artist.name,
      artistUrl: artist.url,
      image,
    };
  };

  /**
   * Build form commit from booru post data
   * @type {(id: string, data: BooruPostData) => import("./content2.js").CaptureCallbackResult}
   */
  const buildPostCommit = (id, data) => {
    /** @type {CaptureImageParams} */
    const parameters = {};
    parameters.url = data.pageUrl;
    if (data.sourceUrl) {
      parameters.source_url = data.sourceUrl;
    }
    if (data.artistName) {
      parameters.artist_name = data.artistName;
    }
    if (data.artistUrl) {
      parameters.artist_url = data.artistUrl;
    }
    /** @type {Array<{data: Uint8Array, mimetype: string, parameter: string}>} */
    const files = [];
    if (data.image && !data.image.error && data.image.data) {
      files.push({ data: data.image.data, mimetype: data.image.mimeType || "application/octet-stream", parameter: "file" });
    }

    return {
      form_id: "capture-image",
      parameters,
      files,
    };
  };

  /**
   * Add capture button to sidebar
   * @type {() => void}
   */
  const addCaptureButton = () => {
    if (!isImagePage()) return;
    if (document.querySelector(`.${BUTTON_MARKER}`)) return;

    const sidebar = getSidebar();
    if (!sidebar) return;

    const id = window.location.href;

    const callback = async (/** @type {string} */ _id) => {
      const postData = await extractPostData();
      return buildPostCommit(_id, postData);
    };

    const button = create_capture_button(id, "image-exists", callback);
    button.classList.add(BUTTON_MARKER);
    button.style.width = "100%";
    button.style.height = "36px";
    button.style.marginBottom = "10px";

    sidebar.insertBefore(button, sidebar.firstChild);
  };

  addCaptureButton();

  const observer = new MutationObserver(() => {
    addCaptureButton();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  console.log("Booru capture extension initialized");
};
