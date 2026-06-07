/** @type {{runtime: {getURL: (path: string) => string}}} */
const browser = /** @type {any} */ (globalThis).browser;

(async () => {
  const src = browser.runtime.getURL("content2.js");
  const { default: init } = await import(src);
  const { do_twitter } = await import(browser.runtime.getURL("site_twitter.js"));
  const { do_booru } = await import(browser.runtime.getURL("site_booru.js"));
  const { do_instagram } = await import(browser.runtime.getURL("site_instagram.js"));
  const { do_deviantart } = await import(browser.runtime.getURL("site_deviantart.js"));
  const { do_artstation } = await import(browser.runtime.getURL("site_artstation.js"));

  await init(browser.runtime.getURL("content2_bg.wasm"));

  const hostname = window.location.hostname;

  /**
   * Run site-specific initialization when DOM is ready
   * @type {(fn: () => void) => void}
   */
  const runWhenReady = (fn) => {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", fn);
    } else {
      fn();
    }
  };

  // Twitter/X
  if (hostname === "twitter.com" || hostname === "x.com") {
    runWhenReady(do_twitter);
  }

  // Booru sites (Gelbooru, Safebooru, Danbooru, etc.)
  if (
    hostname.includes("gelbooru.com") ||
    hostname.includes("safebooru.org") ||
    hostname.includes("danbooru.donmai.us") ||
    hostname.includes("rule34.xxx") ||
    hostname.includes("booru")
  ) {
    runWhenReady(do_booru);
  }

  // Instagram
  if (hostname === "www.instagram.com" || hostname === "instagram.com") {
    runWhenReady(do_instagram);
  }

  // DeviantArt
  if (hostname === "www.deviantart.com" || hostname === "deviantart.com") {
    runWhenReady(do_deviantart);
  }

  // ArtStation
  if (hostname === "www.artstation.com" || hostname === "artstation.com") {
    runWhenReady(do_artstation);
  }
})();
