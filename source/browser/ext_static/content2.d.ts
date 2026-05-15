/**
 * Type declarations for the wasm-bindgen generated content2.js module.
 * This module is generated at build time by wasm-bindgen.
 */

/**
 * Initialize the WASM module.
 * @param wasmPath - Path to the .wasm file
 */
declare function init(wasmPath: string): Promise<void>;

export default init;

/**
 * Create a capture button element.
 * @param id - Unique identifier for the item being captured
 * @param view_query - The view query string for existence checking
 * @param callback - Callback function invoked on button click; should return a promise resolving to an object with form_id and parameters
 * @returns The created button HTMLElement
 */
export function create_capture_button(
  id: string,
  view_query: string,
  callback: (id: string) => Promise<{ form_id: string; parameters: Record<string, string | undefined> }>,
): HTMLElement;
