"""The list of the files of the folder of the page of winweaverweb that the page reads over http
at its start: the catalog files of `imports/`, the themes of `themes/` and the translations of
`lang/<code>/ui.json`, written as `files.json` next to `index.html`. A browser cannot list a
folder of a server, so the page reads what this file names.

    python manifest.py FOLDER

FOLDER is the folder of `index.html`; run it again after a file is added or removed.
"""

import json
import sys
from pathlib import Path

KINDS = [("imports", ".wwcatalog"), ("themes", ".wwtheme")]


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    root = Path(sys.argv[1])
    found = []
    for folder, ext in KINDS:
        for f in sorted((root / folder).glob(f"*{ext}")):
            found.append(f.relative_to(root).as_posix())
    for f in sorted((root / "lang").glob("*/ui.json")):
        found.append(f.relative_to(root).as_posix())
    (root / "files.json").write_text(json.dumps(found, indent=2) + "\n", encoding="utf-8")
    print(f"files.json names {len(found)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
