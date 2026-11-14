// Base64 codec for row-major float32 little-endian pixel payloads
// (Frame.data_b64 in shared/openapi-contract.json). Pure JS, no DOM/Buffer.

const B64_ALPHABET =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_REVOKE: Record<string, number> = {};
for (let i = 0; i < B64_ALPHABET.length; i++) {
  const ch = B64_ALPHABET.charAt(i);
  if (ch !== "") B64_REVOKE[ch] = i;
}

export function base64ToBytes(b64: string): Uint8Array {
  const clean = b64.replace(/[\r\n\s]/g, "");
  if (clean.length % 4 !== 0) throw new Error("invalid base64 length");
  const out = new Uint8Array((clean.length / 4) * 3);
  let o = 0;
  for (let i = 0; i < clean.length; i += 4) {
    const chunk = clean.slice(i, i + 4);
    const pad = chunk.indexOf("=");
    if (pad !== -1 && pad < 2) throw new Error("invalid base64 padding");
    const v =
      ((B64_REVOKE[chunk.charAt(0)] ?? -1) << 18) |
      ((B64_REVOKE[chunk.charAt(1)] ?? -1) << 12) |
      ((pad === 2 ? 0 : (B64_REVOKE[chunk.charAt(2)] ?? -1)) << 6) |
      (pad !== -1 ? 0 : (B64_REVOKE[chunk.charAt(3)] ?? -1));
    if (v < 0) throw new Error("invalid base64 character");
    const b0 = (v >> 16) & 0xff;
    const b1 = (v >> 8) & 0xff;
    const b2 = v & 0xff;
    if (o < out.length) out[o++] = b0;
    if (pad !== 2 && o < out.length) out[o++] = b1;
    if (pad === -1 && o < out.length) out[o++] = b2;
  }
  return out.subarray(0, o);
}

export function base64ToFloat32(b64: string): Float32Array {
  const bytes = base64ToBytes(b64);
  if (bytes.length % 4 !== 0)
    throw new Error("float32 payload is not a multiple of 4 bytes");
  const copy = new Uint8Array(bytes.length);
  copy.set(bytes);
  return new Float32Array(copy.buffer);
}

export function float32ToBase64(data: Float32Array): string {
  const bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const b0 = bytes[i] ?? 0;
    const b1 = bytes[i + 1];
    const b2 = bytes[i + 2];
    const v = (b0 << 16) | ((b1 ?? 0) << 8) | (b2 ?? 0);
    out += B64_ALPHABET.charAt((v >> 18) & 63);
    out += B64_ALPHABET.charAt((v >> 12) & 63);
    out += b1 === undefined ? "=" : B64_ALPHABET.charAt((v >> 6) & 63);
    out += b2 === undefined ? "=" : B64_ALPHABET.charAt(v & 63);
  }
  return out;
}
