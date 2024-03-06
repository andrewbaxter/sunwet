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
        e_stem = e.split("/")[-1].split(".")[0]
        print("Executable {}".format(e_stem))
        subprocess.check_call(
            [
                "wasm-bindgen",
                e,
                "--out-dir={}".format(bindgen_dir),
                "--target=web",
                "--split-linked-modules",
                "--keep-debug",
            ],
        )
        for f in [
            "{}_bg.wasm".format(e_stem),
            "{}.js".format(e_stem),
        ]:
            source = bindgen_dir / f
            dest = static_dir / f
            dest.parent.mkdir(exist_ok=True, parents=True)
            shutil.copy(source, dest)
if failed:
    raise RuntimeError("Encountered errors building wasm")
web_static_dir = here / "rust/web/static"
for source in web_static_dir.iterdir():
    dest = (static_dir / (source.relative_to(web_static_dir))).resolve()
    dest.parent.mkdir(exist_ok=True, parents=True)
    shutil.copy(source, dest)

# Build server
subprocess.check_output(["cargo", "build"], cwd=here / "rust/native")
