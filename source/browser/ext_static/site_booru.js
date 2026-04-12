/**
 * @typedef {Object} MediaResult
 * @property {string} url
 * @property {string} [digest]
 * @property {number} [size]
 * @property {string|null} [mimeType]
 * @property {string} [error]
 */

/**
 * @typedef {Object} BooruPostData
 * @property {string} pageUrl
 * @property {string|null} sourceUrl
 * @property {Record<string, string[]>} tags
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
 */

/** @type {SiteConfig[]} */
const siteConfigs = [
  // Gelbooru
  {
    name: "gelbooru",
    match: (hostname) => hostname.includes("gelbooru.com"),
    sidebarSelectors: [".aside", "aside"],
    tagListSelectors: ["ul#tag-list"],
    sourceSelectors: ["[data-source]"],
    originalImageSelectors: [
      'meta[property="og:image"]',
      'meta[name="twitter:image"]',
    ],
    sourceDataAttribute: "data-source",
    sourceTextPattern: null,
  },
  // Safebooru
  {
    name: "safebooru",
    match: (hostname) => hostname.includes("safebooru.org"),
    sidebarSelectors: ["div.sidebar"],
    tagListSelectors: ["ul#tag-sidebar"],
    sourceSelectors: ["#stats"],
    originalImageSelectors: ['a[href*="/images/"]'],
    sourceDataAttribute: null,
    sourceTextPattern: "Source:",
  },
  // Danbooru
  {
    name: "danbooru",
    match: (hostname) => hostname.includes("danbooru.donmai.us"),
    sidebarSelectors: ["aside", "section.sidebar", "#sidebar"],
    tagListSelectors: ["ul.tag-list", "section.tag-list"],
    sourceSelectors: [".image-container", "#post-info"],
    originalImageSelectors: ['a[href*="/original/"]', 'a.image-view-original-link'],
    sourceDataAttribute: "data-source",
    sourceTextPattern: "Source:",
  },
  // Rule34.xxx
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
  },
  // Generic booru fallback
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
   * Compute SHA256 hash of an ArrayBuffer
   * @type {(buffer: ArrayBuffer) => Promise<string>}
   */
  const sha256 = async (buffer) => {
    const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
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
   * Extract tags grouped by type
   * @type {() => Record<string, string[]>}
   */
  const extractTags = () => {
    /** @type {Record<string, string[]>} */
    const tags = {};

    const tagList = getTagList();
    if (!tagList) return tags;

    const tagItems = tagList.querySelectorAll("li[class*='tag-type-']");

    for (const item of tagItems) {
      const classList = item.className;
      const typeMatch = classList.match(/tag-type-(\w+)/);
      if (!typeMatch) continue;

      const tagType = typeMatch[1];

      const links = item.querySelectorAll("a");
      let tagName = null;

      for (const link of links) {
        const href = link.getAttribute("href") || "";
        if (href.includes("page=post") && href.includes("tags=")) {
          tagName = link.textContent?.trim();
          break;
        }
      }

      if (tagName) {
        if (!tags[tagType]) {
          tags[tagType] = [];
        }
        tags[tagType].push(tagName);
      }
    }

    return tags;
  };

  /**
   * Get the source URL for the image using site-specific methods
   * @type {() => string|null}
   */
  const getSourceUrl = () => {
    // Method 1: Check data attribute if configured
    if (siteConfig.sourceDataAttribute) {
      const el = document.querySelector(`[${siteConfig.sourceDataAttribute}]`);
      if (el) {
        const source = el.getAttribute(siteConfig.sourceDataAttribute);
        if (source && source.trim()) return source.trim();
      }
    }

    // Method 2: Look for source in configured selectors
    for (const selector of siteConfig.sourceSelectors) {
      const container = document.querySelector(selector);
      if (!container) continue;

      // If there's a text pattern, search for it
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
              // Check next sibling element for link
              const nextEl = parent.nextElementSibling;
              if (nextEl?.tagName === "A") {
                const href = nextEl.getAttribute("href");
                if (href && !href.startsWith("#")) return href;
              }
            }
          }
        }
      }

      // Also check for any external link in the container
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
          // Skip navigation links
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
    // Method 1: Try site-specific selectors
    for (const selector of siteConfig.originalImageSelectors) {
      const el = document.querySelector(selector);
      if (!el) continue;

      // Handle meta tags
      if (el.tagName === "META") {
        const content = el.getAttribute("content");
        if (content && content.includes("/images/")) {
          return content;
        }
        continue;
      }

      // Handle anchor tags
      if (el.tagName === "A") {
        const href = el.getAttribute("href");
        const text = el.textContent?.toLowerCase().trim() || "";

        // Check if it's the "Original image" link
        if (text === "original image" || text === "original") {
          if (href) {
            if (href.startsWith("//")) return "https:" + href;
            if (href.startsWith("/")) return window.location.origin + href;
            return href;
          }
        }

        // Or if href contains /images/
        if (href?.includes("/images/")) {
          if (href.startsWith("//")) return "https:" + href;
          if (href.startsWith("/")) return window.location.origin + href;
          return href;
        }
      }
    }

    // Method 2: Look for "Original image" link text anywhere
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

    // Method 3: Check og:image meta tag as fallback
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
    const tags = extractTags();
    const originalImageUrl = getOriginalImageUrl();

    /** @type {MediaResult|null} */
    let image = null;

    if (originalImageUrl) {
      image = await downloadMedia(originalImageUrl);
    }

    return {
      pageUrl,
      sourceUrl,
      tags,
      image,
    };
  };

  /**
   * Create the capture button
   * @type {() => HTMLButtonElement}
   */
  const createCaptureButton = () => {
    const button = document.createElement("button");
    button.className = BUTTON_MARKER;
    button.setAttribute("type", "button");
    button.style.cssText = `
      display: block;
      width: 100%;
      padding: 8px 12px;
      margin-bottom: 10px;
      background: #006ffa;
      color: white;
      border: none;
      border-radius: 4px;
      cursor: pointer;
      font-weight: bold;
      font-size: 14px;
      transition: background-color 0.2s;
    `;
    button.textContent = "Capture";

    button.addEventListener("mouseenter", () => {
      button.style.backgroundColor = "#0056cc";
    });
    button.addEventListener("mouseleave", () => {
      button.style.backgroundColor = "#006ffa";
    });

    button.addEventListener("click", async () => {
      const originalText = button.textContent;
      button.textContent = "...";
      button.disabled = true;

      try {
        const postData = await extractPostData();

        console.log("=== Captured Booru Post Data ===");
        console.log("Site:", siteConfig.name);
        console.log("Page URL:", postData.pageUrl);
        console.log("Source URL:", postData.sourceUrl);
        console.log("Tags:");
        for (const [type, tagList] of Object.entries(postData.tags)) {
          console.log(`  ${type}: ${tagList.join(", ")}`);
        }
        if (postData.image) {
          if (postData.image.error) {
            console.log("Image: ERROR -", postData.image.error);
          } else {
            console.log("Image:");
            console.log("  MIME:", postData.image.mimeType);
            console.log("  SHA256:", postData.image.digest);
          }
        } else {
          console.log("Image: Not found");
        }
        console.log("================================");

        button.textContent = "✓";
        setTimeout(() => {
          button.textContent = originalText;
          button.disabled = false;
        }, 1500);
      } catch (err) {
        console.error("Error capturing booru post:", err);
        button.textContent = "✗";
        setTimeout(() => {
          button.textContent = originalText;
          button.disabled = false;
        }, 1500);
      }
    });

    return button;
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

    const button = createCaptureButton();
    sidebar.insertBefore(button, sidebar.firstChild);
  };

  // Initial setup
  addCaptureButton();

  // Observe for dynamic content changes
  const observer = new MutationObserver(() => {
    addCaptureButton();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  console.log("Booru capture extension initialized");
};
