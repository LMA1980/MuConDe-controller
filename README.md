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

Continuous Delivery (CD) control plane orchestrator managing zero-downtime, safe, autonomous deployment across the `lab.local` homelab infrastructure.

`MuConDe-controller` coordinates with `ActionRunner-controller`, pulls verified artifacts from `MuRegist`, and drives node-level `MuConDe-agent` daemons over `MuConDe-ipc`.

---

## Core Responsibilities

- **Idle Runner Discovery**: Inspects target host worker utilization and pending queue depths to schedule deployments during verified idle windows without interrupting active factory pipelines.
- **TTL-Bounded Deployment Leases**: Negotiates time-to-live leases (e.g. 300s) on target nodes. If lease renewal fails or verification stalls, the target node autonomously rolls back.
- **Declarative IT Failure Contracts (`.muconde/MuConDe.yml`)**:
  - Parses deployment requirements: target node, target slot, probe endpoints, and required critical Integration Test suites (`integration_tests: [...]` or `"all"`).
- **Atomic Promotion / Rollback State Machine**:
  - Stages candidate release in inactive slot (Slot B).
  - Executes live integration tests and health probes against Slot B.
  - **Success Path**: Dispatches atomic promotion trigger to swap Slot B to active Slot A.
  - **Failure Path**: Dispatches immediate rollback trigger to revert Slot B snapshot and marks artifact digest as tainted in `MuRegist`.

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
[ Execute Health Probes & ITs ]
   ├── All Passed ──> [ Trigger Promotion (Slot B ➔ Slot A) ] ──> [ Complete ]
   └── Probe Fail / Timeout ──> [ Trigger Atomic Rollback ] ──> [ Taint Digest in MuRegist ]
```

---

## Declarative Contract (`.muconde/MuConDe.yml`)

```yaml
version: "1"
service:
  name: "action-runner"
  target_nodes:
    - "pey-wail-deu"
  slot_strategy: "ab_btrfs"
  lease_ttl_seconds: 300

verification:
  health_probe:
    endpoint: "http://127.0.0.1:8000/health"
    expected_status: 200
    timeout_seconds: 15
  integration_tests:
    selection: "all"
    fail_fast: true

rollback:
  auto_revert_on_probe_failure: true
  taint_registry_digest: true
```

---

## License

Licensed under the GNU Affero General Public License v3.0 (AGPL-3.0-only). See LICENSE for details.
