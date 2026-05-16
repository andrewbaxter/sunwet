/**
 * @typedef {Object} MediaResult
 * @property {string} url
 * @property {Uint8Array} [data]
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
/// <reference path="extension_config.d.ts" />

/** @type {() => void} */
export const do_twitter = () => {
  const BUTTON_MARKER = "sunwet-capture-btn";

  // Video URLs are intercepted by site_twitter_early.js (runs in MAIN world)
  // and stored as data attributes on a hidden DOM element.

  /**
   * Look up intercepted video URL for a tweet ID
   * @type {(tweetId: string) => string|null}
   */
  const getInterceptedVideoUrl = (tweetId) => {
    const store = document.getElementById('sunwet-video-store');
    if (!store) {
      console.warn("[sunwet] video store element not found in DOM");
      return null;
    }
    return store.getAttribute('data-tweet-' + tweetId);
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
   * Find the GraphQL query ID for TweetResultByRestId by scanning Twitter's
   * loaded script bundles. Twitter embeds query IDs in their JS like:
   * {queryId:"abc123",operationName:"TweetResultByRestId",...}
   * @type {() => Promise<{queryId: string, endpoint: string, features: string|null, fieldToggles: string|null}|null>}
   */
  const findQueryIdFromScripts = async () => {
    // First check if the intercept already captured one
    const store = document.getElementById('sunwet-video-store');
    if (store) {
      const id = store.getAttribute('data-gql-id');
      const ep = store.getAttribute('data-gql-endpoint');
      if (id && ep) {
        return {
          queryId: id,
          endpoint: ep,
          features: store.getAttribute('data-gql-features'),
          fieldToggles: store.getAttribute('data-gql-field-toggles'),
        };
      }
    }

    // Scan Twitter's loaded script bundles for query ID
    const scripts = Array.from(document.querySelectorAll('script[src*="abs.twimg.com"]'));
    for (const script of scripts) {
      const src = /** @type {HTMLScriptElement} */ (script).src;
      if (!src) continue;
      try {
        const resp = await fetch(src);
        if (!resp.ok) continue;
        const text = await resp.text();
        // Look for TweetResultByRestId query ID pattern
        const match = text.match(/queryId:"([^"]+)",operationName:"TweetResultByRestId"/);
        if (match) {
          console.log("[sunwet] found TweetResultByRestId queryId:", match[1]);
          return { queryId: match[1], endpoint: "TweetResultByRestId", features: null, fieldToggles: null };
        }
      } catch {
        continue;
      }
    }
    console.warn("[sunwet] could not find TweetResultByRestId query ID in any script");
    return null;
  };

  /**
   * Fetch video URL from Twitter's GraphQL API.
   * Uses captured query params from intercepted requests, or scans bundles for the query ID.
   * @type {(tweetId: string) => Promise<string|null>}
   */
  const fetchVideoUrlFromApi = async (tweetId) => {
    try {
      const queryInfo = await findQueryIdFromScripts();
      if (!queryInfo) {
        return null;
      }
      const csrfToken = document.cookie.match(/ct0=([^;]+)/)?.[1];
      if (!csrfToken) {
        console.warn("[sunwet] no csrf token found in cookies");
        return null;
      }

      /** @type {Record<string, string>} */
      const params = {};
      if (queryInfo.endpoint === 'TweetResultByRestId') {
        params.variables = JSON.stringify({ tweetId, withCommunity: false, includePromotedContent: false, withVoice: false });
      } else {
        // TweetDetail
        params.variables = JSON.stringify({
          focalTweetId: tweetId,
          with_rux_injections: false,
          rankingMode: "Relevance",
          includePromotedContent: false,
          withCommunity: true,
          withQuickPromoteEligibilityTweetFields: false,
          withBirdwatchNotes: true,
          withVoice: true,
        });
      }
      if (queryInfo.features) {
        params.features = queryInfo.features;
      }
      if (queryInfo.fieldToggles) {
        params.fieldToggles = queryInfo.fieldToggles;
      }

      const searchParams = new URLSearchParams(params);
      const resp = await fetch(`https://x.com/i/api/graphql/${queryInfo.queryId}/${queryInfo.endpoint}?${searchParams}`, {
        headers: {
          "authorization": "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA",
          "x-csrf-token": csrfToken,
          "x-twitter-active-user": "yes",
          "x-twitter-auth-type": "OAuth2Session",
        },
        credentials: "include",
      });
      if (!resp.ok) {
        console.warn("[sunwet] GraphQL API returned", resp.status, await resp.text().catch(() => ""));
        return null;
      }
      const json = await resp.json();
      /** @type {string|null} */
      let bestUrl = null;
      /** @param {any} obj @param {number} depth */
      const findVideo = (obj, depth) => {
        if (!obj || typeof obj !== 'object' || depth > 20 || bestUrl) return;
        if (obj.video_info && obj.video_info.variants) {
          const mp4s = obj.video_info.variants
            .filter(/** @param {any} v */ (v) => v.content_type === 'video/mp4' && v.bitrate)
            .sort(/** @param {any} a @param {any} b */ (a, b) => b.bitrate - a.bitrate);
          if (mp4s.length > 0) {
            bestUrl = mp4s[0].url;
            return;
          }
        }
        if (Array.isArray(obj)) {
          for (const item of obj) findVideo(item, depth + 1);
        } else {
          for (const val of Object.values(obj)) findVideo(val, depth + 1);
        }
      };
      findVideo(json, 0);
      if (bestUrl) {
        const store = document.getElementById('sunwet-video-store');
        if (store) {
          store.setAttribute('data-tweet-' + tweetId, bestUrl);
        }
      }
      return bestUrl;
    } catch (err) {
      console.warn("[sunwet] fetchVideoUrlFromApi error:", err);
      return null;
    }
  };

  /**
   * Get video URL from video element, using intercepted API data or direct API call
   * @type {(videoEl: HTMLVideoElement, article: HTMLElement) => Promise<string>}
   */
  const getVideoUrl = async (videoEl, article) => {
    // Try to find the tweet ID from the article and look up intercepted video URL
    const timeEl = article.querySelector("time[datetime]");
    const postLink = /** @type {HTMLAnchorElement|null} */ (
      timeEl?.closest('a[href*="/status/"]')
    );
    if (postLink) {
      const match = postLink.href.match(/\/status\/(\d+)/);
      if (match) {
        const tweetId = match[1];
        const intercepted = getInterceptedVideoUrl(tweetId);
        if (intercepted) {
          return intercepted;
        }
        // Fallback: fetch directly from API
        console.log("[sunwet] no intercepted URL for tweet", tweetId, "- fetching from API");
        const fetched = await fetchVideoUrlFromApi(tweetId);
        if (fetched) {
          return fetched;
        }
      }
    }
    // Fallback: try source elements and video src
    const sources = videoEl.querySelectorAll("source");
    for (const source of sources) {
      if (source.src && !source.src.startsWith("blob:") && !source.src.startsWith("data:")) {
        return source.src;
      }
    }
    if (videoEl.src && !videoEl.src.startsWith("blob:") && !videoEl.src.startsWith("data:")) {
      return videoEl.src;
    }
    return "";
  };

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
      const videoUrl = await getVideoUrl(video, article);
      if (videoUrl && !videoUrl.startsWith("data:") && !videoUrl.startsWith("blob:")) {
        const result = await downloadMedia(videoUrl);
        if (result.error) {
          throw new Error(`Failed to download video: ${result.error} (${videoUrl})`);
        }
        data.media.push({ type: "video", ...result });
      } else if (video.querySelector("source") || video.src) {
        // A video element exists but we couldn't get a fetchable URL
        throw new Error("Video found but no downloadable URL available");
      }
    }

    return data;
  };

  /**
   * Build form commit from post data
   * @type {(id: string, data: PostData) => import("./content2.js").CaptureCallbackResult}
   */
  const buildPostCommit = (id, data) => {
    /** @type {CaptureMicroblogParams} */
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
      parameters.create_timestamp = data.timestampUtc;
    }
    /** @type {Array<{data: Uint8Array, mimetype: string, parameter: string}>} */
    const files = [];
    for (const m of data.media) {
      if (!m.error && m.data) {
        files.push({ data: m.data, mimetype: m.mimeType || "application/octet-stream", parameter: "media" });
      }
    }

    return {
      form_id: "capture-microblog",
      parameters,
      files,
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
    const postLink = /** @type {HTMLAnchorElement|null} */ (timeEl?.closest('a[href*="/status/"]'));
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
   * @type {(id: string, data: ProfileData) => import("./content2.js").CaptureCallbackResult}
   */
  const buildProfileCommit = (id, data) => {
    /** @type {CaptureProfileParams} */
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
    /** @type {Array<{data: Uint8Array, mimetype: string, parameter: string}>} */
    const files = [];
    for (const m of data.media) {
      if (!m.error && m.data) {
        files.push({ data: m.data, mimetype: m.mimeType || "application/octet-stream", parameter: "images" });
      }
    }

    return {
      form_id: "capture-profile",
      parameters,
      files,
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
