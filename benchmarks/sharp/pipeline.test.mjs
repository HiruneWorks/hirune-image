import { test } from "node:test";
import assert from "node:assert/strict";
import sharp from "sharp";
import { insideDimensions, processImage, sha256 } from "./pipeline.mjs";

test("inside geometry uses the same integral target without floating scale loss", () => {
  assert.deepEqual(insideDimensions(22, 22, 15, 15), [15, 15]);
  assert.deepEqual(insideDimensions(1, 8192, 4096, 4096), [1, 4096]);
});

test("one decoded RGBA source produces sequential lossy/lossless variants with binding", async () => {
  const source = await sharp({ create: { width: 20, height: 10, channels: 4, background: { r: 255, g: 0, b: 0, alpha: .5 } } }).png().toBuffer();
  const recipe = Buffer.from(JSON.stringify({ contract_version: 1, animation_policy: "preserve", outputs: ["lossy", "lossless"].map((mode) => ({ id: mode, resize: { fit: "inside", max_width: 8, max_height: 8 }, encoding: { format: "webp", mode, effort: 0, ...(mode === "lossy" ? { quality: 85 } : {}) } })) }));
  const { manifest, buffers } = await processImage(source, recipe);
  assert.equal(manifest.source_sha256, sha256(source));
  assert.equal(manifest.recipe_sha256, sha256(recipe));
  for (const [i, output] of manifest.outputs.entries()) {
    assert.deepEqual([output.width, output.height, output.frame_count], [8, 4, 1]);
    assert.equal(output.sha256, sha256(buffers[i]));
  }
});

test("animated raw frame stack retains timing and loop", async () => {
  const rgba = Buffer.from([...Array(16).fill([255, 0, 0, 255]).flat(), ...Array(16).fill([0, 0, 255, 255]).flat()]);
  const source = await sharp(rgba, { raw: { width: 4, height: 8, pageHeight: 4, channels: 4 } }).gif({ keepDuplicateFrames: true, delay: [100, 300], loop: 3 }).toBuffer();
  const recipe = Buffer.from(JSON.stringify({ contract_version: 1, animation_policy: "preserve", outputs: [{ id: "small", resize: { fit: "inside", max_width: 2, max_height: 2 }, encoding: { format: "webp", mode: "lossless", effort: 0 } }] }));
  const { manifest } = await processImage(source, recipe);
  assert.equal(manifest.outputs[0].duration_ms, 400);
  assert.equal(manifest.outputs[0].loop_count, 3);
});
