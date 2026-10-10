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

"""Isolated Unix launcher smoke test for the generated Host Echo service."""
import argparse
import json
import pathlib
import select
import socket
import struct
import subprocess
import tempfile
import uuid


def write_frame(stream, value):
    data = json.dumps(value).encode()
    remaining = memoryview(struct.pack(">I", len(data)) + data)
    while remaining:
        written = stream.write(remaining)
        if not written:
            raise EOFError("Host channel closed while writing")
        remaining = remaining[written:]
    stream.flush()


def read_frame(stream):
    def exact(size):
        data = bytearray()
        while len(data) < size:
            if not select.select([stream], [], [], 15)[0]:
                raise TimeoutError("Host frame timed out")
            chunk = stream.read(size - len(data))
            if not chunk:
                raise EOFError("Host closed before completing a frame")
            data.extend(chunk)
        return data
    size = struct.unpack(">I", exact(4))[0]
    if size > 8 * 1024 * 1024:
        raise ValueError("Host frame exceeds test budget")
    return json.loads(exact(size))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--maka", default="maka")
    args = parser.parse_args()
    if not hasattr(socket, "AF_UNIX"):
        parser.error("This fixture requires Unix sockets (macOS/Linux).")
    starter = pathlib.Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix="maka-echo-", dir="/tmp") as temporary:
        base = pathlib.Path(temporary)
        def run(*command):
            return json.loads(subprocess.check_output([args.maka, *map(str, command)], timeout=30))
        def save(path, value):
            path.write_text(json.dumps(value))
        subprocess.run(["node", str(starter / "create.mjs"), "--maka", args.maka,
                        "--target", "host", "--output", str(base / "example")],
                       check=True, capture_output=True, timeout=30)
        project = base / "example"
        manifest = json.loads((project / "plugin/maka.extension.json").read_text())
        contract = manifest["application"]["entries"]["host"]["exports"]["Echo"]["contract"]
        consumer = base / "consumer"
        consumer.mkdir()
        (consumer / "consumer.mjs").write_text("export function activate() { return {}; }")
        save(consumer / "maka.extension.json", {
            "schemaVersion": 2, "id": "example.echo-test",
            "application": {"entries": {"host": {
                "implementation": {"kind": "javascript", "module": "consumer.mjs", "realm": "dedicated"},
                "configuration": {"type": "object"}, "exports": {},
                "imports": {"echo": {"contract": contract, "cardinality": {"kind": "one"}}},
            }}},
        })
        composition = json.loads((project / "composition.json").read_text())
        key = str(uuid.uuid4())
        composition["instances"]["consumer"] = {
            "key": key, "package": {"source": "pinned", "package": run("application", "inspect-package", "--package", consumer)},
            "usePackage": "entry", "bindings": {"echo": [{"kind": "instance", "path": ["example"], "export": "Echo"}]},
        }
        save(project / "composition.json", composition)
        root = base / "root"
        run("host", "init", "--root", root)
        source = run("application", "commit", "--root", root, "--deployment", str(uuid.uuid4()),
                     "--expected-revision", "0", "--input", project / "composition.json",
                     "--package", project / "plugin", "--package", consumer)
        nonce = str(uuid.uuid4())
        address = str(base / "host.sock")
        bootstrap = {
            "root": str(root), "deployment": source["deployment"],
            "namespaces": {"ownership": str(base / "owners"), "control": str(base / "control")},
            "source": source, "consumers": [[key]], "native": [], "address": address, "nonce": nonce,
            "endpoints": {"echo": {"consumer": [key], "import": "echo", "grant": {"methods": ["echo"], "recovery": False}}},
        }
        # The launcher owns bootstrap/control and grants only this declared query.
        # Plugin business code uses its bound capability, not this wire protocol.
        with subprocess.Popen([args.maka, "application", "serve"], stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, bufsize=0) as host:
            try:
                write_frame(host.stdin, bootstrap)
                ready = read_frame(host.stdout)
                assert ready["kind"] == "ready", ready
                epoch = ready["catalogue"]["endpoints"]["echo"]["registration_epoch"]
                for value, valid in [("hello Maka", True), ("😀" * 4096, True), (42, False), ("x" * 4097, False)]:
                    with socket.socket(socket.AF_UNIX) as connection:
                        connection.settimeout(15)
                        connection.connect(address)
                        with connection.makefile("rwb", buffering=0) as channel:
                            write_frame(channel, {"nonce": nonce})
                            read_frame(channel)
                            write_frame(channel, {
                                "endpoint": "echo", "registration_epoch": epoch, "method": "echo", "input": value,
                                "source": {"deployment": source, "consumer": [key], "mount": str(uuid.uuid4()), "request": str(uuid.uuid4())},
                            })
                            write_frame(channel, {})
                            reply = read_frame(channel)
                            if reply["kind"] == "ready":
                                reply = read_frame(channel)
                            assert reply["cleanup"] == {"Ok": None}, reply
                            if valid:
                                assert reply["kind"] == "finished" and reply["result"] == {"Ok": value}, reply
                            else:
                                assert reply["kind"] == "aborted", reply
                write_frame(host.stdin, {"kind": "close", "nonce": nonce})
                assert read_frame(host.stdout) == {"kind": "closed", "nonce": nonce}
                assert host.wait(timeout=15) == 0
            finally:
                if host.poll() is None:
                    host.kill()
                    host.wait(timeout=15)
    print("Host Echo: ASCII/Unicode, invalid inputs and clean shutdown passed.")


if __name__ == "__main__":
    main()
