# MuConDe-controller

<p align="center">
  <a href="https://hook.lma1980.duckdns.org/jobs">
    <img src="https://hook.lma1980.duckdns.org/cards/LMA1980/MuConDe-controller/card.svg" alt="MuConDe-controller Factory Scorecard" width="520"/>
  </a>
</p>

<p align="center">
  <a href="https://hook.lma1980.duckdns.org/jobs"><img src="https://hook.lma1980.duckdns.org/badges/LMA1980/MuConDe-controller/status.svg" alt="CI Status"/></a>
  <a href="https://hook.lma1980.duckdns.org/jobs"><img src="https://hook.lma1980.duckdns.org/badges/LMA1980/MuConDe-controller/score.svg" alt="Quality Score"/></a>
  <a href="https://hook.lma1980.duckdns.org/jobs"><img src="https://hook.lma1980.duckdns.org/badges/LMA1980/MuConDe-controller/coverage.svg" alt="Coverage"/></a>
  <a href="https://hook.lma1980.duckdns.org/jobs"><img src="https://hook.lma1980.duckdns.org/badges/LMA1980/MuConDe-controller/benchmark.svg" alt="Benchmark"/></a>
</p>

---

Continuous Delivery (CD) control plane orchestrator managing zero-downtime, safe, autonomous deployment across cluster worker nodes over `MuConDe-ipc` and `rustls` mTLS (`step-ca` PKI).

`MuConDe-controller` coordinates with `ActionRunner-controller` (`GET https://controller.ci.internal:8443/api/cluster/nodes`), pulls verified artifacts from `MuRegist` (`https://registry.ci.internal:8444`), and drives node-level `MuConDe-agent` daemons over `MuConDe-ipc`.

---

## Core Responsibilities

- **Idle Runner Discovery**: Inspects target host worker utilization and pending queue depths (`active_slots == 0 && !exclusive_locked`) to schedule deployments during verified idle windows without interrupting active factory pipelines.
- **TTL-Bounded Deployment Leases**: Negotiates time-to-live leases (e.g. 300s) on target nodes. If lease renewal fails or verification stalls, the target node autonomously rolls back.
- **Declarative IT Failure Contracts (`.muconde/MuConDe.yml`)**:
  - Parses deployment requirements: target node, target slot, probe endpoints, and required critical Integration Test suites (`integration_tests: [...]` or `"all"`).
- **Atomic Promotion / Rollback State Machine**:
  - Stages candidate release in inactive slot (Slot B).
  - Swaps active execution pointer to candidate slot.
  - Executes live integration tests and health probes against Slot B.
  - **Success Path**: Dispatches atomic promotion trigger to swap Slot B to baseline Slot A and marks release healthy in `MuRegist` (`POST /api/v1/canary/{digest}/promote`).
  - **Failure Path**: Dispatches immediate rollback trigger to restore baseline Slot A, sweeps lease, and marks artifact digest as tainted in `MuRegist` (`POST /api/v1/canary/{digest}/taint`).

---

## Deployment State Machine

```text
[ Idle ]
   │
   ▼
[ Discover Idle Node ] ──(Node Busy)──> [ Wait / Backoff ]
   │
   ▼
[ Acquire TTL Lease ]
   │
   ▼
[ Stage Deployment (Slot B) ]
   │
   ▼
[ Slot Swap to Slot B ]
   │
   ▼
[ Execute Health Probes & ITs ]
   ├── All Passed ──> [ Promote Slot B ➔ Slot A ] ──> [ MuRegist Promote ] ──> [ Complete ]
   └── Probe Fail / Timeout ──> [ Rollback to Slot A ] ──> [ MuRegist Taint Digest ]
```

---

## Declarative Contract (`.muconde/MuConDe.yml`)

```yaml
version: "1"
service: "action-runner"
target_nodes:
  - "worker-riscv64"
lease_ttl_seconds: 300
warmup_timeout_seconds: 30
integration_tests: "all"
```

Or target specific integration test suites:

```yaml
version: "1"
service: "action-runner"
target_nodes:
  - "worker-riscv64"
lease_ttl_seconds: 300
warmup_timeout_seconds: 30
integration_tests:
  - "it_smoke"
  - "it_mtls_handshake"
```

---

## License

Licensed under the GNU Affero General Public License v3.0 (AGPL-3.0-only). See [LICENSE](LICENSE) for details.
