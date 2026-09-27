//! Special handlers for color, transforms, links and nonvisual PDF specials.

use crate::backend::Matrix;
use crate::color::Color;
use crate::links::{parse_destination_name, Destination};
use crate::resource::ResourceProvider;
use crate::units::normalize_zero;
use crate::RenderError;

use super::super::ImageKind;
use super::Walker;

/// Arguments after `keyword` when `text` is exactly it or it plus a space.
fn keyword_args<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    (rest.is_empty() || rest.starts_with(' ')).then_some(rest)
}

impl<P: ResourceProvider> Walker<'_, P> {
    fn set_colors(&mut self, fill: Color, line: Color) {
        self.state.color = fill;
        self.state.pdf_graphics.set_colors(fill, line);
    }

    /// Pop one saved graphics state, recording `underflow` when there is none.
    fn restore_graphics(&mut self, underflow: &str) -> Result<bool, RenderError> {
        let Some(operations) = self.state.pdf_graphics.restore_state() else {
            self.unsupported(underflow)?;
            return Ok(false);
        };
        self.state.color = self.state.pdf_graphics.fill;
        self.append_pdf_operations(operations)?;
        Ok(true)
    }

    pub(super) fn apply_transform_special(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
    ) -> Result<bool, RenderError> {
        if let Some(args) = keyword_args(text, "pdf:btrans") {
            let transform = parse_named_matrix(args).unwrap_or(Matrix::IDENTITY);
            self.state.pdf_graphics.begin_transform(transform, x, y);
            self.state.ledger.record_supported("pdf-begin-transform");
            return Ok(true);
        }
        if text == "pdf:etrans" {
            if self.restore_graphics("pdf-transform-stack-underflow")? {
                self.state.ledger.record_supported("pdf-end-transform");
            }
            return Ok(true);
        }
        let Some(args) = text.strip_prefix("x:") else {
            return Ok(false);
        };
        let mut tokens = args.split_whitespace();
        let Some(command) = tokens.next() else {
            return Ok(false);
        };
        match command {
            "scale" => {
                let values: Vec<_> = tokens
                    .filter_map(|value| value.parse::<f32>().ok())
                    .collect();
                let [sx, sy, ..] = values[..] else {
                    self.unsupported("malformed-x-scale")?;
                    return Ok(true);
                };
                let scale = Matrix {
                    a: sx,
                    d: sy,
                    ..Matrix::IDENTITY
                };
                self.state.pdf_graphics.concatenate_about_pen(scale, x, y);
                self.state.ledger.record_supported("x-scale");
            }
            "rotate" => {
                let Some(degrees) = tokens.next().and_then(|value| value.parse::<f32>().ok())
                else {
                    self.unsupported("malformed-x-rotate")?;
                    return Ok(true);
                };
                let (sin, cos) = degrees.to_radians().sin_cos();
                let rotation = Matrix {
                    a: cos,
                    b: sin,
                    c: -sin,
                    d: cos,
                    e: 0.0,
                    f: 0.0,
                };
                self.state
                    .pdf_graphics
                    .concatenate_about_pen(rotation, x, y);
                self.state.ledger.record_supported("x-rotate");
            }
            "gsave" => {
                self.state.pdf_graphics.save_state();
                self.state.ledger.record_supported("x-save");
            }
            "grestore" => {
                if self.restore_graphics("x-transform-stack-underflow")? {
                    self.state.ledger.record_supported("x-restore");
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(super) fn apply_nonvisual_special(&mut self, text: &str) -> bool {
        if text == "landscape" {
            self.state.ledger.record_supported("landscape");
            return true;
        }
        if keyword_args(text, "dvipdfmx:config").is_some() {
            self.state.ledger.record_supported("dvipdfmx-config");
            return true;
        }
        let Some(args) = text.strip_prefix("pdf:") else {
            return false;
        };
        let args = args.trim_start();
        let keyword = &args[..args
            .find(|character: char| !character.is_ascii_alphabetic())
            .unwrap_or(args.len())];
        if matches!(
            keyword,
            "out"
                | "outline"
                | "ann"
                | "obj"
                | "put"
                | "close"
                | "bcolor"
                | "ecolor"
                | "docinfo"
                | "docview"
                | "pagesize"
                | "majorversion"
                | "minorversion"
                | "bcontent"
                | "econtent"
                | "bxobj"
                | "exobj"
                | "bform"
                | "eform"
                | "fstream"
        ) {
            self.state.ledger.record_supported("pdf-nonvisual");
            return true;
        }
        false
    }

    /// Links and destinations draw nothing but make up the page's click overlay.
    pub(super) fn apply_link_special(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
    ) -> Result<bool, RenderError> {
        let Some(args) = text.strip_prefix("pdf:") else {
            return Ok(false);
        };
        let args = args.trim_start();

        if args.starts_with("dest") {
            if let Some(name) = parse_destination_name(args) {
                let (x_bp, y_bp) = self.state.pdf_graphics.page_point(x, y);
                self.page_mut()?
                    .destinations
                    .push(Destination { name, x_bp, y_bp });
                self.state.ledger.record_supported("pdf-destination");
            } else {
                // Still draws nothing, so the page stays certain.
                self.state.ledger.record_supported("pdf-nonvisual");
            }
            return Ok(true);
        }

        if args.starts_with("bann") {
            self.begin_link(args, x, y);
            self.state
                .ledger
                .record_supported(if self.open_link.is_some() {
                    "pdf-link"
                } else {
                    "pdf-nonvisual"
                });
            return Ok(true);
        }

        if args.starts_with("eann") {
            // The closing special marks the right edge, which glyph positions cannot supply.
            self.extend_link(x, x, y, 0, 0);
            self.close_link();
            self.state.ledger.record_supported("pdf-nonvisual");
            return Ok(true);
        }

        Ok(false)
    }

    pub(super) fn apply_color_special(&mut self, text: &str) -> Result<bool, RenderError> {
        if let Some(args) = keyword_args(text, "pdfcolorstackinit") {
            let Some((fill, line)) = parse_pdf_colors(args, self.state.pdf_graphics.colors())
            else {
                self.unsupported("malformed-pdfcolorstackinit")?;
                return Ok(true);
            };
            self.set_colors(fill, line);
            self.state.ledger.record_supported("pdfcolorstackinit");
            return Ok(true);
        }
        if let Some(args) = keyword_args(text, "color") {
            let args = args.trim();
            if let Some(spec) = args.strip_prefix("push") {
                let Some(color) = parse_color_spec(spec.trim()) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.state
                    .color_stack
                    .push(self.state.pdf_graphics.colors());
                self.set_colors(color, color);
            } else if args == "pop" {
                let Some((fill, line)) = self.state.color_stack.pop() else {
                    self.unsupported("color-stack-underflow")?;
                    return Ok(true);
                };
                self.set_colors(fill, line);
            } else {
                let Some(color) = parse_color_spec(args) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.set_colors(color, color);
            }
            self.state.ledger.record_supported("color");
            return Ok(true);
        }
        if let Some(args) = keyword_args(text, "pdfcolorstack") {
            let mut tokens = args.split_whitespace();
            let stack_number = tokens.next().and_then(|value| value.parse::<i32>().ok());
            let (Some(stack_number), Some(action)) = (stack_number, tokens.next()) else {
                self.unsupported("malformed-pdfcolorstack")?;
                return Ok(true);
            };
            match action {
                "push" | "set" => {
                    let Some((fill, line)) =
                        parse_pdf_colors(args, self.state.pdf_graphics.colors())
                    else {
                        self.unsupported("malformed-pdfcolorstack")?;
                        return Ok(true);
                    };
                    if action == "push" {
                        self.state
                            .pdf_color_stacks
                            .entry(stack_number)
                            .or_default()
                            .push(self.state.pdf_graphics.colors());
                    }
                    self.set_colors(fill, line);
                }
                "pop" => {
                    let colors = self
                        .state
                        .pdf_color_stacks
                        .get_mut(&stack_number)
                        .and_then(Vec::pop);
                    let Some((fill, line)) = colors else {
                        self.unsupported("pdfcolorstack-underflow")?;
                        return Ok(true);
                    };
                    self.set_colors(fill, line);
                }
                "current" => {}
                _ => {
                    self.unsupported("unknown-pdfcolorstack-action")?;
                    return Ok(true);
                }
            }
            self.state.ledger.record_supported("pdfcolorstack");
            return Ok(true);
        }
        Ok(false)
    }
}

/// `pdf:image` placement, before the pen transform is applied.
pub(super) struct ImageSpecial {
    pub(super) path: String,
    pub(super) kind: ImageKind,
    matrix: Matrix,
    pub(super) page: u32,
    pub(super) width: Option<f32>,
    pub(super) height: Option<f32>,
}

impl ImageSpecial {
    /// Page transform for an image drawn at `origin`.
    pub(super) fn transform(&self, mut origin: Matrix) -> Matrix {
        if matches!(self.kind, ImageKind::Pdf) {
            origin.b = -origin.b;
            origin.d = -origin.d;
        }
        self.matrix.concat(origin)
    }
}

fn parse_color_spec(spec: &str) -> Option<Color> {
    let mut tokens = spec.split_whitespace();
    let model = tokens.next()?;
    let values: Vec<f32> = tokens.filter_map(|value| value.parse().ok()).collect();
    match (model.to_ascii_lowercase().as_str(), values.as_slice()) {
        ("gray", &[gray, ..]) => Some(Color::gray(gray)),
        ("rgb", &[red, green, blue, ..]) => Some(Color::rgb(red, green, blue)),
        ("cmyk", &[cyan, magenta, yellow, black, ..]) => {
            Some(Color::cmyk(cyan, magenta, yellow, black))
        }
        _ => None,
    }
}

/// Byte range inside the first parenthesis group, running to the end when unclosed.
fn parenthesized(args: &str) -> Option<(usize, usize)> {
    let open = args.find('(')?;
    let close = args[open + 1..]
        .find(')')
        .map_or(args.len(), |index| open + 1 + index);
    Some((open, close))
}

fn parse_pdf_colors(args: &str, current: (Color, Color)) -> Option<(Color, Color)> {
    let (open, close) = parenthesized(args)?;
    let (mut fill, mut line) = current;
    let mut values = Vec::new();
    let mut recognized = false;
    for token in args[open + 1..close].split_whitespace() {
        if let Ok(value) = token.parse::<f32>() {
            values.push(value);
            continue;
        }
        let tail = |count: usize| values.get(values.len().checked_sub(count)?..);
        match token {
            "g" => fill = Color::gray(*tail(1)?.first()?),
            "G" => line = Color::gray(*tail(1)?.first()?),
            "rg" => {
                let values = tail(3)?;
                fill = Color::rgb(values[0], values[1], values[2]);
            }
            "RG" => {
                let values = tail(3)?;
                line = Color::rgb(values[0], values[1], values[2]);
            }
            "k" => {
                let values = tail(4)?;
                fill = Color::cmyk(values[0], values[1], values[2], values[3]);
            }
            "K" => {
                let values = tail(4)?;
                line = Color::cmyk(values[0], values[1], values[2], values[3]);
            }
            _ => return None,
        }
        recognized = true;
        values.clear();
    }
    recognized.then_some((fill, line))
}

/// Six matrix entries, or `None` unless every one parses.
fn parse_matrix(tokens: &[&str]) -> Option<Matrix> {
    let mut values = [0.0; 6];
    for (value, token) in values.iter_mut().zip(tokens) {
        *value = normalize_zero(token.parse().ok()?);
    }
    let [a, b, c, d, e, f] = values;
    Some(Matrix { a, b, c, d, e, f })
}

fn parse_named_matrix(args: &str) -> Option<Matrix> {
    let tokens: Vec<_> = args.split_whitespace().collect();
    let index = tokens.iter().position(|token| *token == "matrix")?;
    parse_matrix(tokens.get(index + 1..index + 7)?)
}

pub(super) fn parse_image_special(text: &str) -> Option<ImageSpecial> {
    let args = text.strip_prefix("pdf:image")?.trim();
    let (open, close) = parenthesized(args)?;
    let path = args[open + 1..close].to_owned();
    let tokens: Vec<_> = args[..open].split_whitespace().collect();
    let mut matrix = Matrix::IDENTITY;
    let mut page = 0;
    let mut width = None;
    let mut height = None;
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index] {
            "matrix" if index + 6 < tokens.len() => {
                matrix = parse_matrix(&tokens[index + 1..index + 7])?;
                index += 7;
            }
            "page" if index + 1 < tokens.len() => {
                let raw: i32 = tokens[index + 1].parse().ok()?;
                page = u32::try_from(raw.saturating_sub(1)).unwrap_or(0);
                index += 2;
            }
            "width" if index + 1 < tokens.len() => {
                width = parse_image_dimension(tokens[index + 1]);
                index += 2;
            }
            "height" if index + 1 < tokens.len() => {
                height = parse_image_dimension(tokens[index + 1]);
                index += 2;
            }
            _ => index += 1,
        }
    }
    let is_pdf = path
        .rsplit('.')
        .next()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
    Some(ImageSpecial {
        path,
        kind: if is_pdf {
            ImageKind::Pdf
        } else {
            ImageKind::Raster
        },
        matrix,
        page,
        width,
        height,
    })
}

fn parse_image_dimension(value: &str) -> Option<f32> {
    let split = value
        .find(|character: char| {
            !character.is_ascii_digit() && !matches!(character, '.' | '-' | '+')
        })
        .unwrap_or(value.len());
    let number: f32 = value[..split].parse().ok()?;
    let scale = match &value[split..] {
        "" | "bp" => 1.0,
        "pt" => 72.0 / 72.27,
        "in" => 72.0,
        _ => return None,
    };
    Some(normalize_zero(number * scale))
}
