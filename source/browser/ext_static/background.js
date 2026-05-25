// @ts-nocheck

// Fetch with Referer header injection via webRequest (Fetch API forbids
// setting Referer directly). Exposed on globalThis for wasm to call.
globalThis.sunwetFetchWithReferer = (url, referer) => {
  let listener;
  if (referer) {
    listener = (details) => {
      const headers = details.requestHeaders.filter(
        (h) => h.name.toLowerCase() !== "referer",
      );
      headers.push({ name: "Referer", value: referer });
      return { requestHeaders: headers };
    };
    browser.webRequest.onBeforeSendHeaders.addListener(
      listener,
      { urls: [url] },
      ["blocking", "requestHeaders"],
    );
  }
  return fetch(url)
    .then(async (resp) => {
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const mimeType = resp.headers.get("Content-Type");
      const buffer = await resp.arrayBuffer();
      return { data: new Uint8Array(buffer), mimeType };
    })
    .finally(() => {
      if (listener) {
        browser.webRequest.onBeforeSendHeaders.removeListener(listener);
      }
    });
};

const { default: init } = await import("./background2.js");
await init("./background2_bg.wasm");
