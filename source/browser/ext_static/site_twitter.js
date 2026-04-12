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
    // Try to get the highest quality source
    const sources = videoEl.querySelectorAll("source");
    if (sources.length > 0) {
      // Return the first source (usually highest quality)
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

    // Find author URL - look for first user profile link (not the status link)
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
        // This is likely the author profile link
        if (href.match(/\/(x|twitter)\.com\/[^\/]+\/?$/)) {
          data.authorUrl = href;
          break;
        }
      }
    }

    // Find post URL and timestamp from the time element's parent link
    const timeEl = article.querySelector("time[datetime]");
    if (timeEl) {
      const datetime = timeEl.getAttribute("datetime");
      data.timestamp = datetime;
      // Convert to UTC Date object and back to ISO string
      const date = new Date(/** @type {string} */ (datetime));
      data.timestampUtc = date.toISOString();

      // The time element is usually inside the post link
      const postLink = /** @type {HTMLAnchorElement|null} */ (
        timeEl.closest('a[href*="/status/"]')
      );
      if (postLink) {
        data.postUrl = postLink.href;
        // Also extract author from post URL if not found yet
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

    // Find tweet text
    const tweetText = article.querySelector('[data-testid="tweetText"]');
    if (tweetText) {
      data.text = tweetText.textContent;
    }

    // Find images
    const tweetPhotos = article.querySelectorAll('[data-testid="tweetPhoto"]');
    for (const photo of tweetPhotos) {
      // Try to find img element
      const img = /** @type {HTMLImageElement|null} */ (
        photo.querySelector("img[src]")
      );
      if (img && img.src && !img.src.startsWith("data:")) {
        const origUrl = getOriginalImageUrl(img.src);
        const result = await downloadMedia(origUrl);
        data.media.push({ type: "image", ...result });
      } else {
        // Check for background-image style
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

    // Find videos
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
   * Create the capture button SVG icon
   * @type {() => SVGSVGElement}
   */
  const createButtonSvg = () => {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");
    svg.style.width = "1.25em";
    svg.style.height = "1.25em";
    svg.style.fill = "currentColor";
    // Download/save icon
    svg.innerHTML =
      '<g><path d="M12 2.59l5.7 5.7-1.41 1.42L13 6.41V16h-2V6.41l-3.3 3.3-1.41-1.42L12 2.59zM21 15v4c0 1.1-.9 2-2 2H5c-1.1 0-2-.9-2-2v-4h2v4h14v-4h2z"></path></g>';
    return svg;
  };

  /**
   * Add capture button to a tweet
   * @type {(article: HTMLElement) => void}
   */
  const addCaptureButton = (article) => {
    // Check if button already exists
    if (article.querySelector(`.${BUTTON_MARKER}`)) {
      return;
    }

    // Find the bookmark button to insert our button next to it
    const bookmarkBtn = article.querySelector('[data-testid="bookmark"]');
    if (!bookmarkBtn) {
      return;
    }

    // Find the parent container of the bookmark button
    const btnContainer = bookmarkBtn.closest("div");
    if (!btnContainer || !btnContainer.parentElement) {
      return;
    }

    // Create our button wrapper (clone the structure of the bookmark button's container)
    const wrapper = document.createElement("div");
    wrapper.className = BUTTON_MARKER;

    // Create button
    const button = document.createElement("button");
    button.setAttribute("aria-label", "Capture post");
    button.setAttribute("role", "button");
    button.setAttribute("type", "button");
    button.style.cssText = `
            background: none;
            border: none;
            cursor: pointer;
            padding: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: rgb(83, 100, 113);
            transition: color 0.2s;
        `;

    const innerDiv = document.createElement("div");
    innerDiv.style.display = "flex";
    innerDiv.style.alignItems = "center";
    innerDiv.appendChild(createButtonSvg());
    button.appendChild(innerDiv);

    button.addEventListener("mouseenter", () => {
      button.style.color = "rgb(29, 155, 240)";
    });
    button.addEventListener("mouseleave", () => {
      button.style.color = "rgb(83, 100, 113)";
    });

    // Click handler
    button.addEventListener("click", async (e) => {
      e.preventDefault();
      e.stopPropagation();

      // Visual feedback
      button.style.color = "rgb(29, 155, 240)";
      const originalSvg = innerDiv.innerHTML;
      innerDiv.innerHTML = '<span style="font-size: 12px;">...</span>';

      try {
        const postData = await extractPostData(article);
        console.log("=== Captured Post Data ===");
        console.log("Author URL:", postData.authorUrl);
        console.log("Post URL:", postData.postUrl);
        console.log("Timestamp (original):", postData.timestamp);
        console.log("Timestamp (UTC):", postData.timestampUtc);
        console.log("Text:", postData.text);
        console.log("Media count:", postData.media.length);
        for (const m of postData.media) {
          if (m.error) {
            console.log(`  ${m.type}: ${m.url} - ERROR: ${m.error}`);
          } else {
            console.log(`  ${m.type}: ${m.url}`);
            console.log(`    MIME: ${m.mimeType}`);
            console.log(`    SHA256: ${m.digest}`);
            console.log(`    Size: ${m.size} bytes`);
          }
        }
        console.log("=========================");

        // Success feedback
        innerDiv.innerHTML = '<span style="font-size: 12px;">✓</span>';
        setTimeout(() => {
          innerDiv.innerHTML = originalSvg;
          button.style.color = "rgb(83, 100, 113)";
        }, 1500);
      } catch (err) {
        console.error("Error capturing post:", err);
        innerDiv.innerHTML = '<span style="font-size: 12px;">✗</span>';
        setTimeout(() => {
          innerDiv.innerHTML = originalSvg;
          button.style.color = "rgb(83, 100, 113)";
        }, 1500);
      }
    });

    wrapper.appendChild(button);

    // Insert after the bookmark button's container
    btnContainer.parentElement.insertBefore(wrapper, btnContainer.nextSibling);
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

  // Initial processing
  processAllTweets();

  // Set up MutationObserver to detect new tweets
  /** @type {MutationCallback} */
  const tweetObserverCallback = (mutations) => {
    let shouldProcess = false;
    for (const mutation of mutations) {
      if (mutation.addedNodes.length > 0) {
        for (const node of mutation.addedNodes) {
          if (node.nodeType === Node.ELEMENT_NODE) {
            const el = /** @type {Element} */ (node);
            // Check if the added node is or contains a tweet
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

  // Start observing
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

    // Get user URL from current page or profile link
    const path = window.location.pathname;
    const userMatch = path.match(/^\/([^\/]+)\/?$/);
    if (userMatch) {
      data.userHandle = "@" + userMatch[1];
      data.userUrl = window.location.origin + "/" + userMatch[1];
    }

    // Get user name from UserName element
    const userNameEl = document.querySelector('[data-testid="UserName"]');
    if (userNameEl) {
      // The first span with the actual name text
      const nameSpan = userNameEl.querySelector("span span");
      if (nameSpan) {
        data.userName = nameSpan.textContent;
      }
      // Also try to get handle if not found from URL
      if (!data.userHandle) {
        const handleEl = userNameEl.querySelector('div[dir="ltr"] span');
        if (handleEl && handleEl.textContent?.startsWith("@")) {
          data.userHandle = handleEl.textContent;
        }
      }
    }

    // Get profile description with markdown links
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

    // Get profile image URL
    // First try to find the photo link
    const photoLink = document.querySelector('a[href$="/photo"]');
    if (photoLink) {
      // The profile image is typically in a div with background-image inside
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
      // Also check for img element
      if (!data.profileImageUrl) {
        const img = /** @type {HTMLImageElement|null} */ (
          photoLink.querySelector('img[src^="http"]')
        );
        if (img) {
          data.profileImageUrl = getOriginalImageUrl(img.src);
        }
      }
    }

    // Get banner/header image URL
    const headerLink = document.querySelector('a[href$="/header_photo"]');
    if (headerLink) {
      // Look for image inside
      const img = /** @type {HTMLImageElement|null} */ (
        headerLink.querySelector('img[src^="http"]')
      );
      if (img) {
        data.bannerImageUrl = getOriginalImageUrl(img.src);
      } else {
        // Check for background-image
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

    // Download media if URLs found
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
   * Add capture button to profile
   * @type {() => void}
   */
  const addProfileCaptureButton = () => {
    // Check if already added
    if (document.querySelector(`.${PROFILE_BUTTON_MARKER}`)) {
      return;
    }

    // Find the edit profile button or follow button
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

    // Get the parent container
    const btnContainer = profileBtn.parentElement;
    if (!btnContainer) {
      return;
    }

    // Create our button
    const button = document.createElement("button");
    button.className = PROFILE_BUTTON_MARKER;
    button.setAttribute("aria-label", "Capture profile");
    button.setAttribute("role", "button");
    button.setAttribute("type", "button");
    button.style.cssText = `
            background: none;
            border: 1px solid rgb(207, 217, 222);
            border-radius: 9999px;
            cursor: pointer;
            padding: 8px 16px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: rgb(15, 20, 25);
            font-weight: bold;
            font-size: 14px;
            margin-right: 8px;
            transition: background-color 0.2s;
        `;

    const innerSpan = document.createElement("span");
    innerSpan.textContent = "Capture";
    button.appendChild(innerSpan);

    button.addEventListener("mouseenter", () => {
      button.style.backgroundColor = "rgba(15, 20, 25, 0.1)";
    });
    button.addEventListener("mouseleave", () => {
      button.style.backgroundColor = "transparent";
    });

    // Click handler
    button.addEventListener("click", async (e) => {
      e.preventDefault();
      e.stopPropagation();

      innerSpan.textContent = "...";
      button.disabled = true;

      try {
        const profileData = await extractProfileData();
        console.log("=== Captured Profile Data ===");
        console.log("User URL:", profileData.userUrl);
        console.log("User Name:", profileData.userName);
        console.log("User Handle:", profileData.userHandle);
        console.log("Profile Text:", profileData.profileText);
        console.log("Profile Image URL:", profileData.profileImageUrl);
        console.log("Banner Image URL:", profileData.bannerImageUrl);
        console.log("Media count:", profileData.media.length);
        for (const m of profileData.media) {
          if (m.error) {
            console.log(`  ${m.type}: ${m.url} - ERROR: ${m.error}`);
          } else {
            console.log(`  ${m.type}: ${m.url}`);
            console.log(`    MIME: ${m.mimeType}`);
            console.log(`    SHA256: ${m.digest}`);
            console.log(`    Size: ${m.size} bytes`);
          }
        }
        console.log("=============================");

        innerSpan.textContent = "✓";
        setTimeout(() => {
          innerSpan.textContent = "Capture";
          button.disabled = false;
        }, 1500);
      } catch (err) {
        console.error("Error capturing profile:", err);
        innerSpan.textContent = "✗";
        setTimeout(() => {
          innerSpan.textContent = "Capture";
          button.disabled = false;
        }, 1500);
      }
    });

    // Insert before the profile button
    btnContainer.insertBefore(button, profileBtn);
  };

  // Also observe for profile button
  /** @type {MutationCallback} */
  const profileObserverCallback = () => {
    addProfileCaptureButton();
  };

  const profileObserver = new MutationObserver(profileObserverCallback);

  profileObserver.observe(document.body, {
    childList: true,
    subtree: true,
  });

  // Initial check for profile
  addProfileCaptureButton();
};
