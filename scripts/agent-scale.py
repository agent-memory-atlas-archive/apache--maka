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
parser.add_argument("--agents", type=int, nargs="+", default=[10, 100, 1000])
parser.add_argument("--repeat", type=int, default=5)
parser.add_argument("--deltas", type=int, default=32)
parser.add_argument("--output", type=Path, default=Path("/tmp/maka-agent-scale.json"))
args = parser.parse_args()
if args.repeat < 1 or args.deltas < 1 or min(args.agents) < 1:
    parser.error("all workload sizes and repetitions must be positive")
report = {
    "recorded_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "machine": platform.platform(), "logical_cpus": os.cpu_count(),
    "load_average": os.getloadavg(),
    "binary": str(args.binary.resolve()), "runs": [],
    "binary_sha256": hashlib.file_digest(args.binary.open("rb"), "sha256").hexdigest(),
    "workload": "real Host/native owner/shared JS Chat SDK; all HTTP held until every agent arrives; 2ms paced SSE; durable output verified",
    "scope": "includes in-process local HTTP server; excludes TUI rendering and external model latency",
    "cpu_scope": "model admission through stream settlement; memory is process peak across the entire run",
}
for repetition in range(args.repeat):
    for agents in args.agents:
        env = {**os.environ, "MAKA_SCALE_AGENTS": str(agents), "MAKA_SCALE_DELTAS": str(args.deltas)}
        command = [str(args.binary.resolve()), "owner::tests::scale::real_host_agent_scale", "--ignored", "--exact", "--nocapture", "--test-threads=1"]
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
        row["repetition"] = repetition + 1
        report["runs"].append(row)
        args.output.write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(row), flush=True)
        if row["errors"]:
            raise SystemExit(f"benchmark failed; evidence: {args.output}")
report["summary"] = {}
for agents in args.agents:
    rows = [r for r in report["runs"] if r["agents"] == agents]
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
    report["summary"][str(agents)] = summary
args.output.write_text(json.dumps(report, indent=2) + "\n")
print(f"Evidence: {args.output}")
