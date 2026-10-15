#!/usr/bin/env python3
# Licensed to the Apache Software Foundation (ASF) under one
# or more contributor license agreements.  See the NOTICE file
# distributed with this work for additional information
# regarding copyright ownership.  The ASF licenses this file
# to you under the Apache License, Version 2.0 (the
# "License"); you may not use this file except in compliance
# with the License.  You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing,
# software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
# KIND, either express or implied.  See the License for the
# specific language governing permissions and limitations
# under the License.
"""Repeat the Unix release real-Host benchmark, retaining every sample and failure.

Build once: cargo test --release -p maka-agent-native --lib --no-run
Run: python3 scripts/agent-scale.py --binary target/release/deps/maka_agent_native-HASH
The binary must be the test executable printed by Cargo, not a debug build.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--binary", type=Path, required=True)
parser.add_argument("--baseline", type=Path, help="alternate candidate and baseline in the same run")
parser.add_argument("--prompt-bytes", type=int)
parser.add_argument("--delta-bytes", type=int)
parser.add_argument("--agents", type=int, nargs="+", default=[10, 100, 1000])
parser.add_argument("--repeat", type=int, default=5)
parser.add_argument("--deltas", type=int, default=32)
parser.add_argument("--output", type=Path, default=Path("/tmp/maka-agent-scale.json"))
args = parser.parse_args()
if min(args.agents + [args.repeat, args.deltas] + [n for n in (args.prompt_bytes, args.delta_bytes) if n is not None]) < 1:
    parser.error("all workload sizes and repetitions must be positive")
binaries = {"candidate": args.binary.resolve()}
if args.baseline:
    binaries["baseline"] = args.baseline.resolve()
args.output.parent.mkdir(parents=True, exist_ok=True)
report = {
    "recorded_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "machine": platform.platform(), "logical_cpus": os.cpu_count(),
    "load_average": os.getloadavg(),
    "binaries": {name: {"path": str(path), "sha256": hashlib.file_digest(path.open("rb"), "sha256").hexdigest()} for name, path in binaries.items()},
    "runs": [],
    "sizes": {"agents": args.agents, "prompt_bytes": args.prompt_bytes or 5, "delta_bytes": args.delta_bytes or 6, "deltas": args.deltas},
    "workload": "real Host/native owner/shared JS Chat SDK; all HTTP held until every agent arrives; 2ms paced SSE; durable output verified",
    "scope": "includes in-process local HTTP server; excludes TUI rendering and external model latency",
    "cpu_scope": "model admission through stream settlement; memory is process peak across the entire run",
}
for repetition in range(args.repeat):
    for agents in args.agents:
        variants = list(binaries)
        if repetition % 2:
            variants.reverse()
        for variant in variants:
            env = dict(os.environ)
            for name in ("MAKA_SCALE_MEMORY_PROFILE", "MallocStackLogging", "MallocStackLoggingNoCompact", "MAKA_SCALE_PROMPT_BYTES", "MAKA_SCALE_DELTA_BYTES"):
                env.pop(name, None)
            env.update(MAKA_SCALE_AGENTS=str(agents), MAKA_SCALE_DELTAS=str(args.deltas))
            for name, value in (("MAKA_SCALE_PROMPT_BYTES", args.prompt_bytes), ("MAKA_SCALE_DELTA_BYTES", args.delta_bytes)):
                if value is not None:
                    env[name] = str(value)
            command = [str(binaries[variant]), "owner::tests::scale::real_host_agent_scale", "--ignored", "--exact", "--nocapture", "--test-threads=1"]
            started = datetime.datetime.now(datetime.timezone.utc).isoformat()
            load = os.getloadavg()
            try:
                result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=660)
                marker = "MAKA_AGENT_SCALE "
                payloads = [line.split(marker, 1)[1] for line in result.stdout.splitlines() if marker in line]
                row = json.loads(payloads[0]) if result.returncode == 0 and len(payloads) == 1 else {
                    "agents": agents, "errors": 1, "exit_code": result.returncode,
                    "stdout": result.stdout[-16000:], "stderr": result.stderr[-16000:],
                }
            except subprocess.TimeoutExpired:
                row = {"agents": agents, "errors": 1, "timeout_seconds": 660}
            row.update(variant=variant, repetition=repetition + 1, started_at=started, load_average=load)
            report["runs"].append(row)
            args.output.write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps(row), flush=True)
            if row["errors"]:
                raise SystemExit(f"benchmark failed; evidence: {args.output}")
report["summary"] = {}
for variant in binaries:
    for agents in args.agents:
        rows = [r for r in report["runs"] if r["agents"] == agents and r["variant"] == variant]
        summary = {
            key: {"median": statistics.median(r[key] for r in rows), "min": min(r[key] for r in rows), "max": max(r[key] for r in rows)}
            for key in ["all_active_ms", "stream_and_settle_ms", "durable_input_deltas_per_second", "cpu_seconds", "peak_rss_bytes"]
        }
        for key, values in {
            "foreground_query_p99_ms": [r["foreground_query_ms"]["p99"] for r in rows],
            "cancel_and_settle_ms": [r["cancellation"]["cancel_and_settle_ms"] for r in rows],
            "recovery_ms": [r["cancellation"]["recovery_ms"] for r in rows],
        }.items():
            summary[key] = {"median": statistics.median(values), "min": min(values), "max": max(values)}
        report["summary"].setdefault(variant, {})[str(agents)] = summary
args.output.write_text(json.dumps(report, indent=2) + "\n")
print(f"Evidence: {args.output}")
