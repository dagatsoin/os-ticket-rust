// Vitest global setup — jest-dom matchers + MSW server lifecycle.
// Reused by every component/unit test (B3/C4/D2 build on this harness).
import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll } from "vitest";
import { cleanup } from "@testing-library/react";
import { File as NodeFile, Blob as NodeBlob } from "node:buffer";
import { server } from "./mockServer";

// --- multipart globals (TS-M2-A0; consumed by A4 / D3 upload tests) ---
// The jsdom environment overrides FormData/File/Blob with its own classes.
// When a jsdom FormData is passed to fetch() (undici, via MSW node
// interception), serialization deadlocks `request.arrayBuffer()`/`formData()`.
// Restore the platform FormData (recovered from a simple multipart Response —
// undici did not clobber Response/Request/fetch) plus Node's File/Blob, so a
// FormData body the apiClient sends serializes into a real, readable multipart
// stream. The mock handlers parse that stream with readMultipart() (manual
// parser) rather than undici's formData(), which is broken under jsdom.
async function installNativeMultipartGlobals(): Promise<void> {
  const form = await new Response(
    '--b\r\nContent-Disposition: form-data; name="x"\r\n\r\n1\r\n--b--\r\n',
    { headers: { "content-type": "multipart/form-data; boundary=b" } },
  ).formData();
  (globalThis as unknown as { FormData: typeof FormData }).FormData =
    form.constructor as typeof FormData;
  (globalThis as unknown as { File: typeof File }).File = NodeFile as unknown as typeof File;
  (globalThis as unknown as { Blob: typeof Blob }).Blob = NodeBlob as unknown as typeof Blob;
}

// jsdom does not implement URL.createObjectURL / revokeObjectURL. The B2
// download path (fetch→blob→objectURL) calls them; provide inert stubs so the
// page-level tests can drive a download without a real save (the download
// hooks are unit-tested via the injectable seam in AttachmentList.test.tsx).
function installObjectUrlStubs(): void {
  const u = globalThis.URL as unknown as {
    createObjectURL?: (b: Blob) => string;
    revokeObjectURL?: (s: string) => void;
  };
  if (typeof u.createObjectURL !== "function") u.createObjectURL = () => "blob:stub";
  if (typeof u.revokeObjectURL !== "function") u.revokeObjectURL = () => {};
}

beforeAll(async () => {
  await installNativeMultipartGlobals();
  installObjectUrlStubs();
  server.listen({ onUnhandledRequest: "error" });
});
afterEach(() => {
  cleanup();
  server.resetHandlers();
});
afterAll(() => server.close());
