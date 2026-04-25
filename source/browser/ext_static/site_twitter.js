/**
 * @typedef {Object} MediaResult
 * @property {string} url
 * @property {string} [digest]
 * @property {number} [size]
 * @property {string|null} [mimeType]
 * @property {string} [error]
 */

/**
 * @typedef {Object} PostData
 * @property {string|null} authorUrl
 * @property {string|null} postUrl
 * @property {string|null} timestamp
 * @property {string|null} timestampUtc
 * @property {string|null} text
 * @property {Array<{type: string} & MediaResult>} media
 */

/**
 * @typedef {Object} ProfileData
 * @property {string|null} userUrl
 * @property {string|null} userName
 * @property {string|null} userHandle
 * @property {string|null} profileText
 * @property {string|null} profileImageUrl
 * @property {string|null} bannerImageUrl
 * @property {Array<{type: string} & MediaResult>} media
 */

import { create_capture_button } from "./content2.js";

/** @type {() => void} */
export const do_twitter = () => {
  const BUTTON_MARKER = "sunwet-capture-btn";

  /**
   * Compute SHA256 hash of an ArrayBuffer
   * @type {(buffer: ArrayBuffer) => Promise<string>}
   */
  const sha256 = async (buffer) => {
    const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  };

  /**
   * Convert Twitter image URL to original quality
   * @type {(url: string) => string}
   */
  const getOriginalImageUrl = (url) => {
    try {
      const u = new URL(url);
      if (u.hostname === "pbs.twimg.com") {
        u.searchParams.set("name", "orig");
      }
      return u.toString();
    } catch {
      return url;
    }
  };

  /**
   * Get video URL from video element
   * @type {(videoEl: HTMLVideoElement) => string}
   */
  const getVideoUrl = (videoEl) => {
    const sources = videoEl.querySelectorAll("source");
    if (sources.length > 0) {
      return sources[0].src;
    }
    return videoEl.src;
  };

  /**
   * Download media and return sha256 digest
   * @type {(url: string) => Promise<MediaResult>}
   */
  const downloadMedia = async (url) => {
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const mimeType = response.headers.get("Content-Type");
      const buffer = await response.arrayBuffer();
      const digest = await sha256(buffer);
      return { url, digest, size: buffer.byteLength, mimeType };
    } catch (err) {
      return { url, error: /** @type {Error} */ (err).message };
    }
  };

  /**
   * Extract post data from an article element
   * @type {(article: HTMLElement) => Promise<PostData>}
   */
  const extractPostData = async (article) => {
    /** @type {PostData} */
    const data = {
      authorUrl: null,
      postUrl: null,
      timestamp: null,
      timestampUtc: null,
      text: null,
      media: [],
    };

    const links = article.querySelectorAll(
      'a[href*="x.com/"], a[href*="twitter.com/"]',
    );
    for (const link of links) {
      const href = /** @type {HTMLAnchorElement} */ (link).href;
      if (
        href &&
        !href.includes("/status/") &&
        !href.includes("/analytics") &&
        !href.includes("/photo/")
      ) {
        if (href.match(/\/(x|twitter)\.com\/[^\/]+\/?$/)) {
          data.authorUrl = href;
          break;
        }
      }
    }

    const timeEl = article.querySelector("time[datetime]");
    if (timeEl) {
      const datetime = timeEl.getAttribute("datetime");
      data.timestamp = datetime;
      const date = new Date(/** @type {string} */ (datetime));
      data.timestampUtc = date.toISOString();

      const postLink = /** @type {HTMLAnchorElement|null} */ (
        timeEl.closest('a[href*="/status/"]')
      );
      if (postLink) {
        data.postUrl = postLink.href;
        if (!data.authorUrl) {
          const match = postLink.href.match(
            /(https?:\/\/(?:x|twitter)\.com\/[^\/]+)/,
          );
          if (match) {
            data.authorUrl = match[1];
          }
        }
      }
    }

    const tweetText = article.querySelector('[data-testid="tweetText"]');
    if (tweetText) {
      data.text = tweetText.textContent;
    }

    const tweetPhotos = article.querySelectorAll('[data-testid="tweetPhoto"]');
    for (const photo of tweetPhotos) {
      const img = /** @type {HTMLImageElement|null} */ (
        photo.querySelector("img[src]")
      );
      if (img && img.src && !img.src.startsWith("data:")) {
        const origUrl = getOriginalImageUrl(img.src);
        const result = await downloadMedia(origUrl);
        data.media.push({ type: "image", ...result });
      } else {
        const photoEl = /** @type {HTMLElement} */ (photo);
        const style =
          photoEl.style.backgroundImage ||
          window.getComputedStyle(photoEl).backgroundImage;
        const match = style.match(/url\(['"]?(https?:\/\/[^'")\s]+)['"]?\)/);
        if (match) {
          const origUrl = getOriginalImageUrl(match[1]);
          const result = await downloadMedia(origUrl);
          data.media.push({ type: "image", ...result });
        }
      }
    }

    const videos = article.querySelectorAll("video");
    for (const video of videos) {
      const videoUrl = getVideoUrl(video);
      if (videoUrl && !videoUrl.startsWith("data:")) {
        const result = await downloadMedia(videoUrl);
        data.media.push({ type: "video", ...result });
      }
    }

    return data;
  };

  /**
   * Build form commit from post data
   * @type {(id: string, data: PostData) => {form_id: string, parameters: Record<string, string>}}
   */
  const buildPostCommit = (id, data) => {
    /** @type {Record<string, string>} */
    const parameters = {};
    if (data.postUrl) {
      parameters.url = data.postUrl;
    }
    if (data.authorUrl) {
      parameters.author = data.authorUrl;
    }
    if (data.text) {
      parameters.text = data.text;
    }
    if (data.timestampUtc) {
      parameters.date = data.timestampUtc;
    }
    if (data.media.length > 0) {
      const hashes = [];
      for (const m of data.media) {
        if (!m.error && m.digest) {
          hashes.push(`sha256:${m.digest}`);
        }
      }
      if (hashes.length > 0) {
        parameters.media = hashes.join(",");
      }
    }

    return {
      form_id: "capture-microblog",
      parameters,
    };
  };

  /**
   * Add capture button to a tweet
   * @type {(article: HTMLElement) => void}
   */
  const addCaptureButton = (article) => {
    if (article.querySelector(`.${BUTTON_MARKER}`)) {
      return;
    }

    const bookmarkBtn = article.querySelector('[data-testid="bookmark"]');
    if (!bookmarkBtn) {
      return;
    }

    const btnContainer = bookmarkBtn.closest("div");
    if (!btnContainer || !btnContainer.parentElement) {
      return;
    }

    const timeEl = article.querySelector("time[datetime]");
    const postLink = timeEl?.closest('a[href*="/status/"]');
    const id = postLink?.href || article.getAttribute("aria-labelledby") || Math.random().toString(36);

    const callback = async (/** @type {string} */ _id) => {
      const postData = await extractPostData(article);
      return buildPostCommit(_id, postData);
    };

    const button = create_capture_button(id, "microblog-exists", callback);
    button.classList.add(BUTTON_MARKER);

    btnContainer.parentElement.insertBefore(button, btnContainer.nextSibling);
  };

  /**
   * Process all tweets on the page
   * @type {() => void}
   */
  const processAllTweets = () => {
    const tweets = document.querySelectorAll('article[data-testid="tweet"]');
    for (const tweet of tweets) {
      addCaptureButton(/** @type {HTMLElement} */ (tweet));
    }
  };

  processAllTweets();

  /** @type {MutationCallback} */
  const tweetObserverCallback = (mutations) => {
    let shouldProcess = false;
    for (const mutation of mutations) {
      if (mutation.addedNodes.length > 0) {
        for (const node of mutation.addedNodes) {
          if (node.nodeType === Node.ELEMENT_NODE) {
            const el = /** @type {Element} */ (node);
            if (
              el.matches?.('article[data-testid="tweet"]') ||
              el.querySelector?.('article[data-testid="tweet"]')
            ) {
              shouldProcess = true;
              break;
            }
          }
        }
      }
      if (shouldProcess) break;
    }
    if (shouldProcess) {
      processAllTweets();
    }
  };

  const observer = new MutationObserver(tweetObserverCallback);
  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  console.log("Twitter capture extension initialized");

  // ===== Profile Capture =====
  const PROFILE_BUTTON_MARKER = "sunwet-profile-capture-btn";

  /**
   * Extract profile data from the page
   * @type {() => Promise<ProfileData>}
   */
  const extractProfileData = async () => {
    /** @type {ProfileData} */
    const data = {
      userUrl: null,
      userName: null,
      userHandle: null,
      profileText: null,
      profileImageUrl: null,
      bannerImageUrl: null,
      media: [],
    };

    const path = window.location.pathname;
    const userMatch = path.match(/^\/([^\/]+)\/?$/);
    if (userMatch) {
      data.userHandle = "@" + userMatch[1];
      data.userUrl = window.location.origin + "/" + userMatch[1];
    }

    const userNameEl = document.querySelector('[data-testid="UserName"]');
    if (userNameEl) {
      const nameSpan = userNameEl.querySelector("span span");
      if (nameSpan) {
        data.userName = nameSpan.textContent;
      }
      if (!data.userHandle) {
        const handleEl = userNameEl.querySelector('div[dir="ltr"] span');
        if (handleEl && handleEl.textContent?.startsWith("@")) {
          data.userHandle = handleEl.textContent;
        }
      }
    }

    const descEl = document.querySelector('[data-testid="UserDescription"]');
    if (descEl) {
      let text = "";
      /**
       * Process a DOM node recursively to extract text with markdown links
       * @type {(node: Node) => void}
       */
      const processNode = (node) => {
        if (node.nodeType === Node.TEXT_NODE) {
          text += node.textContent;
        } else if (node.nodeType === Node.ELEMENT_NODE) {
          const el = /** @type {HTMLElement} */ (node);
          if (el.tagName === "A") {
            const href = /** @type {HTMLAnchorElement} */ (el).href;
            const linkText = el.textContent;
            text += "[" + linkText + "](" + href + ")";
          } else {
            for (const child of node.childNodes) {
              processNode(child);
            }
          }
        }
      };
      processNode(descEl);
      data.profileText = text.trim();
    }

    const photoLink = document.querySelector('a[href$="/photo"]');
    if (photoLink) {
      const avatarContainer = photoLink.closest(
        '[data-testid^="UserAvatar-Container-"]',
      );
      if (avatarContainer) {
        const imgDiv = avatarContainer.querySelector(
          '[aria-label="Opens profile photo"]',
        );
        if (imgDiv) {
          const innerDiv = /** @type {HTMLElement|null} */ (
            imgDiv.querySelector('div[style*="background-image"]')
          );
          if (innerDiv) {
            const style =
              innerDiv.style.backgroundImage ||
              window.getComputedStyle(innerDiv).backgroundImage;
            const match = style.match(
              /url\(['"]?(https?:\/\/[^'")\s]+)['"]?\)/,
            );
            if (match) {
              data.profileImageUrl = getOriginalImageUrl(match[1]);
            }
          }
        }
      }
      if (!data.profileImageUrl) {
        const img = /** @type {HTMLImageElement|null} */ (
          photoLink.querySelector('img[src^="http"]')
        );
        if (img) {
          data.profileImageUrl = getOriginalImageUrl(img.src);
        }
      }
    }

    const headerLink = document.querySelector('a[href$="/header_photo"]');
    if (headerLink) {
      const img = /** @type {HTMLImageElement|null} */ (
        headerLink.querySelector('img[src^="http"]')
      );
      if (img) {
        data.bannerImageUrl = getOriginalImageUrl(img.src);
      } else {
        const bgDiv = /** @type {HTMLElement|null} */ (
          headerLink.querySelector('div[style*="background-image"]')
        );
        if (bgDiv) {
          const style =
            bgDiv.style.backgroundImage ||
            window.getComputedStyle(bgDiv).backgroundImage;
          const match = style.match(/url\(['"]?(https?:\/\/[^'")\s]+)['"]?\)/);
          if (match) {
            data.bannerImageUrl = getOriginalImageUrl(match[1]);
          }
        }
      }
    }

    if (data.profileImageUrl && !data.profileImageUrl.startsWith("data:")) {
      const result = await downloadMedia(data.profileImageUrl);
      data.media.push({ type: "profile_image", ...result });
    }
    if (data.bannerImageUrl && !data.bannerImageUrl.startsWith("data:")) {
      const result = await downloadMedia(data.bannerImageUrl);
      data.media.push({ type: "banner_image", ...result });
    }

    return data;
  };

  /**
   * Build form commit from profile data
   * @type {(id: string, data: ProfileData) => {form_id: string, parameters: Record<string, string>}}
   */
  const buildProfileCommit = (id, data) => {
    /** @type {Record<string, string>} */
    const parameters = {};
    if (data.userUrl) {
      parameters.url = data.userUrl;
    }
    if (data.userName) {
      parameters.name = data.userName;
    }
    if (data.userHandle) {
      parameters.handle = data.userHandle;
    }
    if (data.profileText) {
      parameters.description = data.profileText;
    }
    if (data.media.length > 0) {
      const hashes = [];
      for (const m of data.media) {
        if (!m.error && m.digest) {
          hashes.push(`sha256:${m.digest}`);
        }
      }
      if (hashes.length > 0) {
        parameters.images = hashes.join(",");
      }
    }

    return {
      form_id: "capture-profile",
      parameters,
    };
  };

  /**
   * Add capture button to profile
   * @type {() => void}
   */
  const addProfileCaptureButton = () => {
    if (document.querySelector(`.${PROFILE_BUTTON_MARKER}`)) {
      return;
    }

    /** @type {Element|null} */
    let profileBtn = document.querySelector(
      '[data-testid="placementTracking"]',
    );
    if (!profileBtn) {
      profileBtn = document.querySelector('[data-testid="editProfileButton"]');
    }
    if (!profileBtn) {
      return;
    }

    const btnContainer = profileBtn.parentElement;
    if (!btnContainer) {
      return;
    }

    const path = window.location.pathname;
    const userMatch = path.match(/^\/([^\/]+)\/?$/);
    const id = userMatch ? window.location.origin + "/" + userMatch[1] : window.location.href;

    const callback = async (/** @type {string} */ _id) => {
      const profileData = await extractProfileData();
      return buildProfileCommit(_id, profileData);
    };

    const button = create_capture_button(id, "profile-exists", callback);
    button.classList.add(PROFILE_BUTTON_MARKER);
    button.style.marginRight = "8px";
    button.style.width = "28px";
    button.style.height = "28px";

    btnContainer.insertBefore(button, profileBtn);
  };

  const profileObserverCallback = () => {
    addProfileCaptureButton();
  };

  const profileObserver = new MutationObserver(profileObserverCallback);
  profileObserver.observe(document.body, {
    childList: true,
    subtree: true,
  });

  addProfileCaptureButton();
};
