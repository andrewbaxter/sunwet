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
export const do_deviantart = () => {
  const BUTTON_MARKER = "sunwet-deviantart-capture-btn";

  /**
   * Download media and return blob data
   * @type {(url: string) => Promise<MediaResult>}
   */
  const downloadMedia = async (url) => {
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const mimeType = response.headers.get("Content-Type");
      const buffer = await response.arrayBuffer();
      return { url, data: new Uint8Array(buffer), mimeType };
    } catch (err) {
      return { url, error: /** @type {Error} */ (err).message };
    }
  };

  /**
   * Check if this is a deviation (artwork) page
   * @type {() => boolean}
   */
  const isDeviationPage = () => {
    // Deviation pages have URLs like /username/art/title-123456
    return /\/art\//.test(window.location.pathname);
  };

  /**
   * Extract the post URL
   * @type {() => string}
   */
  const getPostUrl = () => {
    const ogUrl = document.querySelector('meta[property="og:url"]');
    if (ogUrl) {
      const content = ogUrl.getAttribute("content");
      if (content) return content;
    }
    const canonical = document.querySelector('link[rel="canonical"]');
    if (canonical) {
      const href = canonical.getAttribute("href");
      if (href) return href;
    }
    return window.location.href;
  };

  /**
   * Extract the author URL from the post URL
   * @type {() => string|null}
   */
  const getAuthorUrl = () => {
    const url = getPostUrl();
    const match = url.match(/(https?:\/\/www\.deviantart\.com\/[\w-]+)/);
    return match ? match[1] : null;
  };

  /**
   * Extract the artwork title from og:title or page title
   * @type {() => string|null}
   */
  const getTitle = () => {
    const ogTitle = document.querySelector('meta[property="og:title"]');
    if (ogTitle) {
      const content = ogTitle.getAttribute("content");
      if (content) {
        // Format is typically "Title by Artist on DeviantArt"
        const match = content.match(/^(.+?)\s+by\s+/);
        return match ? match[1] : content;
      }
    }
    return null;
  };

  /**
   * Get the main artwork image URL.
   * Prefers the download link (full resolution), falls back to og:image.
   * @type {() => string|null}
   */
  const getImageUrl = () => {
    // Try the download link first (full resolution)
    const downloadLink = /** @type {HTMLAnchorElement|null} */ (
      document.querySelector('a[download][aria-label="Free download"]')
    );
    if (downloadLink && downloadLink.href) {
      return downloadLink.href;
    }

    // Fallback to og:image
    const ogImage = document.querySelector('meta[property="og:image"]');
    if (ogImage) {
      const content = ogImage.getAttribute("content");
      if (content) return content;
    }

    return null;
  };

  /**
   * Find the download button element
   * @type {() => Element|null}
   */
  const findDownloadButton = () => {
    return document.querySelector('a[download][aria-label="Free download"]');
  };

  /**
   * Add capture button next to the download button
   * @type {() => void}
   */
  const addCaptureButton = () => {
    if (!isDeviationPage()) return;
    if (document.querySelector(`.${BUTTON_MARKER}`)) return;

    const downloadBtn = findDownloadButton();
    if (!downloadBtn) return;

    // The download button is inside a container div
    const btnContainer = downloadBtn.closest("div");
    if (!btnContainer || !btnContainer.parentElement) return;

    const id = getPostUrl();

    const callback = async (/** @type {string} */ _id) => {
      const authorUrl = getAuthorUrl();
      const title = getTitle();
      const imageUrl = getImageUrl();

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
      if (imageUrl) {
        if (imageUrl.startsWith("data:")) {
          const match = imageUrl.match(/^data:([^;,]+)?(?:;base64)?,(.*)$/);
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
        } else {
          const result = await downloadMedia(imageUrl);
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

    // Insert after the download button's container
    btnContainer.parentElement.insertBefore(button, btnContainer.nextSibling);
  };

  addCaptureButton();

  const observer = new MutationObserver(() => {
    addCaptureButton();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  console.log("[sunwet] DeviantArt capture extension initialized");
};
