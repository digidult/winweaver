# The presentation

`make_presentation.py` makes the slides of WinWeaver for a quick show to a prospective user: sixteen
slides of text with the screenshots CI draws and their fragments (the profile tree, the detail of a
rule, the forms, the check, the comparison, This PC, the registry, Group Policy, the catalogs, the
work in the background, MCP, themes, wwlite, the command line). English only for now; another
language is a copy of `SLIDES` with its texts.

The Windows job of CI runs it after the screenshots and uploads `presentation-<version>` with
`WinWeaver.pptx`. By hand, on a machine with Python 3.10 or later:

```bash
pip install python-pptx pillow
gh run download <run id> -n screenshots -D shots
python docs/presentation/make_presentation.py shots WinWeaver.pptx 0.26.0
```

The screenshots are 1440 x 900 pixels at one pixel a point; a fragment is a box `(left, top, right,
bottom)` in those pixels. The file is not kept in the repository: it is made from the screenshots
of the build it shows.
