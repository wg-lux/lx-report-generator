# AGENTS.md (report_pdf_renderer_rust)

## Purpose
Standalone Rust CLI that renders persisted report-template payloads to PDF.

This tool is intended to be installed separately and called from Python (`subprocess`) by `endoreg_db`.

## Design constraints
- Keep the CLI stable: `--input <json> --output <pdf>`
- JSON keys must be `snake_case`
- Do not depend on Django or Python internals inside this repository
- Prefer deterministic rendering for the same payload input
- Keep partial rendering resilient: missing images should not crash the whole PDF

## Input contract
- Input is a JSON document payload
- Supports ordered blocks (`heading`, `paragraph`, `spacer`, `sentence_group`, `image`, `image_grid`)
- `sentence_group` should support template substitution using `{variable_name}` placeholders
- `assets_root` should be honored for relative image paths

## Performance / implementation guidance
- Prefer streaming/simple layout logic over complex DOM/CSS engines
- Avoid unnecessary allocations in image-heavy rendering paths
- Keep page-break logic explicit and easy to reason about
- Log recoverable rendering errors to stderr and continue

## Compatibility expectations
- Output must be a valid PDF file readable by common viewers
- Keep generated PDFs stable enough for snapshot/integration testing (allowing metadata differences)
- Backward compatibility of payload fields is preferred; add new fields as optional

## Local development
```bash
cd tools/report_pdf_renderer_rust
devenv shell
cargo run -- --input examples/report_payload.json --output /tmp/report_example.pdf
```

## Integration with endoreg_db
- Python wrapper reads `ENDOREG_REPORT_PDF_RENDERER_BIN`
- If not present in PATH, backend falls back to internal minimal PDF generation
- Keep failure messages concise; they may be surfaced in backend warnings
