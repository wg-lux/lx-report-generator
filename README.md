# report_pdf_renderer (rust)

Standalone PDF renderer for persisted report templates.

This repository is vendored into `endoreg-db` at:

- `lx-report-generator/`

## Goals
- Fast standalone binary (Rust)
- Render report sections with explicit spacing/layout rules
- Embed frame images (single image or grid)
- Render template sentence sections with variable substitution
- Callable from Python (`subprocess`) without importing heavy PDF libs

## Quick Start (with Nix / `devenv`)
This repo includes a `devenv.nix` with Rust tooling and a bootstrap task.

From the `endoreg-db` repository root:

```bash
cd lx-report-generator
direnv allow   # optional, if you use direnv
devenv shell
```

On shell entry, `devenv` bootstraps the release binary automatically if missing.

Then generate a PDF from the example payload:

```bash
./target/release/report_pdf_renderer \
  --input examples/report_payload.json \
  --output /tmp/report_example.pdf
```

Equivalent explicit build:

```bash
cargo build --release
```

Open the generated file:

```bash
xdg-open /tmp/report_example.pdf
```

## Build (separate install, no devenv)
```bash
cargo build --release
# binary: target/release/report_pdf_renderer
```

## Setup For `endoreg_db` Runtime

Build and export the runtime path from the vendored standalone module:

```bash
cd /home/admin/endoreg-db/lx-report-generator
devenv shell
export ENDOREG_REPORT_PDF_RENDERER_BIN="$PWD/target/release/report_pdf_renderer"
```

Optional local install:

```bash
install -m755 target/release/report_pdf_renderer ~/.local/bin/report_pdf_renderer
export ENDOREG_REPORT_PDF_RENDERER_BIN="$HOME/.local/bin/report_pdf_renderer"
```

Verify the backend resolves the standalone binary:

```bash
cd /home/admin/endoreg-db
python - <<'PY'
from endoreg_db.services.report_pdf_renderer import get_renderer_binary
print(get_renderer_binary())
PY
```

## Clone Standalone Repo Separately

If you want to work on the renderer outside this checkout:

```bash
git clone git@github.com:wg-lux/lx-report-generator.git
cd lx-report-generator
direnv allow   # optional
devenv shell
./target/release/report_pdf_renderer \
  --input examples/report_payload.json \
  --output /tmp/report_example.pdf
```

## CLI
```bash
report_pdf_renderer --input payload.json --output report.pdf
```

## Run From Anywhere
Either:

```bash
./target/release/report_pdf_renderer --input payload.json --output report.pdf
```

or install/copy the binary to your `PATH`:

```bash
install -m755 target/release/report_pdf_renderer ~/.local/bin/report_pdf_renderer
report_pdf_renderer --input payload.json --output report.pdf
```

Backend integration can also point to a custom path via:

```bash
export ENDOREG_REPORT_PDF_RENDERER_BIN=/absolute/path/to/report_pdf_renderer
```

## Input JSON (snake_case)
Top-level keys:
- `title`: string
- `subtitle`: optional string
- `header`: optional object
- `layout`: optional object (`page`, `margins_mm`, spacing values)
- `assets_root`: optional path for relative image lookups
- `blocks`: ordered content blocks

### Supported block types
- `heading`
- `paragraph`
- `spacer`
- `sentence_group` (template sentences + variables)
- `image`
- `image_grid`

### `sentence_group`
```json
{
  "type": "sentence_group",
  "section_title": "Findings",
  "variables": {"segment": "sigma", "size_mm": "8"},
  "sentences": [
    {"template": "Polyp in {segment}.", "enabled": true},
    {"template": "Estimated size {size_mm} mm.", "enabled": true}
  ]
}
```

### `image_grid`
```json
{
  "type": "image_grid",
  "title": "Frames",
  "columns": 3,
  "image_paths": ["frames/f_001.png", "frames/f_002.png"],
  "captions": ["frame 1", "frame 2"]
}
```

## Notes
- This renderer uses a lightweight layout engine (text flow + page breaks).
- For now it uses built-in PDF fonts (Helvetica).
- If an image fails to load, the renderer logs and continues.
- Integrate from Python by writing a JSON payload to a temp file and invoking the binary.
- `assets_root` is used to resolve relative image paths (e.g. frame images in `image_grid`).
