//! Special handlers for color, transforms, links and nonvisual PDF specials.

use crate::backend::Matrix;
use crate::color::Color;
use crate::links::{parse_destination_name, Destination};
use crate::resource::ResourceProvider;
use crate::RenderError;

use super::super::{ImageKind, SP_TO_BP};
use super::Walker;

impl<P: ResourceProvider> Walker<'_, P> {
    pub(super) fn apply_transform_special(
        &mut self,
        x: i32,
        y: i32,
        contents: &[u8],
    ) -> Result<bool, RenderError> {
        let Ok(text) = std::str::from_utf8(contents) else {
            return Ok(false);
        };
        let text = text.trim();
        if text == "pdf:btrans" || text.starts_with("pdf:btrans ") {
            let args = &text["pdf:btrans".len()..];
            let transform = parse_named_matrix(args).unwrap_or(Matrix {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: 0.0,
            });
            self.pdf_graphics.begin_transform(transform, x, y, SP_TO_BP);
            self.ledger.record_supported("pdf-begin-transform");
            return Ok(true);
        }
        if text == "pdf:etrans" {
            let Some(operations) = self.pdf_graphics.restore_state() else {
                self.unsupported("pdf-transform-stack-underflow")?;
                return Ok(true);
            };
            self.color = self.pdf_graphics.fill;
            self.append_pdf_operations(operations)?;
            self.ledger.record_supported("pdf-end-transform");
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
                if values.len() < 2 {
                    self.unsupported("malformed-x-scale")?;
                } else {
                    self.pdf_graphics.concatenate_about_pen(
                        values[0], 0.0, 0.0, values[1], 0.0, 0.0, x, y, SP_TO_BP,
                    );
                    self.ledger.record_supported("x-scale");
                }
            }
            "rotate" => {
                let Some(degrees) = tokens.next().and_then(|value| value.parse::<f32>().ok())
                else {
                    self.unsupported("malformed-x-rotate")?;
                    return Ok(true);
                };
                let radians = degrees.to_radians();
                self.pdf_graphics.concatenate_about_pen(
                    radians.cos(),
                    radians.sin(),
                    -radians.sin(),
                    radians.cos(),
                    0.0,
                    0.0,
                    x,
                    y,
                    SP_TO_BP,
                );
                self.ledger.record_supported("x-rotate");
            }
            "gsave" => {
                self.pdf_graphics.save_state();
                self.ledger.record_supported("x-save");
            }
            "grestore" => {
                let Some(operations) = self.pdf_graphics.restore_state() else {
                    self.unsupported("x-transform-stack-underflow")?;
                    return Ok(true);
                };
                self.color = self.pdf_graphics.fill;
                self.append_pdf_operations(operations)?;
                self.ledger.record_supported("x-restore");
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(super) fn apply_nonvisual_special(&mut self, contents: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(contents) else {
            return false;
        };
        let text = text.trim();
        if text == "landscape" {
            self.ledger.record_supported("landscape");
            return true;
        }
        if text == "dvipdfmx:config" || text.starts_with("dvipdfmx:config ") {
            self.ledger.record_supported("dvipdfmx-config");
            return true;
        }
        let Some(args) = text.strip_prefix("pdf:") else {
            return false;
        };
        let keyword = args
            .trim_start()
            .chars()
            .take_while(|character| character.is_ascii_alphabetic())
            .collect::<String>();
        if matches!(
            keyword.as_str(),
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
            self.ledger.record_supported("pdf-nonvisual");
            return true;
        }
        false
    }

    /// Handle the specials that describe the page's overlay.
    ///
    /// These draw nothing. They say which region of the page is clickable and
    /// where a cross-reference lands, and a viewer that ignored them would show
    /// `\ref`, `\cite` and `\url` as ordinary text.
    pub(super) fn apply_link_special(
        &mut self,
        x: i32,
        y: i32,
        contents: &[u8],
    ) -> Result<bool, RenderError> {
        let Ok(text) = std::str::from_utf8(contents) else {
            return Ok(false);
        };
        let Some(args) = text.trim().strip_prefix("pdf:") else {
            return Ok(false);
        };
        let args = args.trim_start();

        if args.starts_with("dest") {
            if let Some(name) = parse_destination_name(args) {
                self.page_mut()?.destinations.push(Destination {
                    name,
                    x_bp: x as f32 * SP_TO_BP,
                    y_bp: y as f32 * SP_TO_BP,
                });
                self.ledger.record_supported("pdf-destination");
            } else {
                // A destination whose name cannot be read is still not a
                // drawing operation, so the page stays certain.
                self.ledger.record_supported("pdf-nonvisual");
            }
            return Ok(true);
        }

        if args.starts_with("bann") {
            self.begin_link(args, x, y);
            self.ledger.record_supported(if self.open_link.is_some() {
                "pdf-link"
            } else {
                "pdf-nonvisual"
            });
            return Ok(true);
        }

        if args.starts_with("eann") {
            // The closing special sits at the right edge of what it ends, which
            // is the one measurement glyph positions cannot supply.
            self.extend_link(x, x, y, 0, 0);
            self.close_link();
            self.ledger.record_supported("pdf-nonvisual");
            return Ok(true);
        }

        Ok(false)
    }

    pub(super) fn apply_color_special(&mut self, contents: &[u8]) -> Result<bool, RenderError> {
        let Ok(text) = std::str::from_utf8(contents) else {
            return Ok(false);
        };
        let text = text.trim();
        if text == "pdfcolorstackinit" || text.starts_with("pdfcolorstackinit ") {
            let args = &text["pdfcolorstackinit".len()..];
            let Some((fill, line)) = parse_pdf_colors(args, self.pdf_graphics.colors()) else {
                self.unsupported("malformed-pdfcolorstackinit")?;
                return Ok(true);
            };
            self.pdf_graphics.set_colors(fill, line);
            self.color = fill;
            self.ledger.record_supported("pdfcolorstackinit");
            return Ok(true);
        }
        if text == "color" || text.starts_with("color ") {
            let args = &text["color".len()..];
            let args = args.trim();
            if let Some(spec) = args.strip_prefix("push") {
                let Some(color) = parse_color_spec(spec.trim()) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.color_stack.push(self.pdf_graphics.colors());
                self.color = color;
                self.pdf_graphics.set_colors(color, color);
            } else if args == "pop" {
                let Some((fill, line)) = self.color_stack.pop() else {
                    self.unsupported("color-stack-underflow")?;
                    return Ok(true);
                };
                self.color = fill;
                self.pdf_graphics.set_colors(fill, line);
            } else {
                let Some(color) = parse_color_spec(args) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.color = color;
                self.pdf_graphics.set_colors(color, color);
            }
            self.ledger.record_supported("color");
            return Ok(true);
        }
        if text == "pdfcolorstack" || text.starts_with("pdfcolorstack ") {
            let args = &text["pdfcolorstack".len()..];
            let mut tokens = args.split_whitespace();
            let Some(stack_number) = tokens.next().and_then(|value| value.parse::<i32>().ok())
            else {
                self.unsupported("malformed-pdfcolorstack")?;
                return Ok(true);
            };
            let Some(action) = tokens.next() else {
                self.unsupported("malformed-pdfcolorstack")?;
                return Ok(true);
            };
            match action {
                "push" => {
                    let Some((fill, line)) = parse_pdf_colors(args, self.pdf_graphics.colors())
                    else {
                        self.unsupported("malformed-pdfcolorstack")?;
                        return Ok(true);
                    };
                    self.pdf_color_stacks
                        .entry(stack_number)
                        .or_default()
                        .push(self.pdf_graphics.colors());
                    self.color = fill;
                    self.pdf_graphics.set_colors(fill, line);
                }
                "pop" => {
                    let colors = self
                        .pdf_color_stacks
                        .get_mut(&stack_number)
                        .and_then(Vec::pop);
                    let Some((fill, line)) = colors else {
                        self.unsupported("pdfcolorstack-underflow")?;
                        return Ok(true);
                    };
                    self.color = fill;
                    self.pdf_graphics.set_colors(fill, line);
                }
                "set" => {
                    let Some((fill, line)) = parse_pdf_colors(args, self.pdf_graphics.colors())
                    else {
                        self.unsupported("malformed-pdfcolorstack")?;
                        return Ok(true);
                    };
                    self.color = fill;
                    self.pdf_graphics.set_colors(fill, line);
                }
                "current" => {}
                _ => {
                    self.unsupported("unknown-pdfcolorstack-action")?;
                    return Ok(true);
                }
            }
            self.ledger.record_supported("pdfcolorstack");
            return Ok(true);
        }
        Ok(false)
    }
}

pub(super) struct ImageSpecial {
    pub(super) path: String,
    pub(super) kind: ImageKind,
    pub(super) transform: Matrix,
    pub(super) page: u32,
    pub(super) width: Option<f32>,
    pub(super) height: Option<f32>,
}

fn parse_color_spec(spec: &str) -> Option<Color> {
    let mut tokens = spec.split_whitespace();
    let model = tokens.next()?;
    let values: Vec<f32> = tokens.filter_map(|value| value.parse().ok()).collect();
    match model.to_ascii_lowercase().as_str() {
        "gray" if !values.is_empty() => Some(Color::gray(values[0])),
        "rgb" if values.len() >= 3 => Some(Color::rgb(values[0], values[1], values[2])),
        "cmyk" if values.len() >= 4 => {
            Some(Color::cmyk(values[0], values[1], values[2], values[3]))
        }
        _ => None,
    }
}

fn parse_pdf_colors(args: &str, current: (Color, Color)) -> Option<(Color, Color)> {
    let open = args.find('(')?;
    let close = args[open + 1..]
        .find(')')
        .map(|index| open + 1 + index)
        .unwrap_or(args.len());
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

fn parse_named_matrix(args: &str) -> Option<Matrix> {
    let tokens: Vec<_> = args.split_whitespace().collect();
    let index = tokens.iter().position(|token| *token == "matrix")?;
    let values: Option<Vec<f32>> = tokens
        .get(index + 1..index + 7)?
        .iter()
        .map(|value| value.parse().ok().map(normalize_float))
        .collect();
    let values = values?;
    Some(Matrix {
        a: values[0],
        b: values[1],
        c: values[2],
        d: values[3],
        e: values[4],
        f: values[5],
    })
}

pub(super) fn parse_image_special(contents: &[u8], mut origin: Matrix) -> Option<ImageSpecial> {
    let text = std::str::from_utf8(contents).ok()?.trim();
    let args = text.strip_prefix("pdf:image")?.trim();
    let open = args.find('(')?;
    let close = args[open + 1..]
        .find(')')
        .map(|index| open + 1 + index)
        .unwrap_or(args.len());
    let path = args[open + 1..close].to_owned();
    let tokens: Vec<_> = args[..open].split_whitespace().collect();
    let mut matrix = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    let mut page = 0;
    let mut width = None;
    let mut height = None;
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index] {
            "matrix" if index + 6 < tokens.len() => {
                let values: Option<Vec<f32>> = tokens[index + 1..index + 7]
                    .iter()
                    .map(|value| value.parse().ok().map(normalize_float))
                    .collect();
                let values = values?;
                matrix = Matrix {
                    a: values[0],
                    b: values[1],
                    c: values[2],
                    d: values[3],
                    e: values[4],
                    f: values[5],
                };
                index += 7;
            }
            "page" if index + 1 < tokens.len() => {
                let raw: i32 = tokens[index + 1].parse().ok()?;
                page = raw.saturating_sub(1).max(0) as u32;
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
    let kind = if path
        .rsplit('.')
        .next()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        ImageKind::Pdf
    } else {
        ImageKind::Raster
    };
    if matches!(kind, ImageKind::Pdf) {
        origin.b = -origin.b;
        origin.d = -origin.d;
    }
    Some(ImageSpecial {
        path,
        kind,
        transform: matrix.concat(origin),
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
    Some(normalize_float(number * scale))
}

fn normalize_float(value: f32) -> f32 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}
