#!/usr/bin/env python3
import os
import os.path
import subprocess
import json
import shutil
from pathlib import Path

here = Path(__file__).parent

stage_dir = here / "stage"
static_dir = stage_dir / "static"

# Build wasm
bindgen_dir = stage_dir / "bindgen"
for d in [stage_dir, static_dir, bindgen_dir]:
    if d.exists():
        shutil.rmtree(d)
    d.mkdir(exist_ok=True, parents=True)
wasm_proc = subprocess.run(
    [
        "cargo",
        "build",
        "--message-format=json",
        "--target=wasm32-unknown-unknown",
        "--release",
        # "-Zbuild-std=std,panic_abort",
    ],
    cwd=here / "rust/web",
    stdout=subprocess.PIPE,
)
failed = False
for line_raw in wasm_proc.stdout.decode("utf-8").splitlines():
    line = json.loads(line_raw)
    message = line.get("message") or {}
    if message.get("level") == "error":
        print(message.get("rendered"))
        failed = True
    e = line.get("executable")
    if e is not None:
        executable = e
if failed:
    raise RuntimeError("Encountered errors building wasm")
subprocess.check_call(
    [
        "wasm-bindgen",
        executable,
        "--out-dir={}".format(bindgen_dir),
        "--target=web",
        "--split-linked-modules",
        "--keep-debug",
    ],
)
web_static_dir = here / "rust/web/dist"
for source in web_static_dir.iterdir():
    dest = (static_dir / (source.relative_to(web_static_dir))).resolve()
    dest.parent.mkdir(exist_ok=True, parents=True)
    shutil.copy(source, dest)
for f in [
    "web_bg.wasm",
    "web.js",
]:
    source = bindgen_dir / f
    dest = static_dir / f
    dest.parent.mkdir(exist_ok=True, parents=True)
    shutil.copy(source, dest)

# Build server
subprocess.check_output(["cargo", "build"], cwd=here / "rust/native")
