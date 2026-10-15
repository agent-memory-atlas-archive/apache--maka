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
"""Snapshot real-Host memory at explicit lifecycle boundaries on macOS.

Use the release native test binary from scripts/agent-scale.py. This rendezvous
pauses the load driver, so its timing results are not performance measurements.
--stacks adds allocator tracing in a separate run; never compare its RSS with
an uninstrumented run. Raw snapshots and command failures are retained.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--binary", type=Path, required=True)
parser.add_argument("--agents", type=int, nargs="+", default=[10, 100, 1000])
parser.add_argument("--output", type=Path, default=Path("/tmp/maka-agent-memory"))
parser.add_argument("--stacks", action="store_true")
parser.add_argument("--prompt-bytes", type=int)
parser.add_argument("--delta-bytes", type=int)
parser.add_argument("--deltas", type=int, default=32)
args = parser.parse_args()
if platform.system() != "Darwin" or min(args.agents + [args.deltas] + [n for n in (args.prompt_bytes, args.delta_bytes) if n is not None]) < 1:
    parser.error("macOS and positive agent counts are required")
args.output.mkdir(parents=True, exist_ok=True)
report = {
    "machine": platform.platform(),
    "binary": str(args.binary.resolve()),
    "binary_sha256": hashlib.file_digest(args.binary.open("rb"), "sha256").hexdigest(),
    "allocator_tracing": args.stacks,
    "workload": {"prompt_bytes": args.prompt_bytes or 5, "delta_bytes": args.delta_bytes or 6, "deltas": args.deltas},
    "scope": "whole process, including the local HTTP fixture; lifecycle pauses invalidate timing metrics",
    "runs": [],
}


def record():
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")


for count in args.agents:
    folder = args.output / str(count)
    folder.mkdir(exist_ok=True)
    env = {**os.environ, "MAKA_SCALE_AGENTS": str(count), "MAKA_SCALE_MEMORY_PROFILE": "1"}
    for name, value in (("MAKA_SCALE_PROMPT_BYTES", args.prompt_bytes), ("MAKA_SCALE_DELTA_BYTES", args.delta_bytes), ("MAKA_SCALE_DELTAS", args.deltas)):
        env.pop(name, None)
        if value is not None:
            env[name] = str(value)
    env.pop("MallocStackLogging", None)
    if args.stacks:
        env["MallocStackLogging"] = "1"
    row = {"agents": count, "phases": []}
    report["runs"].append(row)
    with (folder / "stdout.log").open("w") as stdout, (folder / "stderr.log").open("w") as stderr:
        process = subprocess.Popen(
            [str(args.binary.resolve()), "owner::tests::scale::real_host_agent_scale", "--ignored", "--exact", "--nocapture", "--test-threads=1"],
            env=env, stdin=subprocess.PIPE, stdout=stdout, stderr=subprocess.PIPE,
            text=True, bufsize=1,
        )
        try:
            for line in process.stderr:
                stderr.write(line)
                stderr.flush()
                if not line.startswith("MAKA_AGENT_MEMORY "):
                    continue
                phase = line.strip().split()[-1]
                started = time.monotonic()
                sample = {"phase": phase, "commands": []}
                rss = subprocess.check_output(["ps", "-o", "rss=", "-p", str(process.pid)], text=True)
                sample["rss_bytes"] = int(rss.strip()) * 1024
                commands = [
                    ("vmmap", ["vmmap", "-summary", str(process.pid)]),
                    ("heap", ["heap", "-s", "--noContent", str(process.pid)]),
                ]
                if args.stacks and phase in ("all_active", "output_settled", "dropped"):
                    commands.append(("stacks", ["malloc_history", str(process.pid), "-callTree", "-invert", "-ignoreThreads", "-noContent", "-consolidateAllBySymbol"]))
                for name, command in commands:
                    destination = folder / f"{phase}-{name}.txt"
                    with destination.open("w") as output:
                        remaining = 40 - (time.monotonic() - started)
                        if remaining <= 0:
                            sample["commands"].append({"tool": name, "skipped": "checkpoint time budget exhausted"})
                            continue
                        timeout = min(20, remaining)
                        try:
                            result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, timeout=timeout)
                            status = {"tool": name, "exit_code": result.returncode}
                        except subprocess.TimeoutExpired:
                            status = {"tool": name, "timeout_seconds": timeout}
                    sample["commands"].append(status)
                heap_text = (folder / f"{phase}-heap.txt").read_text()
                allocated = re.search(r"All zones: (\d+) nodes \((\d+) bytes\)", heap_text)
                if allocated:
                    sample["malloc_live_blocks"] = int(allocated[1])
                    sample["malloc_live_bytes"] = int(allocated[2])
                footprint = re.search(r"Physical footprint: *([^\n]+)", heap_text)
                if footprint:
                    sample["physical_footprint"] = footprint[1]
                sample["checkpoint_seconds"] = time.monotonic() - started
                row["phases"].append(sample)
                record()
                print(json.dumps({"agents": count, **sample}), flush=True)
                process.stdin.write("x")
                process.stdin.flush()
            row["exit_code"] = process.wait(timeout=30)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)
            process.stdin.close()
    record()
    if not row["phases"]:
        raise SystemExit(f"no checkpoints; rebuild the test binary: {folder}")
    if row["exit_code"]:
        raise SystemExit(f"profiled load failed (inspect workload and checkpoint diagnostics): {folder}")
    if any(command.get("exit_code") != 0 for phase in row["phases"] for command in phase["commands"]):
        raise SystemExit(f"incomplete profiler evidence: {folder}")
print(f"Evidence: {args.output / 'report.json'}")
