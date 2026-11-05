"""Packs the tree of a release of wview: the one workspace a release is built in and its SDK is
(wview/sdk/README.md).

    python wview/sdk/pack.py REPOSITORY OUT "rustc 1.99.1 (58cf8f9c3 2026-10-13)"

OUT is made again; the third argument is what `rustc -V` printed for the release.

Cargo hashes the place of a path dependency into the names of its symbols: relative to the root
of the workspace when the crate is inside it, else the absolute path. In the repository the crates
of W# are outside `wview`, so a plugin built anywhere else would miss the imports of `wwrt.dll`.
In this tree every crate is inside its root, and a release built in it loads every plugin built in
a copy of it, wherever that copy is:

    Cargo.toml, Cargo.lock, .cargo/config.toml, rust-toolchain.toml   the workspace of wview
    framework/ww-base, framework/ww   W#, with the fields of the workspace of WinWeaver written in
    crates/wwrt, crates/wview         the runtime and the program
    plugins/wv-text, wv-image, wv-hello
"""
import re
import shutil
import sys
from pathlib import Path

SKIP = shutil.ignore_patterns("target", "*.rs.bk")


def table(text: str, name: str) -> dict:
    """The keys of the table `[name]` of a manifest with the text of their values, one line each."""
    out, inside = {}, False
    for line in text.splitlines():
        if line.startswith("["):
            inside = line.strip() == f"[{name}]"
            continue
        m = re.match(r"^([A-Za-z0-9_-]+)\s*=\s*(.+)$", line)
        if inside and m:
            out[m.group(1)] = m.group(2).strip()
    if not out:
        raise SystemExit(f"the table [{name}] is not in the manifest of the repository")
    return out


def standalone(manifest: str, package: dict, dependencies: dict) -> str:
    """A manifest of the workspace of WinWeaver without it: `x.workspace = true` written out."""
    def field(m):
        key = m.group(1)
        if key in package:
            return f"{key} = {package[key]}"
        if key in dependencies:
            return f"{key} = {dependencies[key]}"
        raise SystemExit(f"{key}.workspace has no value in the workspace of WinWeaver")
    return re.sub(r"(?m)^([A-Za-z0-9_-]+)\.workspace\s*=\s*true\s*$", field, manifest)


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__)
        return 2
    repo, out, rustc = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
    version = re.search(r"rustc (\d+\.\d+\.\d+)", rustc)
    if not version:
        print(f"the version of rustc is not in {rustc!r}")
        return 1
    if out.exists():
        shutil.rmtree(out)
    root = (repo / "Cargo.toml").read_text(encoding="utf-8")
    package = table(root, "workspace.package")
    dependencies = table(root, "workspace.dependencies")
    for crate in ["ww-base", "ww"]:
        to = out / "framework" / crate
        shutil.copytree(repo / "crates" / crate, to, ignore=SKIP)
        manifest = (to / "Cargo.toml").read_text(encoding="utf-8")
        write(to / "Cargo.toml", standalone(manifest, package, dependencies))
    view = repo / "wview"
    for part in ["crates", "plugins", "stable"]:
        shutil.copytree(view / part, out / part, ignore=SKIP)
    runtime = out / "crates" / "wwrt" / "Cargo.toml"
    text = runtime.read_text(encoding="utf-8")
    text, n = re.subn(r'path = "\.\./\.\./\.\./crates/ww"', 'path = "../../framework/ww"', text)
    if n != 1:
        print("the path of ww in the manifest of wwrt is not the one of the repository")
        return 1
    write(runtime, text)
    manifest = (view / "Cargo.toml").read_text(encoding="utf-8")
    manifest, n = re.subn(
        r"(?m)^members = \[.*\]$",
        'members = ["framework/ww-base", "framework/ww", "crates/*", "plugins/*", "stable/rust"]',
        manifest,
    )
    if n != 1:
        print("the members of the workspace of wview are not on one line")
        return 1
    write(out / "Cargo.toml", manifest)
    shutil.copy(view / "Cargo.lock", out / "Cargo.lock")
    shutil.copytree(view / ".cargo", out / ".cargo")
    write(
        out / "rust-toolchain.toml",
        "# The compiler of the release: a plugin of another one does not load.\n"
        f'[toolchain]\nchannel = "{version.group(1)}"\n',
    )
    for name in ["rustfmt.toml", "LICENSE"]:
        shutil.copy(repo / name, out / name)
    shutil.copy(view / "sdk" / "README.md", out / "README.md")
    print(f"{out}: the tree of wview, rustc {version.group(1)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
