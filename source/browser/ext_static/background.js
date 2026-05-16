// @ts-nocheck
const { default: init } = await import("./background2.js");
await init("./background2_bg.wasm");
