"""The presentation of WinWeaver for a quick show to a prospective user: slides made of the
screenshots CI draws (the artifact `screenshots` of the Windows job) and of their fragments.

    python docs/presentation/make_presentation.py <screenshots folder> <output .pptx> [version]

Needs python-pptx and Pillow (`pip install python-pptx pillow`). The screenshots are the PNG files
of `screenshots_of_the_window` (winweaver-ui), of wedit and of wwlite, 1440 x 900 pixels at one
pixel a point; a fragment is a box in those pixels. English only for now; another language is a
copy of SLIDES with its texts.
"""
import sys
import tempfile
from pathlib import Path

from PIL import Image
from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.util import Emu, Inches, Pt

# The window: the tab row, the command bar, the navigation row, the tree and the detail panel.
BAR = (0, 40, 800, 88)
TREE = (0, 125, 380, 870)
DETAIL = (380, 125, 1440, 870)

# A slide: the title, the lines of text, the screenshot (a file name without .png) and its box,
# or None for the whole picture. `wide` puts the picture under the text across the slide.
SLIDES = [
    {
        "title": "WinWeaver",
        "lines": [
            "An editor of Windows 11 installation profiles.",
            "A profile is a tree of about 280 rules with the forms of the installation; "
            "Build writes autounattend.xml with its scripts.",
            "The same profile reads, checks and applies the settings of a running PC, "
            "natively or through stand-alone scripts.",
            "Portable: one executable, its data folders next to it; nothing is installed. "
            "Windows 10 and 11. MIT license.",
        ],
        "shot": "01-profile",
        "box": None,
    },
    {
        "title": "A profile is a tree of rules",
        "lines": [
            "Three roots: Installation (what the answer file and its scripts do), Policies "
            "(every rule that only writes the registry), Other (services, features, apps).",
            "Every group counts its rules that are on; a group switches its rules at once.",
            "Presets to start from: Office, Strict, Laptop, Home.",
            "Search filters the tree as you type; Only on shows the rules in effect.",
        ],
        "shot": "26-only-on",
        "box": TREE,
    },
    {
        "title": "Every rule explained",
        "lines": [
            "What it does, the risk, what it writes (the registry values with the defaults of "
            "Windows), the rules it needs or conflicts with, versions, the check and the rollback.",
            "Parameters with their limits: texts, numbers, choices, lists.",
            "Links to the registry, to the same policy in other catalogs, to the rules that "
            "write the same values.",
        ],
        "shot": "25-params",
        "box": (380, 125, 1440, 730),
    },
    {
        "title": "The forms of the installation",
        "lines": [
            "Edition and product key, the name of the computer (with a template: serial number, "
            "MAC address, random characters), workgroup, disk, time zone.",
            "Accounts with their groups and passwords; a users file keeps them encrypted.",
            "Display, system and user languages, input languages and keyboard layouts, "
            "listed by Windows itself.",
        ],
        "shot": "28-many-tabs",
        "box": (380, 125, 1440, 480),
    },
    {
        "title": "Check and build",
        "lines": [
            "Check (F7) lists errors, warnings and notes; a click leads to the item.",
            "Build (F9) writes autounattend.xml and its PowerShell scripts, ASCII only, the "
            "profile embedded so it comes back whole from the answer file.",
            "Copy the file to the root of the installation USB stick: the installation runs "
            "without questions.",
        ],
        "shot": "03-checked",
        "box": None,
    },
    {
        "title": "Compare profiles",
        "lines": [
            "Two profiles side by side: every rule, parameter and form field that differs.",
            "A comment on each difference; apply one or all of them.",
            "The unsaved changes of a profile against its file; a diff of a PC against a preset.",
        ],
        "shot": "04-diff",
        "box": None,
    },
    {
        "title": "This PC",
        "lines": [
            "Read the settings of this PC into a profile and see what is in effect, partly in "
            "effect or off.",
            "Check the selected rules; apply them or return them to the Windows defaults as "
            "administrator, with a backup of every value and an undo.",
            "For other PCs: stand-alone scripts that apply, revert, audit and undo; the audit "
            "report opens as a profile.",
        ],
        "shot": "25-params",
        "box": (380, 740, 1440, 850),
        "wide": True,
    },
    {
        "title": "The registry",
        "lines": [
            "The registry of this PC in a tab: keys, values in the types of regedit, find under a "
            "key, permissions of a key, bookmarks.",
            "Every key a rule writes opens here; the tab names the rules that write a key.",
        ],
        "shot": "22-registry",
        "box": None,
    },
    {
        "title": "Registry files and Group Policy",
        "lines": [
            ".reg files as a structure: keys, values and comments edited in place, applied to "
            "this PC after a question.",
            "Registry.pol of the computer and of the users, or any Group Policy file: its entries "
            "edited, the root chosen, saved back or exported as .reg.",
        ],
        "shot": "23-pol-edit",
        "box": None,
    },
    {
        "title": "Policy templates (ADMX)",
        "lines": [
            "Import the templates of this Windows, of a folder, an archive or a catalog file; "
            "every policy becomes a rule with its parameters.",
            "Compare two catalogs policy by policy; copy the policies you need into a catalog of "
            "your own and pass it on as a file.",
            "An editor of catalogs: your own policies with the values they write.",
        ],
        "shot": "24-compare",
        "box": None,
    },
    {
        "title": "Work in the background",
        "lines": [
            "A read of this PC, an import, a change of this PC and a search of the registry run "
            "in the background while the window stays alive.",
            "A panel in the corner shows them with the time they take and Stop where they can "
            "stop; it folds to a spinner.",
        ],
        "shot": "29-jobs",
        "box": (1030, 740, 1440, 880),
        "wide": True,
    },
    {
        "title": "An MCP server for AI agents",
        "lines": [
            "Thirty-six tools: the catalog, the profiles, the bookmarks, the catalogs of "
            "templates, the registry, this PC; stdio and HTTP with a token.",
            "Modes: read, edit, files; what an agent may touch is set per configuration file.",
            "A journal of every call in the window, with the places it names as links.",
        ],
        "shot": "06-mcp",
        "box": None,
    },
    {
        "title": "Themes, languages, documentation",
        "lines": [
            "Light and Dark follow Windows; Latte and Matrix built in; a theme file is colours "
            "and fonts, with an editor.",
            "Nine languages built in; another is a file next to the program.",
            "The guide and the reference of every parameter open in the window.",
        ],
        "shot": "11-matrix",
        "box": None,
    },
    {
        "title": "wwlite, the light edition",
        "lines": [
            "The profile editor, the check and the build alone, in a smaller executable.",
            "Nothing reads or changes the PC; no registry, no MCP.",
            "For the person who only needs the answer file.",
        ],
        "shot": "l1-profile",
        "box": None,
    },
    {
        "title": "The command line",
        "lines": [
            "winweaver --build Office --out C:\\usb\\autounattend.xml",
            "winweaver --read-pc --out pc.wwprofile",
            "winweaver --diff Office Strict --out diffs",
            "winweaver --script apply profile.wwprofile --out apply.ps1",
            "winweaver --import-admx system; winweaver --export-catalog system --out c.wwcatalog",
            "winweaver --mcp http --port 8765 --token ... --mode edit",
        ],
        "shot": None,
        "box": None,
        "mono": True,
    },
    {
        "title": "Try it",
        "lines": [
            "Unpack the archive anywhere and run winweaver.exe; the guide opens at the first start.",
            "Everything stays in the folder of the program: profiles, output, imports, settings.",
            "github.com/digidult/winweaver",
        ],
        "shot": "08-guide",
        "box": None,
    },
]

WIDTH, HEIGHT = Inches(13.333), Inches(7.5)
MARGIN = Inches(0.5)
TEXT_WIDTH = Inches(4.6)
INK = RGBColor(0x1F, 0x29, 0x37)
MUTED = RGBColor(0x55, 0x5F, 0x6D)
ACCENT = RGBColor(0x2F, 0x6F, 0xD6)


def fragment(shots: Path, name: str, box, work: Path) -> Path | None:
    """The screenshot or its fragment as a file for the slide; None when the screenshot is
    missing (the slide keeps its text)."""
    source = shots / f"{name}.png"
    if not source.is_file():
        return None
    if box is None:
        return source
    with Image.open(source) as image:
        part = image.crop(box)
        target = work / f"{name}-{'-'.join(str(v) for v in box)}.png"
        part.save(target)
        return target


def add_text(slide, left, top, width, height, lines, size, color=INK, mono=False, bullets=True):
    frame = slide.shapes.add_textbox(left, top, width, height).text_frame
    frame.word_wrap = True
    for i, line in enumerate(lines):
        paragraph = frame.paragraphs[0] if i == 0 else frame.add_paragraph()
        paragraph.text = ("\u2022 " + line) if bullets and not mono else line
        paragraph.space_after = Pt(8)
        for run in paragraph.runs:
            run.font.size = Pt(size)
            run.font.color.rgb = color
            run.font.name = "Consolas" if mono else "Segoe UI"


def place_picture(slide, path: Path, left, top, width, height):
    """The picture inside the box, keeping its proportions."""
    with Image.open(path) as image:
        w, h = image.size
    scale = min(width / w, height / h)
    pw, ph = int(w * scale), int(h * scale)
    slide.shapes.add_picture(str(path), left + (width - pw) // 2, top, pw, ph)


def build(shots: Path, out: Path, version: str) -> None:
    deck = Presentation()
    deck.slide_width, deck.slide_height = WIDTH, HEIGHT
    blank = deck.slide_layouts[6]
    work = Path(tempfile.mkdtemp(prefix="winweaver-slides-"))
    for n, spec in enumerate(SLIDES, 1):
        slide = deck.slides.add_slide(blank)
        title_size = 36 if n == 1 else 28
        add_text(slide, MARGIN, Inches(0.35), WIDTH - 2 * MARGIN, Inches(0.9), [spec["title"]],
                 title_size, ACCENT if n == 1 else INK, bullets=False)
        picture = fragment(shots, spec["shot"], spec["box"], work) if spec["shot"] else None
        text_top = Inches(1.4)
        mono = spec.get("mono", False)
        if picture is None:
            add_text(slide, MARGIN, text_top, WIDTH - 2 * MARGIN, Inches(5.5), spec["lines"],
                     18, mono=mono)
        elif spec.get("wide"):
            add_text(slide, MARGIN, text_top, WIDTH - 2 * MARGIN, Inches(2.4), spec["lines"], 16)
            place_picture(slide, picture, MARGIN, Inches(4.0), WIDTH - 2 * MARGIN, Inches(3.1))
        else:
            add_text(slide, MARGIN, text_top, TEXT_WIDTH, Inches(5.6), spec["lines"], 16)
            left = MARGIN + TEXT_WIDTH + Inches(0.3)
            place_picture(slide, picture, left, text_top, WIDTH - left - MARGIN, Inches(5.7))
        add_text(slide, MARGIN, HEIGHT - Inches(0.5), WIDTH - 2 * MARGIN, Inches(0.4),
                 [f"WinWeaver {version}    {n} / {len(SLIDES)}"], 10, MUTED, bullets=False)
    out.parent.mkdir(parents=True, exist_ok=True)
    deck.save(str(out))
    print(f"{out}: {len(SLIDES)} slides")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    build(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3] if len(sys.argv) > 3 else "")
