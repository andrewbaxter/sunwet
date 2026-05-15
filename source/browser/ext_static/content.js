/** @type {{runtime: {getURL: (path: string) => string}}} */
const browser = /** @type {any} */ (globalThis).browser;

// Wrapper for navigator.locks.request that uses `await` instead of `.then()`
// to avoid Firefox "Permission denied to access property 'then'" errors
// on cross-context promises in content scripts.
globalThis.__sunwet_request_lock = async (/** @type {string} */ name, /** @type {LockGrantedCallback<any>} */ cb) => {
  return await globalThis.navigator.locks.request(name, cb);
};

(async () => {
  const src = browser.runtime.getURL("content2.js");
  const { default: init } = await import(src);
  const { do_twitter } = await import(browser.runtime.getURL("site_twitter.js"));
  const { do_booru } = await import(browser.runtime.getURL("site_booru.js"));

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
})();
