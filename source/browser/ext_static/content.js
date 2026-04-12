import init from "./content2.js";
import { do_twitter } from "./site_twitter.js";
import { do_booru } from "./site_booru.js";

init("./content2_bg.wasm");

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
