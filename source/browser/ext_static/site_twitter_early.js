// Early content script (document_start, ISOLATED world) that injects the
// fetch interceptor into the page context via a script src (avoids CSP inline block).
{
  const s = document.createElement('script');
  s.src = /** @type {any} */ (globalThis).browser.runtime.getURL('site_twitter_intercept.js');
  document.documentElement.prepend(s);
}
