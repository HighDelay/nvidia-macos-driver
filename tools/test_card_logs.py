#!/usr/bin/env python3
"""Exercise the production log collector and redactor with isolated fixtures."""
import os
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PCI = '''+-o display@0
  "vendor-id" = <de100000>
  "device-id" = <82280000>
  "subsystem-vendor-id" = <de100000>
  "subsystem-id" = <00000000>
  "assigned-addresses" = <fixture-bar-data>
  "nvrm-boot-hold" = "display armed"
+-o unrelated@1
  "vendor-id" = <02100000>
  "device-id" = <ffff0000>
'''
REDACTION = r'''
import Foundation
func sh(_ path: String, _ args: [String]) -> String { return "" }
func services(_ name: String) -> [Int] { return [] }
func str(_ value: Int, _ key: String) -> String? { return nil }
func IOObjectRelease(_ value: Int) {}
PRODUCTION
let path = URL(fileURLWithPath: "/Users").appendingPathComponent("fixture").appendingPathComponent("test.log").path
let sample = path + " GPU 10de:2882 BAR1 0x10000000000"
let result = redact(sample)
assert(result == "[home]/test.log GPU 10de:2882 BAR1 0x10000000000")
assert(redact(NSHomeDirectory() + "/test.log") == "[home]/test.log")
print("PASS home paths removed while PCI IDs and BAR addresses remain")
'''
with tempfile.TemporaryDirectory(prefix="nullmoth-card-log-tests-") as directory:
    tmp = Path(directory)
    commands = tmp / "bin"
    commands.mkdir()
    mocks = {
        "ioreg": "cat <<'PCI'\n" + PCI + "PCI\n",
        "log": "echo 'fixture log query unavailable'; exit 3\n",
        "dmesg": "echo 'NVRM: GSP ring fixture'\n",
        "system_profiler": "echo 'Chipset Model: NVIDIA fixture'\n",
        "sw_vers": "echo 15.8.1\n",
        "sysctl": "echo 'debug.nvaccelfb: 1'\n",
        "kmutil": "echo 'com.nullmoth.NVRM'\n",
        "nvram": "echo 'boot-args fixture'\n",
        "csrutil": "echo 'fixture'\n",
        "stat": "echo root\n",
        "ls": "exit 0\n",
    }
    for name, body in mocks.items():
        path = commands / name
        path.write_text("#!/bin/bash\n" + body)
        path.chmod(0o755)
    source = (ROOT / "app/Resources/nullmoth-setup.sh").read_text()
    start = '  { echo "macOS $(sw_vers -productVersion)'
    stop = '  for f in $(ls -t /Library/Logs/DiagnosticReports/'
    assert source.count(start) == source.count(stop) == 1
    body = start + source.split(start, 1)[1].split(stop, 1)[0]
    # Redirect the optional file-log source into the isolated fixture directory.
    assert body.count('/private/tmp/nvmtl.log') == 1
    filelog = tmp / "nvmtl.log"
    filelog.write_text("NVMTL: vkCreateDevice -> -8\n")
    body = body.replace('/private/tmp/nvmtl.log', str(filelog))
    collection = tmp / "logs"
    collection.mkdir()
    script = tmp / "collect.sh"
    script.write_text('COLLECT="$1"\n' + body)
    env = dict(os.environ, PATH=str(commands) + ":" + os.environ["PATH"])
    subprocess.run(["/bin/bash", str(script), str(collection)], env=env, check=True)
    state = (collection / "driver-state.txt").read_text()
    for needle in ('"device-id" = <82280000>', 'fixture-bar-data', '"nvrm-boot-hold" = "display armed"'):
        assert needle in state
    assert 'unrelated@1' not in state and 'ffff0000' not in state
    kernel = (collection / "driver-kernel-log.txt").read_text()
    plugin = (collection / "driver-plugin-log.txt").read_text()
    assert "log show exit: 3" in kernel and "GSP ring fixture" in kernel
    assert "log show exit: 3" in plugin and "vkCreateDevice -> -8" in plugin
    print("PASS NVIDIA identifiers, BARs, query failures, kernel ring, and plugin errors retained")
    swift = (ROOT / "app/Sources/main.swift").read_text()
    start, stop = 'func redact(_ s: String) -> String {', 'func hardwareFacts() -> String {'
    assert swift.count(start) == swift.count(stop) == 1
    redact = start + swift.split(start, 1)[1].split(stop, 1)[0]
    fixture = tmp / "redact.swift"
    fixture.write_text(REDACTION.replace("PRODUCTION", redact))
    subprocess.run(["xcrun", "swift", str(fixture)], check=True)
print("PASS card-log collection regressions")
