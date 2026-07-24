// Shared MSW mock-API server. The default handlers cover the health endpoint;
// individual tests override per-case with `server.use(...)`.
// This is the mock-API layer reused by the B3/C4/D2 component tests, and the
// multipart-capable helpers below are reused by the A4 (/open file input) and
// D3 (reply composer) component tests — see ROADMAP Decisions (M2) §13.
import { setupServer } from "msw/node";
import { http, HttpResponse, type HttpHandler } from "msw";
import type { DefaultBodyType, ResponseResolver } from "msw";

export const handlers = [
  http.get("/api/health", () =>
    HttpResponse.json({ status: "ok", db: "up" }),
  ),
  // Benign default so the global AppShell forced-password banner (which calls
  // ProfileStore.ensureLoaded() whenever a staff session is authenticated) does
  // not 500 in tests that don't care about the profile. Tests exercising the
  // profile override this with server.use(...).
  http.get("/api/staff/profile", () =>
    HttpResponse.json({
      id: 0,
      username: "",
      firstname: "",
      lastname: "",
      email: "",
      phone: "",
      phone_ext: "",
      mobile: "",
      signature: "",
      timezone_id: null,
      daylight_saving: false,
      max_page_size: 25,
      auto_refresh_rate: 0,
      default_signature_type: "none",
      default_paper_size: "Letter",
      change_passwd: false,
      onvacation: false,
      dept_id: 0,
    }),
  ),
];

export const server = setupServer(...handlers);

// Re-export so test files can build ad-hoc handlers without a second import.
export { http, HttpResponse };

/** A captured file part: metadata plus the raw bytes / decoded text content. */
export interface CapturedFilePart {
  name: string;
  type: string;
  size: number;
  bytes: Uint8Array;
  /** UTF-8 decode of the part body — convenient for small text fixtures. */
  text: string;
}

/**
 * A normalised, assertion-friendly view of a captured `multipart/form-data`
 * request: text parts as plain strings, file parts as metadata + raw bytes.
 */
export interface CapturedMultipart {
  /** Text (non-file) parts, keyed by part name. */
  fields: Record<string, string>;
  /** File parts (those with a `filename`), keyed by part name. */
  files: Record<string, CapturedFilePart>;
  /** The request's Content-Type header (carries the browser multipart boundary). */
  contentType: string | null;
}

const CRLF = "\r\n";

/**
 * Read a `multipart/form-data` request and split it into text vs. file parts.
 *
 * NOTE: this parses the raw request bytes by hand rather than calling
 * `request.formData()`. Under Vitest's jsdom environment, undici's built-in
 * multipart parser fails on its own serialized body, so `request.formData()`
 * is unusable there; reading the raw bytes (`request.arrayBuffer()`) works in
 * both jsdom and node and gives the A4 / D3 component tests a reliable way to
 * assert on uploaded parts. The parser is a minimal RFC 7578 reader — adequate
 * for test fixtures, not a hardened production parser.
 */
export async function readMultipart(request: Request): Promise<CapturedMultipart> {
  const contentType = request.headers.get("Content-Type");
  const fields: CapturedMultipart["fields"] = {};
  const files: CapturedMultipart["files"] = {};

  const boundary = /boundary=(?:"([^"]+)"|([^;]+))/i.exec(contentType ?? "");
  const marker = boundary?.[1] ?? boundary?.[2]?.trim();
  if (!marker) {
    throw new Error(`readMultipart: no boundary in Content-Type: ${contentType ?? "(none)"}`);
  }

  const raw = new Uint8Array(await request.arrayBuffer());
  // Latin-1 view preserves every byte 1:1 for header/structure scanning while
  // raw bytes are sliced out for file parts (so binary content stays intact).
  const latin1 = new TextDecoder("latin1").decode(raw);
  const delimiter = `--${marker}`;

  const segments = latin1.split(delimiter);
  let cursor = 0; // byte offset tracking, kept in lockstep with the latin1 split.
  for (const segment of segments) {
    const segStart = cursor;
    cursor += segment.length + delimiter.length;

    // Skip the preamble (before the first boundary) and the closing "--"/epilogue.
    if (segment.length === 0 || segment.startsWith("--")) continue;

    const headerEnd = segment.indexOf(CRLF + CRLF);
    if (headerEnd === -1) continue;
    const headerBlock = segment.slice(0, headerEnd);
    // Strip the leading CRLF after the boundary and the trailing CRLF before the next.
    const bodyStartInSeg = headerEnd + 4;
    const bodyText = segment.slice(bodyStartInSeg).replace(/\r\n$/, "");

    // Parse the part name and optional filename independently. The `\bname=`
    // word boundary stops `filename="..."` from being mis-read as the part name.
    const nameMatch = /(?:^|;|\s)name="([^"]*)"/i.exec(headerBlock);
    const fileMatch = /;\s*filename="([^"]*)"/i.exec(headerBlock);
    if (!nameMatch) continue;
    const partName = nameMatch[1];
    const filename = fileMatch ? fileMatch[1] : undefined;

    if (filename === undefined) {
      fields[partName] = bodyText;
      continue;
    }

    const typeMatch = /content-type:\s*([^\r\n]+)/i.exec(headerBlock);
    // Recover the exact body bytes by offset so binary fixtures survive.
    const bodyByteStart = segStart + bodyStartInSeg;
    const trailing = segment.slice(bodyStartInSeg).endsWith(CRLF) ? 2 : 0;
    const bodyByteEnd = segStart + segment.length - trailing;
    const bytes = raw.slice(bodyByteStart, bodyByteEnd);
    files[partName] = {
      name: filename,
      type: typeMatch ? typeMatch[1].trim() : "",
      size: bytes.byteLength,
      bytes,
      text: new TextDecoder().decode(bytes),
    };
  }

  return { fields, files, contentType };
}

/**
 * Build an MSW POST handler for a multipart upload. The captured parts are
 * passed to `onParts` so a test can assert on individual text/file parts; the
 * resolver's return value (default `{ ok: true }`) is sent as the JSON response.
 *
 * Consumed by the A4 (/open) and D3 (reply composer) component tests.
 */
export function multipartHandler(
  path: string,
  onParts: (captured: CapturedMultipart, request: Request) => DefaultBodyType | void = () => ({ ok: true }),
): HttpHandler {
  const resolver: ResponseResolver = async ({ request }) => {
    const captured = await readMultipart(request as Request);
    const result = onParts(captured, request as Request);
    return HttpResponse.json((result ?? { ok: true }) as DefaultBodyType);
  };
  return http.post(path, resolver);
}
