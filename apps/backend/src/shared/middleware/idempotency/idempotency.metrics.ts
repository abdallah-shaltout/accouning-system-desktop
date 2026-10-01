import client from "prom-client";

/**
 * The reference imports these three counters from a shared `@@shared/metrics` registry module that
 * does not exist in this project yet (no metrics module has landed under src/shared/metrics). Rather
 * than invent cross-cutting metrics infrastructure outside the paths this task owns, the counters are
 * registered directly against prom-client's default global registry here — any future `/metrics`
 * exporter (Phase A manager's call) will pick them up automatically via `client.register`, since
 * prom-client Counters register themselves globally unless given an explicit `registers: []`.
 */
export const idempotencyHitsTotal = new client.Counter({
    name: "idempotency_hits_total",
    help: "Total idempotency middleware decisions",
    labelNames: ["scope", "result"] as const,
});

export const idempotencyPendingContentionsTotal = new client.Counter({
    name: "idempotency_pending_lock_contentions_total",
    help: "Requests that hit a pending lock",
    labelNames: ["scope"] as const,
});

export const idempotencyStoreErrorsTotal = new client.Counter({
    name: "idempotency_store_errors_total",
    help: "Redis errors inside the idempotency store",
    labelNames: ["operation"] as const,
});
