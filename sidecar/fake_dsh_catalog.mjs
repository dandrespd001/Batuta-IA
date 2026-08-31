export function listProviders() {
  return ["minimax", "opencode"];
}

export function listModels(provider) {
  return provider === "minimax" ? ["MiniMax-M2.1"] : ["free-model"];
}

export function resolveModelInfo(provider, model) {
  return {
    provider,
    model,
    revision: "2026-08",
    inputModalities: ["text"],
    contextWindow: 100000,
    reasoningEfforts: ["high", "low"],
    apiKey: "must-never-escape",
    balance: 999,
  };
}

export function stream() {
  throw new Error("stream must never be called by catalog sidecar");
}
