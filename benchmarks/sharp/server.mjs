// Use this service only with trusted benchmark fixtures. It is not a production image service.
import http from "node:http";
import { randomUUID } from "node:crypto";
import { multipart, processImage, processor } from "./pipeline.mjs";
const concurrency = Number(process.env.REQUEST_CONCURRENCY ?? 1);
let active = 0;
http.createServer(async (request, response) => {
  if (request.url === "/health/ready") { response.writeHead(204).end(); return; }
  if (request.url === "/v1/capabilities") { response.writeHead(200, { "content-type": "application/json" }).end(JSON.stringify({ processor, concurrency })); return; }
  if (request.url !== "/v1/images/process" || request.method !== "POST") { response.writeHead(404).end(); return; }
  if (active >= concurrency) { response.writeHead(503).end(); return; }
  active += 1;
  let released = false;
  const release = () => { if (!released) { released = true; active -= 1; } };
  response.once("close", release);
  response.once("finish", release);
  try {
    const chunks = [];
    let bytes = 0;
    for await (const chunk of request) {
      bytes += chunk.length;
      if (bytes > 33 * 1024 * 1024) throw new Error("benchmark request too large");
      chunks.push(chunk);
    }
    const form = await new Request("http://benchmark/", { method: "POST", headers: { "content-type": request.headers["content-type"] }, body: Buffer.concat(chunks) }).formData();
    const file = form.get("file"), recipe = form.get("recipe");
    const source = Buffer.from(await file.arrayBuffer());
    const rawRecipe = typeof recipe === "string" ? Buffer.from(recipe) : Buffer.from(await recipe.arrayBuffer());
    const { manifest, buffers } = await processImage(source, rawRecipe);
    const boundary = "sharp-" + randomUUID();
    const body = multipart(manifest, buffers, boundary);
    response.writeHead(200, { "content-type": `multipart/related; boundary=${boundary}`, "content-length": body.length }).end(body);
  } catch {
    response.writeHead(500).end("benchmark processing failed");
  }
}).listen(Number(process.env.PORT ?? 8080), "0.0.0.0");
