#!/usr/bin/env node

const MAX_REQUEST_BYTES = 64 * 1024;

function fail(id, code, field, message, details = {}) {
  return { schema_version: 1, id, ok: false, error: { code, field, message, details } };
}

function exactKeys(value, keys) {
  return value && typeof value === "object" && !Array.isArray(value) &&
    Object.keys(value).sort().join("\0") === [...keys].sort().join("\0");
}

function providerId(value) {
  if (typeof value === "string") return value;
  return value?.id ?? value?.provider ?? value?.name;
}

function modelId(value) {
  if (typeof value === "string") return value;
  return value?.id ?? value?.model ?? value?.name;
}

function textArray(value) {
  if (!Array.isArray(value)) return [];
  return value.filter((item) => typeof item === "string").sort();
}

async function snapshot(llm) {
  if (!llm || typeof llm.listProviders !== "function" ||
      typeof llm.listModels !== "function" ||
      typeof llm.resolveModelInfo !== "function") {
    throw new Error("configured module must expose listProviders, listModels and resolveModelInfo");
  }
  const providers = [...await llm.listProviders()].map(providerId).filter(Boolean).sort();
  const routes = [];
  for (const provider of providers) {
    const models = [...await llm.listModels(provider)].map(modelId).filter(Boolean).sort();
    for (const model of models) {
      const info = await llm.resolveModelInfo(provider, model);
      routes.push({
        provider,
        model,
        revision: typeof info?.revision === "string" ? info.revision : null,
        modalities: textArray(info?.inputModalities ?? info?.modalities),
        context_window: Number.isSafeInteger(info?.contextWindow) && info.contextWindow > 0
          ? info.contextWindow : null,
        reasoning_efforts: textArray(info?.reasoningEfforts),
        cost: { input: null, output: null, cache_read: null, cache_write: null },
      });
    }
  }
  routes.sort((a, b) => `${a.provider}\0${a.model}\0${a.revision ?? ""}`
    .localeCompare(`${b.provider}\0${b.model}\0${b.revision ?? ""}`));
  return { routes };
}

async function handle(line) {
  let request;
  try {
    request = JSON.parse(line);
  } catch (error) {
    return fail(null, "invalid_json", "request", String(error));
  }
  const id = typeof request?.id === "string" ? request.id : null;
  if (!exactKeys(request, ["schema_version", "id", "method"])) {
    return fail(id, "invalid_request", "request", "request fields must be exactly schema_version, id and method");
  }
  if (request.schema_version !== 1 || request.method !== "catalog_snapshot" || !id) {
    return fail(id, "invalid_request", "method", "expected JSONL v1 catalog_snapshot request");
  }
  const moduleRef = process.env.BATUTA_DSH_CATALOG_MODULE;
  if (!moduleRef) {
    return fail(id, "configuration_error", "BATUTA_DSH_CATALOG_MODULE", "catalog module is not configured");
  }
  try {
    const imported = await import(moduleRef);
    const llm = imported.default ?? imported.llm ?? imported;
    const result = await snapshot(llm);
    return { schema_version: 1, id, ok: true, result };
  } catch (error) {
    return fail(id, "catalog_error", "catalog", String(error));
  }
}

let input = "";
let oversized = false;
for await (const chunk of process.stdin) {
  input += chunk;
  if (Buffer.byteLength(input) > MAX_REQUEST_BYTES) {
    process.stdout.write(`${JSON.stringify(fail(null, "request_too_large", "request", "request exceeds 65536 bytes"))}\n`);
    oversized = true;
    break;
  }
}

if (oversized) {
  process.exitCode = 2;
} else {
  const lines = input.split(/\r?\n/).filter((line) => line.length > 0);
  if (lines.length !== 1) {
    process.stdout.write(`${JSON.stringify(fail(null, "protocol_error", "request", "exactly one request line is required"))}\n`);
    process.exitCode = 2;
  } else {
    process.stdout.write(`${JSON.stringify(await handle(lines[0]))}\n`);
    process.exitCode = 0;
  }
}
