/**
 * @typedef {Object} MediaResult
 * @property {string} url
 * @property {Uint8Array} [data]
 * @property {string|null} [mimeType]
 * @property {string} [error]
 */

import { create_capture_button } from "./content2.js";
/// <reference path="extension_config.d.ts" />

/** @type {() => void} */
export const do_artstation = () => {
  const BUTTON_MARKER = "sunwet-artstation-capture-btn";

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
   * Download media and return blob data
   * @type {(url: string) => Promise<MediaResult>}
   */
  const downloadMedia = async (url) => {
    try {
      // Try direct fetch first
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const mimeType = response.headers.get("Content-Type");
      const buffer = await response.arrayBuffer();
      return { url, data: new Uint8Array(buffer), mimeType };
    } catch {
      // Fallback to background fetch for CORS
      try {
        return await downloadMediaBackground(url);
      } catch (err) {
        return { url, error: /** @type {Error} */ (err).message };
      }
    }
  };

  /**
   * Check if this is an artwork page
   * @type {() => boolean}
   */
  const isArtworkPage = () => {
    return /\/artwork\//.test(window.location.pathname);
  };

  /**
   * Get the post URL
   * @type {() => string}
   */
  const getPostUrl = () => {
    const ogUrl = document.querySelector('meta[property="og:url"]');
    if (ogUrl) {
      const content = ogUrl.getAttribute("content");
      if (content) return content;
    }
    // Try content=... property=og:url ordering
    for (const meta of document.querySelectorAll("meta")) {
      if ((meta.getAttribute("property") || "") === "og:url") {
        const content = meta.getAttribute("content");
        if (content) return content;
      }
    }
    return window.location.href;
  };

  /**
   * Get the project title
   * @type {() => string|null}
   */
  const getTitle = () => {
    // og:title format is "Title, ArtistName"
    for (const meta of document.querySelectorAll("meta")) {
      if ((meta.getAttribute("property") || "") === "og:title") {
        const content = meta.getAttribute("content");
        if (content) {
          // Remove artist name suffix if present
          const parts = content.split(",");
          return parts[0].trim();
        }
      }
    }
    // Fallback: page title "ArtStation - Title"
    const title = document.title;
    const match = title.match(/ArtStation\s*-\s*(.+)/);
    return match ? match[1].trim() : null;
  };

  /**
   * Get the artist profile URL
   * @type {() => string|null}
   */
  const getAuthorUrl = () => {
    // Artist profile link is typically in the project header area
    // Look for a link with class containing user-name or link to artstation.com/username
    const userLink = document.querySelector(".project-author-name a[href]");
    if (userLink) {
      return /** @type {HTMLAnchorElement} */ (userLink).href;
    }
    // Look for profile links in the project header
    const links = document.querySelectorAll("a[href*='artstation.com/']");
    for (const link of links) {
      const href = /** @type {HTMLAnchorElement} */ (link).getAttribute("href") || "";
      // Match artist profile URLs but not artwork/project URLs
      if (href.match(/artstation\.com\/[\w-]+\/?$/) && !href.includes("/artwork/") && !href.includes("/projects/")) {
        return href;
      }
    }
    return null;
  };

  /**
   * Extract all artwork images from the page.
   * ArtStation projects can have multiple images.
   * @type {() => HTMLImageElement[]}
   */
  const extractArtworkImages = () => {
    // Artwork images are in .asset-image containers with class "img img-fluid block-center img-fit"
    const imgs = document.querySelectorAll("img.img-fit");
    if (imgs.length > 0) {
      return /** @type {HTMLImageElement[]} */ (Array.from(imgs));
    }
    // Fallback: images in asset containers
    const assetImgs = document.querySelectorAll(".asset-image img");
    return /** @type {HTMLImageElement[]} */ (Array.from(assetImgs));
  };

  /**
   * Find the like button
   * @type {() => Element|null}
   */
  const findLikeButton = () => {
    // ArtStation like button has class "btn btn-reset reaction like"
    return document.querySelector("button.reaction.like");
  };

  /**
   * Add capture button next to the like button
   * @type {() => void}
   */
  const addCaptureButton = () => {
    if (!isArtworkPage()) return;
    if (document.querySelector(`.${BUTTON_MARKER}`)) return;

    const likeBtn = findLikeButton();
    if (!likeBtn) return;

    // The like button is inside .reaction-wrapper > button
    const reactionWrapper = likeBtn.closest(".reaction-wrapper");
    if (!reactionWrapper || !reactionWrapper.parentElement) return;

    const id = getPostUrl();

    const callback = async (/** @type {string} */ _id) => {
      const authorUrl = getAuthorUrl();
      const title = getTitle();
      const images = extractArtworkImages();

      /** @type {CaptureMicroblogParams} */
      const parameters = {};
      parameters.url = getPostUrl();
      if (authorUrl) {
        parameters.author = authorUrl;
      }
      if (title) {
        parameters.text = title;
      }

      /** @type {Array<{data: Uint8Array, mimetype: string, parameter: string}>} */
      const files = [];
      for (const img of images) {
        if (img.src && img.src.startsWith("data:")) {
          const match = img.src.match(/^data:([^;,]+)?(?:;base64)?,(.*)$/);
          if (match) {
            const mimeType = match[1] || "application/octet-stream";
            const base64 = match[2];
            const binary = atob(base64);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
              bytes[i] = binary.charCodeAt(i);
            }
            files.push({ data: bytes, mimetype: mimeType, parameter: "file" });
          }
        } else if (img.src && !img.src.startsWith("blob:")) {
          const result = await downloadMedia(img.src);
          if (!result.error && result.data) {
            files.push({ data: result.data, mimetype: result.mimeType || "application/octet-stream", parameter: "file" });
          }
        }
      }

      return {
        form_id: "capture-microblog",
        parameters,
        files,
      };
    };

    const button = create_capture_button(id, "microblog-exists", callback);
    button.classList.add(BUTTON_MARKER);
    button.style.marginLeft = "8px";

    // Insert after the reaction wrapper
    reactionWrapper.parentElement.insertBefore(button, reactionWrapper.nextSibling);
  };

  addCaptureButton();

  const observer = new MutationObserver(() => {
    addCaptureButton();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  console.log("[sunwet] ArtStation capture extension initialized");
};
