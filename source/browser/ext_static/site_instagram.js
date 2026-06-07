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
export const do_instagram = () => {
  const BUTTON_MARKER = "sunwet-instagram-capture-btn";

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
   * Find the article element for a post (modal or standalone)
   * @type {() => HTMLElement|null}
   */
  const findPostArticle = () => {
    return /** @type {HTMLElement|null} */ (document.querySelector("article[role='presentation']"));
  };

  /**
   * Extract post images from an article element.
   * Handles both single-image and carousel posts.
   * @type {(article: HTMLElement) => HTMLImageElement[]}
   */
  const extractPostImages = (article) => {
    // Post images have crossorigin=anonymous and are inside the media area (not profile pics)
    // Profile pics have "profile picture" in alt text
    const allImgs = article.querySelectorAll("img[crossorigin='anonymous']");
    /** @type {HTMLImageElement[]} */
    const postImgs = [];
    for (const img of allImgs) {
      const alt = (/** @type {HTMLImageElement} */ (img)).alt || "";
      if (!alt.includes("profile picture")) {
        postImgs.push(/** @type {HTMLImageElement} */ (img));
      }
    }
    return postImgs;
  };

  /**
   * Extract username from the article
   * @type {(article: HTMLElement) => string|null}
   */
  const extractUsername = (article) => {
    // The username link is an anchor with href like /username/ inside the article header
    // It typically appears before the post image
    const links = article.querySelectorAll("a[role='link'][href]");
    for (const link of links) {
      const href = /** @type {HTMLAnchorElement} */ (link).getAttribute("href") || "";
      // Match /username/ but not /p/, /reel/, /explore/, etc.
      const match = href.match(/^(?:https:\/\/www\.instagram\.com)?\/([\w.]+)\/?$/);
      if (match && !["p", "reel", "reels", "explore", "stories", "accounts", "direct"].includes(match[1])) {
        return match[1];
      }
    }
    return null;
  };

  /**
   * Extract caption text from the article
   * @type {(article: HTMLElement) => string|null}
   */
  const extractCaption = (article) => {
    // Caption is in an h1 element within the comments section
    const h1 = article.querySelector("h1");
    if (h1 && h1.textContent) {
      return h1.textContent.trim();
    }
    return null;
  };

  /**
   * Extract post timestamp
   * @type {(article: HTMLElement) => string|null}
   */
  const extractTimestamp = (article) => {
    const time = article.querySelector("time[datetime]");
    if (time) {
      return time.getAttribute("datetime");
    }
    return null;
  };

  /**
   * Get the post URL from the article
   * @type {(article: HTMLElement) => string|null}
   */
  const extractPostUrl = (article) => {
    // Look for a link to /p/POST_ID/ or /reel/POST_ID/
    const links = article.querySelectorAll("a[href]");
    for (const link of links) {
      const href = /** @type {HTMLAnchorElement} */ (link).href;
      if (href && (href.includes("/p/") || href.includes("/reel/"))) {
        return href;
      }
    }
    // Fallback: check URL bar
    const path = window.location.pathname;
    if (path.includes("/p/") || path.includes("/reel/")) {
      return window.location.href;
    }
    return null;
  };

  /**
   * Build form commit from Instagram post data
   * @type {(id: string, article: HTMLElement) => Promise<import("./content2.js").CaptureCallbackResult>}
   */
  const buildPostCommit = async (id, article) => {
    const username = extractUsername(article);
    const caption = extractCaption(article);
    const timestamp = extractTimestamp(article);
    const postUrl = extractPostUrl(article);
    const images = extractPostImages(article);

    /** @type {CaptureMicroblogParams} */
    const parameters = {};
    if (postUrl) {
      parameters.url = postUrl;
    }
    if (username) {
      parameters.author = `https://www.instagram.com/${username}/`;
    }
    if (caption) {
      parameters.text = caption;
    }
    if (timestamp) {
      parameters.create_timestamp = new Date(timestamp).toISOString();
    }

    /** @type {Array<{data: Uint8Array, mimetype: string, parameter: string}>} */
    const files = [];
    for (const img of images) {
      if (img.src && !img.src.startsWith("data:") && !img.src.startsWith("blob:")) {
        const result = await downloadMedia(img.src);
        if (!result.error && result.data) {
          files.push({ data: result.data, mimetype: result.mimeType || "application/octet-stream", parameter: "file" });
        }
      } else if (img.src && img.src.startsWith("data:")) {
        // SingleFile captured page - extract data URI
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
      }
    }

    return {
      form_id: "capture-microblog",
      parameters,
      files,
    };
  };

  /**
   * Find the like button in the article
   * @type {(article: HTMLElement) => Element|null}
   */
  const findLikeButton = (article) => {
    // The like button contains an SVG with aria-label="Like"
    const likeSvg = article.querySelector('svg[aria-label="Like"]');
    if (likeSvg) {
      // Walk up to find the clickable button container
      const btn = likeSvg.closest("[role='button']");
      if (btn) return btn;
      return likeSvg.parentElement;
    }
    // Fallback: look for unliked heart (aria-label="Unlike" means already liked)
    const unlikeSvg = article.querySelector('svg[aria-label="Unlike"]');
    if (unlikeSvg) {
      const btn = unlikeSvg.closest("[role='button']");
      if (btn) return btn;
      return unlikeSvg.parentElement;
    }
    return null;
  };

  /**
   * Add capture button to the post
   * @type {() => void}
   */
  const addCaptureButton = () => {
    const article = findPostArticle();
    if (!article) return;
    if (article.querySelector(`.${BUTTON_MARKER}`)) return;

    const likeBtn = findLikeButton(article);
    if (!likeBtn) return;

    // The like button is inside a span; find the parent container holding all action buttons
    const actionContainer = likeBtn.closest("span");
    if (!actionContainer || !actionContainer.parentElement) return;

    const postUrl = extractPostUrl(article);
    const id = postUrl || window.location.href;

    const callback = async (/** @type {string} */ _id) => {
      return buildPostCommit(_id, article);
    };

    const button = create_capture_button(id, "microblog-exists", callback);
    button.classList.add(BUTTON_MARKER);
    button.style.marginLeft = "4px";

    // Insert after the like button's span container
    actionContainer.parentElement.insertBefore(button, actionContainer.nextSibling);
  };

  addCaptureButton();

  const observer = new MutationObserver(() => {
    addCaptureButton();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  console.log("[sunwet] Instagram capture extension initialized");
};
