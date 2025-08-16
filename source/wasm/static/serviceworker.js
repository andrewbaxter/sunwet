const CACHE = "cache1";
addEventListener("install", async () => {
  self.skipWaiting();
});
addEventListener("activate", async (ev) =>
  ev.waitUntil(
    (async () => {
      const bg = [];
      for (const k of await caches.keys()) {
        if (k !== CACHE) {
          bg.push(caches.delete(k));
        }
      }
      return Promise.all(bg);
    })()
  )
);

let wantReload = false;
let outstandingReqs = 0;
/** @type (number|null) */
let debounceRefresh = null;
const incOutstanding = () => {
  if (debounceRefresh != null) {
    clearTimeout(debounceRefresh);
    debounceRefresh = null;
  }
};
const decOutstanding = () => {
  outstandingReqs -= 1;
  if (outstandingReqs == 0 && wantReload) {
    debounceRefresh = setTimeout(() => {
      window.location.reload();
    }, 10000);
  }
};
const pathSplits = self.location.pathname.split("/");
pathSplits.pop();
const baseUrl = `${self.location.origin}${pathSplits.join("/")}`;
const doFetch = async (/** @type {Request} */ request) => {
  // Dynamic requests are cached/downloaded at a different level, don't handle here.
  // I.e. the below filters should only allow root level static files.
  if (!request.url.startsWith(baseUrl) || request.method != "GET") {
    return await fetch(request);
  }
  const relPath = request.url.slice(baseUrl.length);
  if (
    !request.url.startsWith(baseUrl) ||
    request.method != "GET" ||
    relPath.startsWith("/api") ||
    relPath.startsWith("/oidc") ||
    relPath.startsWith("/logout") ||
    relPath.startsWith("/file")
  ) {
    return await fetch(request);
  }

  // Handle caching of app static requests
  incOutstanding();
  const cache = await caches.open(CACHE);
  const cacheResp = await cache.match(request);
  if (cacheResp != null) {
    (async () => {
      const etag = "ETag";
      const cacheEtag = cacheResp.headers.get(etag);
      if (cacheEtag != null) {
        request.headers.set("If-None-Match", cacheEtag);
      }
      try {
        const resp = await fetch(request);
        if (resp.status != 304) {
          if (!wantReload) {
            window.postMessage({
              log: `Reloading; static request to [${
                request.url
              }] with etag [${cacheEtag}] returned non-304 with etag [${resp.headers.get(
                etag
              )}]`,
            });
          }
          wantReload = true;
          cache.put(request, resp);
        }
      } finally {
        decOutstanding();
      }
    })();
    return cacheResp;
  } else {
    try {
      const resp = await fetch(request);
      cache.put(request, resp.clone());
      return resp;
    } finally {
      decOutstanding();
    }
  }
};
addEventListener("fetch", (ev) => ev.respondWith(doFetch(ev.request)));
