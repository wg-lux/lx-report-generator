use anyhow::{Context, Result};
use clap::Parser;
use printpdf::{BuiltinFont, Image, ImageTransform, Mm, PdfDocument, PdfDocumentReference, PdfLayerIndex, PdfPageIndex};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "report_pdf_renderer")]
struct Cli {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
struct LayoutConfig {
    page_width_mm: Option<f64>,
    page_height_mm: Option<f64>,
    margin_left_mm: Option<f64>,
    margin_right_mm: Option<f64>,
    margin_top_mm: Option<f64>,
    margin_bottom_mm: Option<f64>,
    line_height_mm: Option<f64>,
    paragraph_spacing_mm: Option<f64>,
    section_spacing_mm: Option<f64>,
    image_spacing_mm: Option<f64>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            page_width_mm: Some(210.0),
            page_height_mm: Some(297.0),
            margin_left_mm: Some(18.0),
            margin_right_mm: Some(18.0),
            margin_top_mm: Some(18.0),
            margin_bottom_mm: Some(18.0),
            line_height_mm: Some(5.2),
            paragraph_spacing_mm: Some(3.0),
            section_spacing_mm: Some(6.0),
            image_spacing_mm: Some(4.0),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
struct HeaderData {
    center_name: Option<String>,
    patient_label: Option<String>,
    examination_date: Option<String>,
    report_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RenderPayload {
    title: String,
    subtitle: Option<String>,
    header: Option<HeaderData>,
    layout: Option<LayoutConfig>,
    assets_root: Option<String>,
    blocks: Vec<Block>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Block {
    Heading {
        text: String,
        level: Option<u8>,
    },
    Paragraph {
        text: String,
    },
    Spacer {
        height_mm: Option<f64>,
    },
    SentenceGroup {
        section_title: Option<String>,
        variables: Option<BTreeMap<String, String>>,
        sentences: Vec<SentenceTemplate>,
    },
    Image {
        image_path: String,
        caption: Option<String>,
        width_mm: Option<f64>,
        height_mm: Option<f64>,
    },
    ImageGrid {
        title: Option<String>,
        columns: Option<usize>,
        cell_width_mm: Option<f64>,
        cell_height_mm: Option<f64>,
        image_paths: Vec<String>,
        captions: Option<Vec<String>>,
    },
}

#[derive(Debug, Deserialize)]
struct SentenceTemplate {
    template: String,
    enabled: Option<bool>,
    variables: Option<BTreeMap<String, String>>,
}

struct Renderer {
    doc: PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    font_regular: printpdf::IndirectFontRef,
    font_bold: printpdf::IndirectFontRef,
    page_width_mm: f64,
    page_height_mm: f64,
    margin_left_mm: f64,
    margin_right_mm: f64,
    margin_top_mm: f64,
    margin_bottom_mm: f64,
    line_height_mm: f64,
    paragraph_spacing_mm: f64,
    section_spacing_mm: f64,
    image_spacing_mm: f64,
    cursor_y_mm: f64,
    assets_root: Option<PathBuf>,
}

impl Renderer {
    fn new(title: &str, layout: LayoutConfig, assets_root: Option<PathBuf>) -> Result<Self> {
        let page_width_mm = layout.page_width_mm.unwrap_or(210.0);
        let page_height_mm = layout.page_height_mm.unwrap_or(297.0);
        let margin_left_mm = layout.margin_left_mm.unwrap_or(18.0);
        let margin_right_mm = layout.margin_right_mm.unwrap_or(18.0);
        let margin_top_mm = layout.margin_top_mm.unwrap_or(18.0);
        let margin_bottom_mm = layout.margin_bottom_mm.unwrap_or(18.0);
        let line_height_mm = layout.line_height_mm.unwrap_or(5.2);
        let paragraph_spacing_mm = layout.paragraph_spacing_mm.unwrap_or(3.0);
        let section_spacing_mm = layout.section_spacing_mm.unwrap_or(6.0);
        let image_spacing_mm = layout.image_spacing_mm.unwrap_or(4.0);

        let (doc, page, layer) = PdfDocument::new(title, Mm(page_width_mm), Mm(page_height_mm), "layer_1");
        let font_regular = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

        Ok(Self {
            doc,
            page,
            layer,
            font_regular,
            font_bold,
            page_width_mm,
            page_height_mm,
            margin_left_mm,
            margin_right_mm,
            margin_top_mm,
            margin_bottom_mm,
            line_height_mm,
            paragraph_spacing_mm,
            section_spacing_mm,
            image_spacing_mm,
            cursor_y_mm: page_height_mm - margin_top_mm,
            assets_root,
        })
    }

    fn current_layer(&self) -> printpdf::PdfLayerReference {
        self.doc.get_page(self.page).get_layer(self.layer)
    }

    fn content_width_mm(&self) -> f64 {
        self.page_width_mm - self.margin_left_mm - self.margin_right_mm
    }

    fn ensure_vertical_space(&mut self, needed_mm: f64) {
        let limit_y = self.margin_bottom_mm;
        if self.cursor_y_mm - needed_mm < limit_y {
            let (page, layer) = self.doc.add_page(Mm(self.page_width_mm), Mm(self.page_height_mm), "layer");
            self.page = page;
            self.layer = layer;
            self.cursor_y_mm = self.page_height_mm - self.margin_top_mm;
        }
    }

    fn draw_text_line(&mut self, text: &str, font_size_pt: f64, bold: bool) {
        self.ensure_vertical_space(self.line_height_mm);
        let layer = self.current_layer();
        let font = if bold { &self.font_bold } else { &self.font_regular };
        layer.use_text(text, font_size_pt, Mm(self.margin_left_mm), Mm(self.cursor_y_mm), font);
        self.cursor_y_mm -= self.line_height_mm;
    }

    fn estimate_chars_per_line(&self, font_size_pt: f64) -> usize {
        let width_mm = self.content_width_mm();
        let avg_char_mm = (font_size_pt * 0.352_778_f64) * 0.48;
        ((width_mm / avg_char_mm).floor() as usize).max(20)
    }

    fn wrap_text(&self, text: &str, font_size_pt: f64) -> Vec<String> {
        let max_chars = self.estimate_chars_per_line(font_size_pt);
        let mut out = Vec::new();
        for para in text.split('\n') {
            let words: Vec<&str> = para.split_whitespace().collect();
            if words.is_empty() {
                out.push(String::new());
                continue;
            }
            let mut line = String::new();
            for w in words {
                let candidate = if line.is_empty() {
                    w.to_string()
                } else {
                    format!("{} {}", line, w)
                };
                if candidate.chars().count() > max_chars && !line.is_empty() {
                    out.push(line);
                    line = w.to_string();
                } else {
                    line = candidate;
                }
            }
            if !line.is_empty() {
                out.push(line);
            }
        }
        out
    }

    fn draw_paragraph(&mut self, text: &str, font_size_pt: f64) {
        let lines = self.wrap_text(text, font_size_pt);
        let needed = (lines.len() as f64 * self.line_height_mm) + self.paragraph_spacing_mm;
        self.ensure_vertical_space(needed);
        for line in lines {
            self.draw_text_line(&line, font_size_pt, false);
        }
        self.cursor_y_mm -= self.paragraph_spacing_mm;
    }

    fn draw_heading(&mut self, text: &str, level: u8) {
        let font_size = match level {
            1 => 18.0,
            2 => 14.0,
            _ => 12.0,
        };
        self.ensure_vertical_space(self.section_spacing_mm + self.line_height_mm * 2.0);
        self.cursor_y_mm -= self.section_spacing_mm / 2.0;
        for line in self.wrap_text(text, font_size) {
            self.draw_text_line(&line, font_size, true);
        }
        self.cursor_y_mm -= self.section_spacing_mm / 2.0;
    }

    fn draw_spacer(&mut self, height_mm: f64) {
        self.ensure_vertical_space(height_mm);
        self.cursor_y_mm -= height_mm;
    }

    fn resolve_image_path(&self, raw_path: &str) -> PathBuf {
        let p = Path::new(raw_path);
        if p.is_absolute() {
            return p.to_path_buf();
        }
        if let Some(root) = &self.assets_root {
            return root.join(p);
        }
        p.to_path_buf()
    }

    fn draw_image_block(
        &mut self,
        image_path: &str,
        caption: Option<&str>,
        width_mm: Option<f64>,
        height_mm: Option<f64>,
    ) {
        let resolved = self.resolve_image_path(image_path);
        let dyn_img = match image::open(&resolved) {
            Ok(img) => img,
            Err(err) => {
                self.draw_paragraph(&format!("[image missing: {} ({})]", resolved.display(), err), 9.0);
                return;
            }
        };

        let (img_w_px, img_h_px) = (dyn_img.width() as f64, dyn_img.height() as f64);
        if img_w_px <= 0.0 || img_h_px <= 0.0 {
            self.draw_paragraph("[invalid image dimensions]", 9.0);
            return;
        }

        let target_w = width_mm.unwrap_or(self.content_width_mm().min(120.0));
        let target_h = height_mm.unwrap_or(target_w * (img_h_px / img_w_px));
        let extra_caption = if caption.is_some() { self.line_height_mm + self.image_spacing_mm } else { 0.0 };
        self.ensure_vertical_space(target_h + extra_caption + self.image_spacing_mm);

        let image = Image::from_dynamic_image(&dyn_img);
        let layer = self.current_layer();

        let dpi = 300.0;
        let img_w_mm_native = img_w_px * 25.4 / dpi;
        let img_h_mm_native = img_h_px * 25.4 / dpi;
        let scale_x = if img_w_mm_native > 0.0 { target_w / img_w_mm_native } else { 1.0 };
        let scale_y = if img_h_mm_native > 0.0 { target_h / img_h_mm_native } else { 1.0 };

        image.add_to_layer(
            layer.clone(),
            ImageTransform {
                translate_x: Some(Mm(self.margin_left_mm)),
                translate_y: Some(Mm(self.cursor_y_mm - target_h)),
                rotate: None,
                scale_x: Some(scale_x),
                scale_y: Some(scale_y),
                dpi: Some(dpi),
            },
        );

        self.cursor_y_mm -= target_h + self.image_spacing_mm;
        if let Some(c) = caption {
            self.draw_paragraph(c, 9.0);
        }
    }

    fn draw_image_grid(
        &mut self,
        title: Option<&str>,
        columns: usize,
        cell_width_mm: Option<f64>,
        cell_height_mm: Option<f64>,
        image_paths: &[String],
        captions: Option<&[String]>,
    ) {
        if let Some(t) = title {
            self.draw_heading(t, 3);
        }
        let cols = columns.max(1);
        let gap = self.image_spacing_mm;
        let total_gap = gap * ((cols - 1) as f64);
        let w = cell_width_mm.unwrap_or((self.content_width_mm() - total_gap) / cols as f64);
        let h = cell_height_mm.unwrap_or(w * 0.75);

        for (idx, img_path) in image_paths.iter().enumerate() {
            let col = idx % cols;
            if col == 0 {
                // new row
                let row_space = h + self.line_height_mm + self.image_spacing_mm * 2.0;
                self.ensure_vertical_space(row_space);
            }

            let x = self.margin_left_mm + (col as f64) * (w + gap);
            let y_top = self.cursor_y_mm;
            let resolved = self.resolve_image_path(img_path);
            let dyn_img = match image::open(&resolved) {
                Ok(img) => img,
                Err(_) => {
                    let layer = self.current_layer();
                    layer.use_text(
                        "[img missing]",
                        8.0,
                        Mm(x),
                        Mm(y_top - 4.0),
                        &self.font_regular,
                    );
                    if col == cols - 1 || idx == image_paths.len() - 1 {
                        self.cursor_y_mm -= h + self.line_height_mm + self.image_spacing_mm;
                    }
                    continue;
                }
            };

            let image = Image::from_dynamic_image(&dyn_img);
            let img_w_px = dyn_img.width() as f64;
            let img_h_px = dyn_img.height() as f64;
            let dpi = 300.0;
            let native_w_mm = img_w_px * 25.4 / dpi;
            let native_h_mm = img_h_px * 25.4 / dpi;
            let scale_x = if native_w_mm > 0.0 { w / native_w_mm } else { 1.0 };
            let scale_y = if native_h_mm > 0.0 { h / native_h_mm } else { 1.0 };

            image.add_to_layer(
                self.current_layer(),
                ImageTransform {
                    translate_x: Some(Mm(x)),
                    translate_y: Some(Mm(y_top - h)),
                    rotate: None,
                    scale_x: Some(scale_x),
                    scale_y: Some(scale_y),
                    dpi: Some(dpi),
                },
            );

            if let Some(caps) = captions {
                if let Some(cap) = caps.get(idx) {
                    self.current_layer().use_text(
                        cap,
                        8.0,
                        Mm(x),
                        Mm(y_top - h - 3.0),
                        &self.font_regular,
                    );
                }
            }

            if col == cols - 1 || idx == image_paths.len() - 1 {
                self.cursor_y_mm -= h + self.line_height_mm + self.image_spacing_mm;
            }
        }
        self.cursor_y_mm -= self.image_spacing_mm;
    }

    fn render_header(&mut self, payload: &RenderPayload) {
        self.draw_heading(&payload.title, 1);
        if let Some(sub) = &payload.subtitle {
            self.draw_paragraph(sub, 10.0);
        }
        if let Some(h) = &payload.header {
            let mut meta_lines = Vec::new();
            if let Some(v) = &h.center_name { meta_lines.push(format!("Center: {}", v)); }
            if let Some(v) = &h.patient_label { meta_lines.push(format!("Patient: {}", v)); }
            if let Some(v) = &h.examination_date { meta_lines.push(format!("Examination date: {}", v)); }
            if let Some(v) = &h.report_version { meta_lines.push(format!("Version: {}", v)); }
            if !meta_lines.is_empty() {
                self.draw_paragraph(&meta_lines.join(" | "), 9.0);
            }
        }
        self.draw_spacer(self.section_spacing_mm);
    }

    fn render_block(&mut self, block: Block) {
        match block {
            Block::Heading { text, level } => self.draw_heading(&text, level.unwrap_or(2)),
            Block::Paragraph { text } => self.draw_paragraph(&text, 11.0),
            Block::Spacer { height_mm } => self.draw_spacer(height_mm.unwrap_or(self.paragraph_spacing_mm)),
            Block::SentenceGroup { section_title, variables, sentences } => {
                if let Some(t) = section_title {
                    self.draw_heading(&t, 2);
                }
                let base = variables.unwrap_or_default();
                let mut rendered_lines = Vec::new();
                for sentence in sentences {
                    if sentence.enabled == Some(false) {
                        continue;
                    }
                    let mut vars = base.clone();
                    if let Some(extra) = sentence.variables {
                        for (k, v) in extra {
                            vars.insert(k, v);
                        }
                    }
                    let rendered = render_template_sentence(&sentence.template, &vars);
                    if !rendered.trim().is_empty() {
                        rendered_lines.push(rendered);
                    }
                }
                self.draw_paragraph(&rendered_lines.join("\n"), 11.0);
            }
            Block::Image { image_path, caption, width_mm, height_mm } => {
                self.draw_image_block(&image_path, caption.as_deref(), width_mm, height_mm)
            }
            Block::ImageGrid { title, columns, cell_width_mm, cell_height_mm, image_paths, captions } => {
                self.draw_image_grid(
                    title.as_deref(),
                    columns.unwrap_or(3),
                    cell_width_mm,
                    cell_height_mm,
                    &image_paths,
                    captions.as_deref(),
                )
            }
        }
    }

    fn save(self, out_path: &Path) -> Result<()> {
        let mut file = fs::File::create(out_path)
            .with_context(|| format!("create output file {}", out_path.display()))?;
        self.doc.save(&mut file)?;
        Ok(())
    }
}

fn render_template_sentence(template: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = template[i + 1..].find('}') {
                let key = &template[i + 1..i + 1 + end];
                if let Some(val) = vars.get(key) {
                    out.push_str(val);
                }
                i += end + 2;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let raw = fs::read_to_string(&cli.input)
        .with_context(|| format!("read input {}", cli.input.display()))?;
    let payload: RenderPayload = serde_json::from_str(&raw)
        .with_context(|| format!("parse json {}", cli.input.display()))?;

    let layout = payload.layout.clone().unwrap_or_default();
    let assets_root = payload.assets_root.as_ref().map(PathBuf::from);

    let mut renderer = Renderer::new(&payload.title, layout, assets_root)?;
    renderer.render_header(&payload);
    for block in payload.blocks {
        renderer.render_block(block);
    }
    renderer.save(&cli.output)?;
    Ok(())
}
