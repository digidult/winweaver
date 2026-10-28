"""The browser edition of wwlite as one HTML file: the glue of wasm-bindgen inlined as a module
script and the wasm module packed with gzip into base64, unpacked by the browser itself
(DecompressionStream). The file opens from a web server or straight from the disk.

    python single.py FOLDER OUT

FOLDER holds what wasm-bindgen wrote (wwlite.js, wwlite_bg.wasm) and index.html; OUT is the one
file to write (wwlite.html).
"""

import base64
import gzip
import re
import sys
from pathlib import Path

SCRIPT = """  <script type="module">
{glue}
    // The module, packed: gzip in base64, unpacked by the browser and handed to the glue.
    const packed = "{packed}";
    async function start() {{
      const response = await fetch("data:application/gzip;base64," + packed);
      const stream = response.body.pipeThrough(new DecompressionStream("gzip"));
      const bytes = await new Response(stream).arrayBuffer();
      await {init}({{ module_or_path: bytes }});
    }}
    start().catch((e) => {{
      const p = document.getElementById("loading_text");
      if (p) {{ p.textContent = "wwlite did not start: " + e; }}
      console.error(e);
    }});
  </script>"""

COMMENT = """<!-- The browser edition of wwlite in one file: the canvas the window draws in, a text while
       the module starts, the glue of wasm-bindgen and the module itself, packed. It opens from a
       web server or from the disk (a double click); nothing else is loaded. -->"""


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    folder, out = Path(sys.argv[1]), Path(sys.argv[2])
    glue = (folder / "wwlite.js").read_text(encoding="utf-8")
    page = (folder / "index.html").read_text(encoding="utf-8")
    wasm = (folder / "wwlite_bg.wasm").read_bytes()
    # The glue must stand alone: an import of a snippet file would need the folder again.
    if re.search(r"^\s*import\s", glue, re.M) or (folder / "snippets").exists():
        print("the glue imports other files; the page cannot be one file")
        return 1
    # The start the glue exports as its default: `export { initSync, __wbg_init as default };`
    # (wasm-bindgen 0.2.129) or `export default __wbg_init;` (older ones).
    found = re.search(r"^export\s*\{[^}]*?\b(\w+)\s+as\s+default\b", glue, re.M)
    found = found or re.search(r"^export default (\w+);", glue, re.M)
    if not found:
        print("the glue has no default export of its start")
        return 1
    init = found.group(1)
    if "</script" in glue.lower():
        glue = re.sub(r"</(script)", r"<\\/\1", glue, flags=re.I)
    packed = base64.b64encode(gzip.compress(wasm, 9, mtime=0)).decode("ascii")
    script = SCRIPT.format(glue=glue, packed=packed, init=init)
    start = page.index('  <script type="module">')
    end = page.index("</script>", start) + len("</script>")
    page = page[:start] + script + page[end:]
    page = re.sub(r"<!--.*?-->", COMMENT, page, count=1, flags=re.S)
    out.write_text(page, encoding="utf-8", newline="\n")
    print(f"{out}: {out.stat().st_size} bytes (the module {len(wasm)} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
