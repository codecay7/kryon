# Kryon

> A Rust-native distributed key-value store for exploring storage, concurrency, persistence, and fault tolerance.

**Kryon** is an experimental key-value database written in Rust, built from the ground up to study and implement the core systems behind modern storage and distributed databases.

The project starts with a simple in-memory storage engine and progressively evolves toward a distributed system with persistence, sharding, replication, failure recovery, and consensus.

> **Status:** Early development — currently establishing the storage-engine foundation.

---

## Why Kryon?

Kryon is not intended to be a production replacement for Redis, etcd, or other established databases.

The goal is to understand **how these systems work internally** by implementing their fundamental mechanisms from scratch and measuring their behavior experimentally.

The project focuses on questions such as:

* How should an in-memory key-value store be structured?
* How can concurrent clients safely access shared state?
* How should data survive process crashes?
* How do write-ahead logs and snapshots work?
* How can data be partitioned across nodes?
* How should replicas synchronize?
* What happens when a node crashes or the network fails?
* How does a consensus algorithm such as Raft maintain correctness?

---

## Project Goals

Kryon's development is organized around progressively increasing system complexity:

```text
In-Memory Store
       │
       ▼
Command System
       │
       ▼
TCP Server
       │
       ▼
Concurrent Server
       │
       ▼
TTL + Expiration
       │
       ▼
Persistence
       │
       ▼
Storage Engine
       │
       ▼
Sharding
       │
       ▼
Replication
       │
       ▼
Failure Recovery
       │
       ▼
Raft Consensus
       │
       ▼
Distributed Kryon
```

The implementation will prioritize **correctness before performance, and measurement before optimization**.

---

## Planned Features

### Core Storage

* [x] Rust project foundation
* [ ] In-memory key-value store
* [ ] `SET`
* [ ] `GET`
* [ ] `DELETE`
* [ ] `EXISTS`
* [ ] `CLEAR`
* [ ] Structured error handling
* [ ] Unit and integration tests

### Server

* [ ] TCP server
* [ ] Client connections
* [ ] Command parser
* [ ] Request/response protocol
* [ ] Concurrent clients
* [ ] Connection limits
* [ ] Request limits

### Data Management

* [ ] TTL / key expiration
* [ ] Lists
* [ ] Hashes
* [ ] Sets
* [ ] Sorted sets
* [ ] Memory management
* [ ] Eviction policies

### Persistence

* [ ] Write-ahead log
* [ ] Append-only log
* [ ] Crash recovery
* [ ] Snapshots
* [ ] Log replay
* [ ] Corruption handling

### Storage Engine

* [ ] MemTable
* [ ] SSTables
* [ ] Compaction
* [ ] Tombstones
* [ ] Bloom filters
* [ ] Read/write/space amplification analysis

### Distributed System

* [ ] Node identity
* [ ] Node discovery
* [ ] Health checks
* [ ] Sharding
* [ ] Consistent hashing
* [ ] Data rebalancing
* [ ] Primary/replica replication
* [ ] Replica recovery
* [ ] Failure detection
* [ ] Automatic failover

### Consensus

* [ ] Raft leader election
* [ ] Terms
* [ ] Log replication
* [ ] Commit index
* [ ] Quorum
* [ ] Leader failure recovery
* [ ] Split-brain testing

### Observability

* [ ] Structured logging
* [ ] Runtime statistics
* [ ] Metrics
* [ ] HTTP administration API
* [ ] Health endpoint
* [ ] Cluster status
* [ ] Prometheus-compatible metrics

### Engineering

* [ ] Property-based testing
* [ ] Fuzz testing
* [ ] Failure injection
* [ ] Benchmark suite
* [ ] CI/CD
* [ ] Docker deployment
* [ ] Configuration system
* [ ] Rust client
* [ ] Documentation and architecture decision records

---

## Architecture

Kryon is designed as a layered system so that each subsystem can be implemented, tested, and benchmarked independently.

```text
                    ┌─────────────────────┐
                    │      Clients        │
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │   Protocol / API    │
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │ Command Processing  │
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │    Storage Layer    │
                    └──────────┬──────────┘
                               │
                 ┌─────────────┼─────────────┐
                 ▼             ▼             ▼
              MemTable        WAL         SSTables
                 │                           │
                 └─────────────┬─────────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │  Cluster / Sharding │
                    └──────────┬──────────┘
                               │
                 ┌─────────────┼─────────────┐
                 ▼             ▼             ▼
               Node 1        Node 2        Node 3
                 │             │             │
                 └─────────────┼─────────────┘
                               │
                               ▼
                       Replication / Raft
```

This architecture will evolve as the project progresses.

---

## Technology

| Component     | Technology                                  |
| ------------- | ------------------------------------------- |
| Language      | Rust                                        |
| Build system  | Cargo                                       |
| Networking    | TCP                                         |
| Concurrency   | Rust threads / async Rust                   |
| Storage       | Custom storage engine                       |
| Persistence   | WAL / snapshots                             |
| Consensus     | Raft                                        |
| Testing       | Rust test framework + property/fuzz testing |
| Observability | Metrics + structured logging                |
| Deployment    | Linux / Docker                              |

---

## Development Philosophy

Kryon follows a few core principles:

### 1. Correctness first

A fast database that loses data or violates its consistency guarantees is not a successful database.

### 2. Build from the inside out

The storage engine comes before networking.

Networking comes before distribution.

Distribution comes before consensus.

### 3. Measure before optimizing

Performance claims will be supported by benchmarks rather than assumptions.

Metrics will include:

* Throughput
* p50 latency
* p95 latency
* p99 latency
* CPU utilization
* Memory usage
* Disk usage
* Network overhead
* Recovery time

### 4. Define failure behavior

Distributed systems are defined as much by their behavior during failures as by their behavior during normal operation.

Kryon will explicitly test:

* Node crashes
* Process restarts
* Network delays
* Network disconnections
* Packet loss
* Replication lag
* Leader failures
* Recovery

---

## Research Direction

Kryon is also intended to serve as an experimental platform for distributed-systems research.

Example research question:

> **How does replication factor affect latency, throughput, storage overhead, and recovery time in a distributed key-value store?**

Possible experiments:

```text
Replication Factor
       │
       ├── 1 replica
       ├── 2 replicas
       └── 3 replicas
              │
              ▼
        Measure:
        - latency
        - throughput
        - storage overhead
        - recovery time
        - availability
```

The objective is not merely to implement features, but to understand the **trade-offs between them**.

---

## Project Structure

The structure will evolve as Kryon grows. The intended organization is approximately:

```text
kryon/
├── Cargo.toml
├── Cargo.lock
├── LICENSE
├── README.md
├── src/
│   ├── main.rs
│   ├── storage/
│   ├── protocol/
│   ├── server/
│   ├── persistence/
│   ├── cluster/
│   ├── replication/
│   └── consensus/
├── tests/
├── benches/
└── docs/
    ├── architecture/
    ├── protocol/
    ├── research/
    └── adr/
```

---

## Building

Kryon currently requires a Rust toolchain with the 2024 edition.

Clone the repository:

```bash
git clone git@github.com:codecay7/kryon.git
cd kryon
```

Build:

```bash
cargo build
```

Check the project:

```bash
cargo check
```

Run tests:

```bash
cargo test
```

Run Clippy:

```bash
cargo clippy
```

Format the code:

```bash
cargo fmt
```

---

## Development Roadmap

Kryon will be developed incrementally.

### Phase 1 — Storage Foundation

Build a correct in-memory key-value store.

### Phase 2 — Command System

Introduce a typed command model and parser.

### Phase 3 — Networking

Expose Kryon through a TCP server.

### Phase 4 — Concurrency

Support multiple clients safely and efficiently.

### Phase 5 — Expiration

Add TTL and expiration semantics.

### Phase 6 — Persistence

Introduce WAL/AOF-style persistence and crash recovery.

### Phase 7 — Storage Engine

Move beyond a simple `HashMap` toward an LSM-inspired architecture.

### Phase 8 — Benchmarking

Build reproducible workloads and compare system behavior.

### Phase 9 — Distribution

Introduce nodes, membership, sharding, and routing.

### Phase 10 — Replication

Implement primary/replica replication and recovery.

### Phase 11 — Failure Handling

Introduce controlled failure injection and recovery testing.

### Phase 12 — Consensus

Implement Raft for coordinated distributed state.

---

## Status

Kryon is currently in **early development**.

The immediate objective is:

```text
Rust
 ↓
Store
 ↓
Tests
 ↓
Command abstraction
 ↓
TCP
```

More advanced distributed functionality will only be introduced after the underlying components are correct and measurable.

---

## License

Kryon is licensed under the **Apache License 2.0**.

See [`LICENSE`](LICENSE) for the full license text.

---

## Author

**codecay7**

GitHub: `https://github.com/codecay7/kryon`
