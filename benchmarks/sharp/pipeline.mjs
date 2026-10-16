import sharp from "sharp";
import { createHash } from "node:crypto";

sharp.concurrency(Number(process.env.SHARP_CONCURRENCY ?? 1));
sharp.cache(false);
export const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
export const processor = { name: "standalone-sharp-benchmark", node: process.version, versions: sharp.versions, native_concurrency: sharp.concurrency(), cache: sharp.cache() };

export function insideDimensions(width, height, maxWidth, maxHeight) {
  if (width <= maxWidth && height <= maxHeight) return [width, height];
  if (maxWidth * height <= maxHeight * width) return [maxWidth, Math.max(1, Math.floor(height * maxWidth / width))];
  return [Math.max(1, Math.floor(width * maxHeight / height)), maxHeight];
}

function facts(metadata) {
  const count = metadata.pages ?? 1;
  return { width: metadata.width, height: metadata.pageHeight ?? metadata.height,
    frame_count: count, duration_ms: count > 1 ? metadata.delay.reduce((a, b) => a + b, 0) : 0,
    loop_count: count > 1 ? metadata.loop ?? 1 : 1 };
}

export async function processImage(source, rawRecipe) {
  const recipe = JSON.parse(rawRecipe.toString("utf8"));
  if (recipe.contract_version !== 1 || recipe.animation_policy !== "preserve" || !Array.isArray(recipe.outputs) || recipe.outputs.length < 1 || recipe.outputs.length > 1024) throw new Error("invalid benchmark recipe");
  const metadata = await sharp(source, { animated: true }).metadata();
  if (!["jpeg", "png", "gif", "webp"].includes(metadata.format)) throw new Error("unsupported benchmark source");
  const sourceFacts = facts(metadata);
  // Decode source pixels once. The metadata inspection above reads headers without decoding pixels.
  const decode = sharp(source, { animated: true }).autoOrient().ensureAlpha();
  if (metadata.icc) decode.withIccProfile("srgb", { attach: false });
  else decode.toColourspace("srgb");
  const { data: rgba, info } = await decode.raw().toBuffer({ resolveWithObject: true });
  const pages = sourceFacts.frame_count;
  const pageHeight = info.height / pages;
  const outputs = [];
  const buffers = [];
  for (const output of recipe.outputs) {
    const { max_width, max_height, fit } = output.resize;
    if (!/^[a-z][a-z0-9_]{0,31}$/.test(output.id) || fit !== "inside" || !Number.isInteger(max_width) || !Number.isInteger(max_height) || Math.min(max_width, max_height) < 1 || Math.max(max_width, max_height) > 16383) throw new Error("invalid benchmark output");
    const [width, height] = insideDimensions(info.width, pageHeight, max_width, max_height);
    const encoding = output.encoding;
    if (encoding.format !== "webp" || !["lossy", "lossless"].includes(encoding.mode)) throw new Error("invalid benchmark encoding");
    const buffer = await sharp(rgba, { animated: true, raw: { width: info.width, height: info.height, channels: 4, ...(pages > 1 ? { pageHeight } : {}) } })
      .resize({ width, height, fit: "fill", kernel: "lanczos3" })
      .webp({ lossless: encoding.mode === "lossless", quality: encoding.quality ?? 75, effort: encoding.effort,
        alphaQuality: 100, exact: true, ...(pages > 1 ? { loop: sourceFacts.loop_count, delay: metadata.delay } : {}) }).toBuffer();
    const actual = await sharp(buffer, { animated: true }).metadata();
    // Decode all output pixels for verification, as Hirune does. Metadata and hash checks are separate.
    await sharp(buffer, { animated: true }).raw().toBuffer();
    const outputFacts = facts(actual);
    if (outputFacts.width !== width || outputFacts.height !== height || outputFacts.duration_ms !== sourceFacts.duration_ms || outputFacts.loop_count !== sourceFacts.loop_count) throw new Error("output verification failed: " + JSON.stringify({ sourceFacts, outputFacts, expected: [width, height], raw: info }));
    outputs.push({ id: output.id, media_type: "image/webp", byte_size: buffer.length, sha256: sha256(buffer), ...outputFacts });
    buffers.push(buffer);
  }
  return { manifest: { contract_version: 1, processor, source_sha256: sha256(source), recipe_sha256: sha256(rawRecipe),
    source: { format: metadata.format, byte_size: source.length, orientation: metadata.orientation ?? 1, ...sourceFacts }, outputs }, buffers };
}

export function multipart(manifest, buffers, boundary) {
  const part = (type, id, data) => [Buffer.from(`--${boundary}\r\nContent-Type: ${type}\r\nContent-ID: <${id}>\r\nContent-Length: ${data.length}\r\n\r\n`), data, Buffer.from("\r\n")];
  return Buffer.concat([...part("application/json", "manifest", Buffer.from(JSON.stringify(manifest))),
    ...buffers.flatMap((buffer, i) => part("image/webp", `output-${manifest.outputs[i].id}`, buffer)), Buffer.from(`--${boundary}--\r\n`)]);
}
