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
"""Measure real TUI startup in a 120x40 PTY (macOS/Linux, standard library).

Example: benchmark-startup.py --binary target/debug/maka --baseline /tmp/maka-old \
    --runs 5 --output /tmp/maka-startup-evidence
Installation is measured separately; reopen rounds alternate baseline/candidate.
Readiness requires Settings + New session, then Ctrl-K showing Close commands.
Evidence retains isolated installation roots, raw ANSI, and results.json. No sampling.
"""
import argparse
import errno
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import statistics
import struct
import subprocess
import tempfile
import termios
import time

ANSI = re.compile(rb'\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07]*?(?:\x07|\x1b\\)')
QUERIES = [(b'\x1b[6n', b'\x1b[1;1R'), (b'\x1b[c', b'\x1b[?1;2c'),
           (b'\x1b[>c', b'\x1b[>0;0;0c'),
           (b'\x1b]11;?', b'\x1b]11;rgb:1010/1010/1010\x1b\\')]


def terminal_owner():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


def stop(process):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            break
        try:
            process.wait(timeout=3)
            break
        except subprocess.TimeoutExpired:
            continue
    process.wait()


def cleanup(case):
    # Public Hosts may detach into a new session. Only target our unique roots.
    roots = {str(case / 'host'), str(case / 'ui')}
    listing = subprocess.run(['ps', '-axo', 'pid=,command='], capture_output=True, text=True)
    for line in listing.stdout.splitlines():
        try:
            pid, command = line.strip().split(None, 1)
            # ps renders argv without shell quoting, including paths with spaces.
            owns_root = any(re.search(r'(?<!\S)--root ' + re.escape(root) + r'(?: |$)', command) for root in roots)
            if owns_root and int(pid) != os.getpid():
                os.kill(int(pid), signal.SIGTERM)
        except (ValueError, OSError):
            continue


def measure(binary, case, name, installation, timeout):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 120, 0, 0))
    args = [str(binary), 'application', 'launch', '--root', str(case / 'ui')]
    if installation:
        args += ['--configuration', str(case / 'installation.json')]
    env = dict(os.environ, TERM='xterm-256color', COLORTERM='truecolor', LC_ALL='en_US.UTF-8')
    env.pop('NO_COLOR', None)
    output, queries = bytearray(), set()
    result = dict(run=name, first_install=installation, ready_s=None, interactive_s=None,
                  exit_code=None, error=None, raw_ansi=str(case / f'{name}.ansi'))
    process = None
    started = time.monotonic()
    try:
        process = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave,
                                   cwd=case, env=env, preexec_fn=terminal_owner)
        os.close(slave)
        slave = None
        while time.monotonic() - started < timeout:
            readable, _, _ = select.select([master], [], [], 0.05)
            if readable:
                try:
                    chunk = os.read(master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not chunk:
                    break
                output.extend(chunk)
                for pattern, reply in QUERIES:
                    for match in re.finditer(re.escape(pattern), output):
                        key = (pattern, match.start())
                        if key not in queries:
                            os.write(master, reply)
                            queries.add(key)
                clean = ANSI.sub(b'', output)
                if result['ready_s'] is None and b'Settings' in clean and b'New session' in clean:
                    result['ready_s'] = time.monotonic() - started
                    os.write(master, b'\x0b')
                if result['ready_s'] is not None and result['interactive_s'] is None and b'Close commands' in clean:
                    result['interactive_s'] = time.monotonic() - started
                    os.write(master, b'\x1b\x11')
            if process.poll() is not None and not readable:
                break
        if process.poll() is None:
            process.wait(timeout=max(0.01, timeout - (time.monotonic() - started)))
        if result['interactive_s'] is None:
            result['error'] = 'exited before command palette became usable'
        elif process.returncode != 0:
            result['error'] = 'nonzero exit after Ctrl-Q'
    except (OSError, subprocess.TimeoutExpired) as error:
        result['error'] = f'launch/exit deadline exceeded ({timeout}s)' if isinstance(error, subprocess.TimeoutExpired) else f'{type(error).__name__}: {error}'
    finally:
        if slave is not None:
            os.close(slave)
        if process is not None:
            if process.poll() is None:
                stop(process)
            result['exit_code'] = process.returncode
        os.close(master)
        Path(result['raw_ansi']).write_bytes(output)
        result['total_s'] = time.monotonic() - started
    if result['error'] is not None:
        cleanup(case)
    print(json.dumps(result), flush=True)
    return result


def summarize(rows):
    successful = [row for row in rows if row['error'] is None]
    result = dict(runs=len(rows), failures=len(rows) - len(successful))
    for field in ('ready_s', 'interactive_s'):
        values = [row[field] for row in successful]
        result[field] = dict(median=statistics.median(values), min=min(values), max=max(values)) if values else None
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--binary', required=True, type=Path, help='candidate CLI executable')
    parser.add_argument('--baseline', type=Path, help='optional baseline CLI executable')
    parser.add_argument('--runs', type=int, default=5, help='reopen runs per binary (default: 5)')
    parser.add_argument('--output', required=True, type=Path, help='evidence directory; roots are always new')
    parser.add_argument('--timeout', type=float, default=60, help='seconds allowed per launch including exit')
    options = parser.parse_args()
    if options.runs < 1 or options.timeout <= 0:
        parser.error('--runs and --timeout must be positive')
    binaries = {'baseline': options.baseline, 'candidate': options.binary}
    binaries = {label: path.resolve() for label, path in binaries.items() if path is not None}
    for path in binaries.values():
        if not path.is_file() or not os.access(path, os.X_OK):
            parser.error(f'not an executable: {path}')
    options.output.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix='startup-', dir=options.output.resolve()))
    cases, results = {}, dict(binaries={key: str(path) for key, path in binaries.items()},
                              terminal={'columns': 120, 'rows': 40}, installation={}, reopens={}, summary={})
    try:
        for label, binary in binaries.items():
            case = evidence / label
            case.mkdir()
            (case / 'workspace').mkdir()
            cases[label] = case
            configuration = dict(presentation='tui', locale='en', scheme='dark', hostRoot=str(case / 'host'),
                                 agent={'workspaceRoots': [{'label': 'Workspace', 'path': str(case / 'workspace')}],
                                        'tools': [], 'permissions': {'access': 'read_only', 'requestPermissions': True}},
                                 skills={'bundled': True, 'sources': []})
            (case / 'installation.json').write_text(json.dumps(configuration))
            results['installation'][label] = measure(binary, case, 'installation', True, options.timeout)
            results['reopens'][label] = []
        for run in range(options.runs):
            for label, binary in binaries.items():
                if results['installation'][label]['error'] is None:
                    results['reopens'][label].append(measure(binary, cases[label], f'reopen-{run + 1}', False, options.timeout))
    finally:
        for label, case in cases.items():
            cleanup(case)
            results['summary'][label] = summarize(results['reopens'].get(label, []))
        results['failures'] = sum(bool(row['error']) for row in results['installation'].values()) + sum(item['failures'] for item in results['summary'].values())
        (evidence / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        print(f'Evidence: {evidence / "results.json"}', flush=True)
    return int(results['failures'] > 0)


if __name__ == '__main__':
    raise SystemExit(main())
