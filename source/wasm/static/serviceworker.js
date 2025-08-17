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

const pathSplits = self.location.pathname.split("/");
pathSplits.pop();
const baseUrl = `${self.location.origin}${pathSplits.join("/")}`;
const doFetch = async (
  /** @type {Request} */ request0,
  /** @type {string} */ clientId
) => {
  let request = request0;

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
  const cache = await caches.open(CACHE);
  const cacheResp = await cache.match(request);
  if (cacheResp != null) {
    (async () => {
      try {
        const etag = "ETag";
        const cacheEtag = cacheResp.headers.get(etag);
        if (cacheEtag != null) {
          const headers = new Headers(request.headers);
          headers.set("If-None-Match", cacheEtag);
          request = new Request(request, {
            headers: headers,
            mode: "same-origin",
          });
          const newHeaders2 = [];
          for (const h of request.headers) {
            newHeaders2.push(h[0]);
          }
        } else {
          console.log(
            `Cached response for ${request.url} has no ETag, requesting as normal`
          );
        }
        const resp = await fetch(request);
        if (resp.status == 304) {
          // nop
        } else {
          console.log(
            `Reloading; request to [${
              request.url
            }] with etag [${cacheEtag}] returned non-304 with etag [${resp.headers.get(
              etag
            )}]`
          );
          const client = await self.clients.get(clientId);
          if (client != null) {
            client.postMessage("reload");
          }
          cache.put(request, resp);
        }
      } catch (e) {
        console.log("Error while doing background request handling", e);
        throw e;
      }
    })();
    return cacheResp;
  } else {
    const resp = await fetch(request);
    cache.put(request, resp.clone());
    return resp;
  }
};
addEventListener("fetch", (ev1) => {
  try {
    const ev = /** @type {FetchEvent} */ (ev1);
    return ev.respondWith(doFetch(ev.request, ev.clientId));
  } catch (e) {
    console.log("Error while handling request", e);
    throw e;
  }
});
