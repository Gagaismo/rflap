// `api(request)`: the one way the UI talks to rflap-api.
//
// A transport is `async (requestObject) => responseObject` (the `{ok, op, result}` / `{ok:false, error}`
// JSON of rflap-api). Local wasm is the default; when the host forbids WebAssembly (the default MCP Apps
// CSP has no 'wasm-unsafe-eval') the app switches to the remote transport, which asks the MCP server.

export class ApiError extends Error {
  constructor(error) {
    super(error && error.message ? error.message : 'unknown error');
    this.name = 'ApiError';
    this.code = (error && error.code) || 'internal';
    this.details = error && error.details;
  }
}

/** Transport over a synchronous `call(text) -> text` (the wasm caller). */
export function localTransport(call) {
  return async (request) => JSON.parse(call(JSON.stringify(request)));
}

/**
 * Transport that runs the request on the MCP server through the host (`tools/call`).
 * The tool takes `{request: "<json>"}` and answers the rflap-api response as `structuredContent`
 * (or as JSON text content).
 */
export function remoteTransport(host, toolName = 'rflap_call') {
  return async (request) => {
    const result = await host.callTool(toolName, { request: JSON.stringify(request) });
    if (result && result.structuredContent && 'ok' in result.structuredContent) {
      return result.structuredContent;
    }
    const text = result && result.content && result.content.find((c) => c.type === 'text');
    if (text) return JSON.parse(text.text);
    throw new Error('the server answered without a result');
  };
}

/**
 * Builds `api(request) -> result`. `getTransport` is called on every request so the app can swap
 * transports at runtime. Rejects with `ApiError` when the answer is `ok: false`.
 */
export function createApi(getTransport) {
  return async function api(request) {
    const transport = getTransport();
    if (!transport) throw new ApiError({ code: 'internal', message: 'the engine is not loaded' });
    const response = await transport(request);
    if (!response || response.ok !== true) throw new ApiError(response && response.error);
    return response.result;
  };
}
