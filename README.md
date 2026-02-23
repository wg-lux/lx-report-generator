# report_pdf_renderer (rust)

Standalone PDF renderer for persisted report templates.

## Goals
- Fast standalone binary (Rust)
- Render report sections with explicit spacing/layout rules
- Embed frame images (single image or grid)
- Render template sentence sections with variable substitution
- Callable from Python (`subprocess`) without importing heavy PDF libs

## Quick Start (with `devenv`)
This repo includes a `devenv.nix` with Rust tooling.

```bash
cd tools/report_pdf_renderer_rust
devenv shell
```

Then build and run:

```bash
cargo build --release
./target/release/report_pdf_renderer \
  --input examples/report_payload.json \
  --output /tmp/report_example.pdf
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
