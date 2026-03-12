# WASM, Edge Workers, and GraphQL: A Technology Landscape

**Date:** 2026-03-08

This document evaluates the performance and trade-offs of running GraphQL servers (and federation gateways) as Rust/WASM on CDN edge workers, compared to alternative languages, runtimes, and deployment models.

---

## 1. Rust/WASM vs Go: Performance Comparison

### Cold Start

WASM-based edge platforms eliminate the cold start problem that plagues container and VM-based serverless:

| Platform | Cold Start | Mechanism |
|---|---|---|
| Cloudflare Workers (V8 isolates) | ~0ms perceived | Parallelizes instantiation with TLS handshake |
| Fastly Compute (Wasmtime) | ~10ms | Native WASM runtime, microsecond-level for small modules |
| Fermyon Spin (WASM) | <5ms | Wasmtime-based |
| AWS Lambda (Node.js) | ~100ms | Firecracker microVM |
| AWS Lambda (JVM) | 1,000ms+ | Worst case |

WASM module instantiation breaks down to roughly: 2-5ms module instantiation + 1-3ms memory allocation + <1ms capability linking. Cloudflare runs 1,000+ Worker isolates per process, amortizing the runtime overhead.

Neither Rust nor Go has a meaningful cold start advantage on these platforms -- the WASM instantiation cost dominates and is small in both cases. Go's larger binary size (see below) may add marginally to instantiation time but remains well under 10ms.

### Request Latency

At P95, Cloudflare Workers delivers **40ms** response times vs Lambda@Edge at **216ms** and Lambda at **882ms**. The edge location proximity accounts for most of this gap, not the language.

Within the worker itself, Cloudflare reports the average Worker uses approximately **2.2ms CPU time per request**. For a GraphQL proxy like ours -- parsing a query, making 1-2 upstream HTTP calls, and assembling a response -- CPU time is dwarfed by upstream fetch latency.

### WASM Overhead vs Native

Compiling to WASM adds overhead compared to running the same code natively. Micro-benchmarks show:

| Language | Native | WASM | Overhead |
|---|---|---|---|
| Rust | 0.0006s | 0.003s | ~5x |
| Go | 0.0013s | 0.017s | ~13x |

Source: [karnwong.me benchmark](https://karnwong.me/posts/2024/12/native-implementation-vs-wasm-for-go-python-and-rust-benchmark/) (simple multiply function). Real-world overhead is lower for I/O-bound workloads where CPU is not the bottleneck -- which describes most GraphQL gateway patterns.

**Takeaway:** Rust produces faster WASM than Go by roughly 2-3x in compute-bound scenarios. For I/O-bound GraphQL proxying, both are fast enough that the difference rarely matters.

### Binary Size

| Language/Toolchain | WASM Binary | Notes |
|---|---|---|
| Rust (optimized) | 44-200 KB | With `lto = true`, `opt-level = "s"`, `codegen-units = 1` |
| TinyGo | ~20 KB | Subset of Go; missing some std library features |
| Go (standard compiler) | 1.2 MB+ | Includes Go runtime; requires paid CF plan (3 MB limit on free) |
| AssemblyScript | ~3.5 KB | TypeScript subset |

Binary size matters on edge platforms with hard limits. Cloudflare Workers allows 3 MB compressed (free) or 10 MB compressed (paid), with 64 MB uncompressed max. Rust stays comfortably within these limits. Standard Go binaries are larger but still workable on paid plans.

### Memory

Cloudflare Workers enforces a **128 MB per isolate** limit (JS heap + WASM linear memory combined). Rust WASM is frugal -- a sorting benchmark (100K elements, 500 iterations) used ~21 MB in Rust/WASM vs ~55 MB for the equivalent JavaScript. For a GraphQL proxy handling typical query payloads, memory is rarely the binding constraint.

---

## 2. WASM for GraphQL Servers and Federation Gateways

### Federation Gateway Benchmarks

The GraphQL federation gateway space has coalesced around a few Rust and Go implementations. Benchmark results from the community-maintained [graphql-gateways-benchmark](https://github.com/graphql-hive/graphql-gateways-benchmark) (September 2025, constant-rate testing):

| Gateway | Language | License | RPS | P99 Latency |
|---|---|---|---|---|
| Hive Router | Rust | MIT | 1,827 | 79ms |
| Cosmo Router | Go | Apache 2.0 | 571 | 348ms |
| Grafbase Gateway | Rust | MPL 2.0 | 451 | -- |
| Apollo Router | Rust | Elastic V2 | 317 | 496ms |

Rust gateways dominate the top of the benchmark, but implementation quality varies widely -- Apollo Router (Rust) underperforms Cosmo Router (Go). Language choice is necessary but not sufficient; architecture and query planning matter more.

### Notable Gateways

**Hive Router** (Rust, MIT) -- Highest throughput and lowest latency in benchmarks. From The Guild, who also maintain the benchmark suite. Fully open source.

**Grafbase Gateway** (Rust, MPL 2.0) -- Self-hosted federation gateway with WASM hooks for extensibility (auth, custom resolvers). Claims 40% better CPU and memory usage vs competitors. First gateway with built-in MCP server support. Uses a Steiner tree query planner with lock-free execution DAG.

**Cosmo Router** (Go, Apache 2.0) -- Built on `graphql-go-tools`. Offers "Ludicrous Mode" with single-flight request deduplication. Compatible with Apollo Federation v1 and v2. 3.3x faster than its predecessor.

**Apollo Router** (Rust, Elastic V2) -- The incumbent. Restrictive license (not OSI open source). Underperforms in independent benchmarks despite being written in Rust.

### Pros of WASM for GraphQL Servers

- **Near-zero cold starts** compared to container-based gateways
- **Sandboxed execution** -- WASM's capability-based security model limits blast radius
- **Polyglot extensibility** -- gateways like Grafbase use WASM hooks, allowing users to write plugins in any language that compiles to WASM
- **Predictable performance** -- no GC pauses (for Rust/C/C++), deterministic memory management
- **Edge deployment** -- run federation at the edge, close to users, rather than in a central region

### Cons of WASM for GraphQL Servers

- **Platform resource limits** -- Cloudflare's 128 MB memory, 10 MB compressed binary, and CPU time limits are real constraints. One practitioner [reported](https://nickb.dev/blog/reality-check-for-cloudflare-wasm-workers-and-rust/) hitting limits "at every turn" with Rust WASM Workers, including 2.5-3s extra latency on cold requests and only 1-in-20 requests served warm under intermittent traffic
- **Ecosystem gaps** -- some Rust crates don't compile to `wasm32-unknown-unknown` (anything touching OS-level features: filesystem, threads, raw sockets)
- **Debugging difficulty** -- WASM stack traces are less readable than native. Profiling tools are immature compared to native Rust or Go
- **No persistent connections** -- edge workers are stateless and short-lived. No gRPC streaming, no long-lived DB connection pools. Upstream calls must be HTTP request/response
- **Complex schema performance** -- deeply nested or high-cardinality federation queries can exhaust CPU time limits. A single complex query hitting 10+ subgraph services may time out on platforms with 30s wall-clock limits

### Maintainability

| Dimension | Rust/WASM | Go (native) |
|---|---|---|
| Compilation | Slower (cross-compile to wasm32) | Faster |
| Type safety | Strong (borrow checker, enums) | Adequate (simpler type system) |
| Error messages | Excellent at compile time, poor at WASM runtime | Good at both |
| Library ecosystem | async-graphql is mature and well-maintained | graphql-go-tools is production-proven |
| Hiring | Smaller talent pool | Larger talent pool |
| Debugging | Harder (WASM indirection) | Easier (native binary) |
| Refactoring | Compiler catches more | Easier to learn, more runtime surprises |

---

## 3. Languages That Compile to WASM

### Maturity for Server-Side WASM

| Language | WASM Adoption | Server-Side Readiness | Binary Size | Key Limitation |
|---|---|---|---|---|
| **Rust** | 46.8% | Production-ready | 44-200 KB | Steep learning curve |
| **C/C++** | 17.4% | Production-ready | Very small | Memory safety burden; less ergonomic for web services |
| **AssemblyScript** | Niche | Production-ready | ~3.5 KB | TypeScript subset; limited ecosystem; ~2x slower than Rust |
| **Go (TinyGo)** | 8.3% | Usable with caveats | ~20 KB (TinyGo) | TinyGo missing std library features; standard Go produces large binaries |
| **C# / .NET** | 11.9% | Browser production-ready | Large | Server-side WASI still emerging |
| **Kotlin/Wasm** | 18.4% | Emerging | Moderate | Requires WasmGC (Safari added Dec 2024) |
| **Zig** | 3.7% | Experimental | Very small | Small ecosystem; early WASM tooling |
| **Swift** | 1.8% | Experimental | Unknown | SwiftWasm project; limited adoption |

Adoption percentages from the [State of WebAssembly 2025 survey](https://platform.uno/blog/the-state-of-webassembly-2025-2026/).

### Production-Ready Tier List

1. **Rust** -- The default choice. Best tooling, smallest binaries, fastest execution, largest WASM ecosystem. Every major edge platform supports it as a first-class citizen.
2. **C/C++** -- Mature via Emscripten (browser) and WASI-SDK (server). Practical for porting existing C/C++ codebases. Not a good choice for greenfield web services.
3. **AssemblyScript** -- Lowest barrier to entry for TypeScript developers. Tiny binaries. Good for lightweight edge functions but limited for complex applications.
4. **Go (via TinyGo)** -- Workable for simple services. Standard Go compiler produces large WASM binaries. TinyGo trades binary size for missing std library features (reflection, full concurrency).

Everything else (Kotlin, C#, Zig, Swift) is pre-production for server-side WASM as of early 2026.

### WASI Status

WASI Preview 2 (0.2) launched January 2024, standardizing host/guest communication via the Component Model. WASI 0.3 is in development, adding native async support. Wasmtime has experimental 0.3 support. The trajectory is toward a portable, language-agnostic server-side runtime, but the ecosystem is still stabilizing.

---

## 4. Edge Workers vs Kubernetes (EKS)

### Latency

| Scenario | Edge Workers | Kubernetes (Centralized) |
|---|---|---|
| Cold start | 0-10ms | Pod spin-up: seconds to minutes (mitigated by keeping pods warm) |
| Geographic latency to user | <15ms avg (300+ DCs) | 50-200ms+ depending on region proximity |
| Warm request CPU overhead | 2-5ms typical | Sub-millisecond within cluster |
| End-to-end P95 | ~40ms (Cloudflare) | Depends on deployment region vs user location |

Edge workers save 50-70% of round-trip time for geographically distributed users. For users co-located with the Kubernetes cluster, the difference narrows.

However, if the edge worker must call an origin API in a specific region (as in this project), the edge-to-origin hop partially negates the latency advantage. The benefit is greatest when the edge can serve from cache or when the origin is replicated across regions.

### Cost

| Dimension | Cloudflare Workers | Kubernetes (EKS) |
|---|---|---|
| Base cost | $5/mo (10M requests, 30M CPU-ms) | ~$73/mo EKS control plane + node costs |
| 15M requests/mo, 7ms avg CPU | ~$8/mo | ~$180/mo (2-node cluster) |
| Scaling cost | Linear: $0.30 per additional 1M requests | Step function: add nodes at capacity boundaries |
| Egress | Free | $0.09/GB (AWS) |
| Idle cost | $0 (pay-per-request) | Full node cost even at 0 requests |

Edge workers are dramatically cheaper for low-to-moderate traffic (under ~50M requests/month). Kubernetes becomes cost-competitive at high sustained throughput where node utilization stays above 60%, amortizing the always-on cost.

### Operational Complexity

| Dimension | Edge Workers | Kubernetes |
|---|---|---|
| Deployment | `wrangler deploy` (seconds) | Helm charts, CI/CD pipelines, rolling updates, image registries |
| Scaling | Automatic, global, instant | HPA/VPA configuration, cluster autoscaler, capacity planning |
| Monitoring | Platform-provided (limited customization) | Full control (Prometheus, Grafana, OTel) |
| Networking | Fully managed, no VPC | Full control: service mesh, network policies, ingress controllers |
| TLS/DNS | Managed | cert-manager, external-dns, or manual |
| State | KV, Durable Objects, D1 (platform-specific) | Any database, Redis, persistent volumes |
| On-call burden | Minimal (platform manages infra) | Significant (node failures, etcd, upgrades, security patches) |

### Edge Worker Limitations

Hard constraints that may be deal-breakers depending on workload:

| Constraint | Cloudflare Workers | Kubernetes |
|---|---|---|
| CPU time per request | 10ms (free) / 30s (paid) / 5 min max | Unlimited (configurable) |
| Memory per instance | 128 MB | Unlimited (configurable) |
| Binary size | 10 MB compressed / 64 MB uncompressed | Unlimited |
| Outbound connections | 6 concurrent per request | Unlimited |
| Request duration | 30s default, 5 min max | Unlimited |
| Persistent connections | No (HTTP req/res only; Durable Objects for WebSocket) | Yes (gRPC streaming, DB connection pools, WebSocket) |
| Filesystem | None | Full |
| GPU | None | Yes |
| Background processing | Limited (waitUntil, Queues, Cron Triggers) | Full (batch jobs, queues, cron) |
| Database connectivity | HTTP-based only; no raw TCP DB connections | Direct connections, connection pooling |

### When to Use Which

**Edge workers are the better fit when:**
- Traffic is geographically distributed and latency-sensitive
- Workload is request/response (not streaming)
- Each request is lightweight (<30s, <128 MB)
- Traffic is variable or low-to-moderate (pay-per-request wins)
- Operational simplicity is valued over control
- The service is a proxy, gateway, or API facade (not compute-heavy)

**Kubernetes is the better fit when:**
- Sustained high throughput where always-on nodes stay utilized
- Workloads need persistent connections (gRPC, WebSocket, DB pools)
- Processing exceeds edge resource limits (CPU, memory, duration)
- Full observability and debugging toolchain is required
- Complex microservice graphs with service-to-service communication
- Compliance requirements demand infrastructure control

### Hybrid Approach

A common production pattern: edge workers handle routing, auth, caching, and request validation, then proxy to Kubernetes-hosted services for heavy business logic. This captures edge latency benefits for the "front door" while retaining Kubernetes flexibility for the backend.

---

## Sources

- [Cloudflare: Eliminating Cold Starts](https://blog.cloudflare.com/eliminating-cold-starts-with-cloudflare-workers/)
- [Cloudflare: Serverless Performance Comparison](https://blog.cloudflare.com/serverless-performance-comparison-workers-lambda/)
- [Cloudflare Workers Limits](https://developers.cloudflare.com/workers/platform/limits/)
- [Native vs WASM Benchmark (Go/Python/Rust)](https://karnwong.me/posts/2024/12/native-implementation-vs-wasm-for-go-python-and-rust-benchmark/)
- [Grafbase: Federation Gateway Benchmark (Sep 2025)](https://grafbase.com/blog/benchmarking-graphql-federation-gateways)
- [Hive: Federation Gateway Performance](https://the-guild.dev/graphql/hive/federation-gateway-performance)
- [GitHub: graphql-gateways-benchmark](https://github.com/graphql-hive/graphql-gateways-benchmark)
- [Cosmo Router Architecture](https://wundergraph.com/blog/cosmo_router_high_performance_federation_v1_v2_router_gateway)
- [Grafbase WASM Hooks](https://grafbase.com/blog/introducing-grafbase-gateway-webassembly-hooks)
- [WASM: TinyGo vs Rust vs AssemblyScript](https://ecostack.dev/posts/wasm-tinygo-vs-rust-vs-assemblyscript/)
- [State of WebAssembly 2025-2026](https://platform.uno/blog/the-state-of-webassembly-2025-2026/)
- [2ality: WebAssembly Language Ecosystem](https://2ality.com/2025/01/webassembly-language-ecosystem.html)
- [Server-Side WASM Guide 2025](https://toolshelf.tech/blog/server-side-webassembly-wasm-guide-2025/)
- [Reality Check: Cloudflare WASM Workers + Rust](https://nickb.dev/blog/reality-check-for-cloudflare-wasm-workers-and-rust/)
- [Edge Functions vs Serverless 2025](https://byteiota.com/edge-functions-vs-serverless-the-2025-performance-battle/)
- [Rust, WASM, and Edge Performance](https://dzone.com/articles/rust-wasm-and-edge-next-level-performance)
