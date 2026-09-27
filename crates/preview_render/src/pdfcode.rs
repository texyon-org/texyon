//! Persistent interpreter for `pdf:code` content-stream fragments.

use crate::units::SP_TO_BP;
use crate::{Color, Matrix, Path, PathCommand, StrokeStyle};

const ONE_INCH_BP: f32 = 72.0;

#[derive(Clone)]
pub(crate) enum PdfOperation {
    Fill {
        path: Path,
        even_odd: bool,
        transform: Matrix,
        color: Color,
    },
    Stroke {
        path: Path,
        style: StrokeStyle,
        transform: Matrix,
        color: Color,
    },
    Clip {
        path: Path,
        even_odd: bool,
        transform: Matrix,
    },
    PopClip,
}

#[derive(Clone, Copy)]
struct SavedState {
    ctm: Matrix,
    gh: i32,
    gv: i32,
    fill: Color,
    line: Color,
    style: StrokeStyle,
    clip_depth: u32,
}

/// Persistent PDF graphics state. XeTeX commonly splits one path over many specials.
#[derive(Clone)]
pub(crate) struct PdfGraphics {
    ctm: Matrix,
    gh: i32,
    gv: i32,
    pub(crate) fill: Color,
    line: Color,
    style: StrokeStyle,
    clip_depth: u32,
    path: Path,
    stack: Vec<SavedState>,
}

pub(crate) struct PdfRun {
    pub(crate) operations: Vec<PdfOperation>,
    pub(crate) supported: Vec<&'static str>,
    pub(crate) unsupported: Vec<String>,
}

impl Default for PdfGraphics {
    fn default() -> Self {
        Self {
            ctm: Matrix {
                d: -1.0,
                e: ONE_INCH_BP,
                f: ONE_INCH_BP,
                ..Matrix::IDENTITY
            },
            gh: 0,
            gv: 0,
            fill: Color::BLACK,
            line: Color::BLACK,
            style: StrokeStyle {
                width: 1.0,
                join: 0,
                cap: 0,
                miter_limit: 10.0,
                dash: [0.0; 4],
                dash_len: 0,
                dash_phase: 0.0,
            },
            clip_depth: 0,
            path: Path::default(),
            stack: Vec::new(),
        }
    }
}

impl PdfGraphics {
    pub(crate) fn reset_page(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn set_colors(&mut self, fill: Color, line: Color) {
        self.fill = fill;
        self.line = line;
    }

    pub(crate) fn colors(&self) -> (Color, Color) {
        (self.fill, self.line)
    }

    /// Pen position relative to the last transform origin, in PDF user space.
    fn user_offset(&self, pen_x: i32, pen_y: i32) -> (f32, f32) {
        (
            pen_x.wrapping_sub(self.gh) as f32 * SP_TO_BP,
            -(pen_y.wrapping_sub(self.gv) as f32 * SP_TO_BP),
        )
    }

    pub(crate) fn current_transform(&self, pen_x: i32, pen_y: i32) -> Matrix {
        let (e, f) = self.user_offset(pen_x, pen_y);
        Matrix {
            e,
            f,
            ..Matrix::IDENTITY
        }
        .concat(self.ctm)
    }

    pub(crate) fn save_state(&mut self) {
        self.stack.push(SavedState {
            ctm: self.ctm,
            gh: self.gh,
            gv: self.gv,
            fill: self.fill,
            line: self.line,
            style: self.style,
            clip_depth: self.clip_depth,
        });
    }

    fn restore(&mut self, operations: &mut Vec<PdfOperation>) -> bool {
        let Some(saved) = self.stack.pop() else {
            return false;
        };
        for _ in saved.clip_depth..self.clip_depth {
            operations.push(PdfOperation::PopClip);
        }
        self.ctm = saved.ctm;
        self.gh = saved.gh;
        self.gv = saved.gv;
        self.fill = saved.fill;
        self.line = saved.line;
        self.style = saved.style;
        self.clip_depth = saved.clip_depth;
        true
    }

    pub(crate) fn restore_state(&mut self) -> Option<Vec<PdfOperation>> {
        let mut operations = Vec::new();
        self.restore(&mut operations).then_some(operations)
    }

    pub(crate) fn begin_transform(&mut self, transform: Matrix, pen_x: i32, pen_y: i32) {
        self.save_state();
        self.ctm = self.current_transform(pen_x, pen_y);
        self.gh = pen_x;
        self.gv = pen_y;
        self.ctm = transform.concat(self.ctm);
    }

    /// Apply the linear part of `matrix` about the pen instead of the origin.
    pub(crate) fn concatenate_about_pen(&mut self, matrix: Matrix, pen_x: i32, pen_y: i32) {
        let Matrix { a, b, c, d, e, f } = matrix;
        let (user_x, user_y) = self.user_offset(pen_x, pen_y);
        let transform = Matrix {
            e: (1.0 - a) * user_x - c * user_y + e,
            f: (1.0 - d) * user_y - b * user_x + f,
            ..matrix
        };
        self.ctm = transform.concat(self.ctm);
    }

    pub(crate) fn run(&mut self, payload: &[u8], pen_x: i32, pen_y: i32) -> PdfRun {
        let mut run = PdfRun {
            operations: Vec::new(),
            supported: Vec::new(),
            unsupported: Vec::new(),
        };
        let mut operands = Vec::new();
        let mut pending_array = None;
        for token in lex(payload) {
            match token {
                Token::Number(value) => operands.push(value),
                Token::Array(values) => pending_array = Some(values),
                Token::Name => {}
                Token::Operator(operator) => {
                    self.execute(
                        &operator,
                        &operands,
                        &mut pending_array,
                        pen_x,
                        pen_y,
                        &mut run,
                    );
                    operands.clear();
                    pending_array = None;
                }
            }
        }
        run
    }

    fn execute(
        &mut self,
        operator: &str,
        operands: &[f32],
        array: &mut Option<Vec<f32>>,
        pen_x: i32,
        pen_y: i32,
        run: &mut PdfRun,
    ) {
        let arg = |count: usize, index: usize| {
            operands
                .len()
                .checked_sub(count)
                .and_then(|start| operands.get(start + index))
                .copied()
                .unwrap_or(0.0)
        };
        match operator {
            "cm" => {
                let matrix = Matrix {
                    a: arg(6, 0),
                    b: arg(6, 1),
                    c: arg(6, 2),
                    d: arg(6, 3),
                    e: arg(6, 4),
                    f: arg(6, 5),
                };
                self.ctm = matrix.concat(self.current_transform(pen_x, pen_y));
                self.gh = pen_x;
                self.gv = pen_y;
                run.supported.push("pdf-code-transform");
            }
            "q" => {
                self.save_state();
                run.supported.push("pdf-code-save");
            }
            "Q" => {
                if self.restore(&mut run.operations) {
                    run.supported.push("pdf-code-restore");
                } else {
                    run.unsupported.push("pdf-code-stack-underflow".into());
                }
            }
            "g" => {
                self.fill = Color::gray(arg(1, 0));
                run.supported.push("pdf-code-color");
            }
            "G" => {
                self.line = Color::gray(arg(1, 0));
                run.supported.push("pdf-code-color");
            }
            "rg" => {
                self.fill = Color::rgb(arg(3, 0), arg(3, 1), arg(3, 2));
                run.supported.push("pdf-code-color");
            }
            "RG" => {
                self.line = Color::rgb(arg(3, 0), arg(3, 1), arg(3, 2));
                run.supported.push("pdf-code-color");
            }
            "k" => {
                self.fill = Color::cmyk(arg(4, 0), arg(4, 1), arg(4, 2), arg(4, 3));
                run.supported.push("pdf-code-color");
            }
            "K" => {
                self.line = Color::cmyk(arg(4, 0), arg(4, 1), arg(4, 2), arg(4, 3));
                run.supported.push("pdf-code-color");
            }
            "w" => {
                self.style.width = arg(1, 0);
                run.supported.push("pdf-code-stroke-state");
            }
            "j" => {
                self.style.join = arg(1, 0) as i32;
                run.supported.push("pdf-code-stroke-state");
            }
            "J" => {
                self.style.cap = arg(1, 0) as i32;
                run.supported.push("pdf-code-stroke-state");
            }
            "M" => {
                self.style.miter_limit = arg(1, 0);
                run.supported.push("pdf-code-stroke-state");
            }
            "d" => {
                if let Some(dash) = array.take() {
                    self.style.dash = [0.0; 4];
                    self.style.dash_len = dash.len().min(4);
                    self.style.dash[..self.style.dash_len]
                        .copy_from_slice(&dash[..self.style.dash_len]);
                    self.style.dash_phase = arg(1, 0);
                    run.supported.push("pdf-code-stroke-state");
                }
            }
            "m" => self
                .path
                .commands
                .push(PathCommand::MoveTo(arg(2, 0), arg(2, 1))),
            "l" => self
                .path
                .commands
                .push(PathCommand::LineTo(arg(2, 0), arg(2, 1))),
            "c" => self.path.commands.push(PathCommand::CurveTo(
                arg(6, 0),
                arg(6, 1),
                arg(6, 2),
                arg(6, 3),
                arg(6, 4),
                arg(6, 5),
            )),
            "v" => self.path.commands.push(PathCommand::CurveTo(
                arg(4, 0),
                arg(4, 1),
                arg(4, 0),
                arg(4, 1),
                arg(4, 2),
                arg(4, 3),
            )),
            "y" => self.path.commands.push(PathCommand::CurveTo(
                arg(4, 0),
                arg(4, 1),
                arg(4, 2),
                arg(4, 3),
                arg(4, 2),
                arg(4, 3),
            )),
            "re" => self.path.commands.push(PathCommand::Rect(
                arg(4, 0),
                arg(4, 1),
                arg(4, 0) + arg(4, 2),
                arg(4, 1) + arg(4, 3),
            )),
            "h" => self.path.commands.push(PathCommand::Close),
            "n" => {
                self.path.commands.clear();
                run.supported.push("pdf-code-path");
            }
            "f" | "F" | "f*" => {
                run.operations.push(PdfOperation::Fill {
                    path: self.path.clone(),
                    even_odd: operator == "f*",
                    transform: self.current_transform(pen_x, pen_y),
                    color: self.fill,
                });
                self.path.commands.clear();
                run.supported.push("pdf-code-fill");
            }
            "S" | "s" => {
                if operator == "s" {
                    self.path.commands.push(PathCommand::Close);
                }
                run.operations.push(PdfOperation::Stroke {
                    path: self.path.clone(),
                    style: self.style,
                    transform: self.current_transform(pen_x, pen_y),
                    color: self.line,
                });
                self.path.commands.clear();
                run.supported.push("pdf-code-stroke");
            }
            "b" | "b*" | "B" | "B*" => {
                if matches!(operator, "b" | "b*") {
                    self.path.commands.push(PathCommand::Close);
                }
                let even_odd = operator.ends_with('*');
                let transform = self.current_transform(pen_x, pen_y);
                run.operations.push(PdfOperation::Fill {
                    path: self.path.clone(),
                    even_odd,
                    transform,
                    color: self.fill,
                });
                run.operations.push(PdfOperation::Stroke {
                    path: self.path.clone(),
                    style: self.style,
                    transform,
                    color: self.line,
                });
                self.path.commands.clear();
                run.supported.push("pdf-code-fill-stroke");
            }
            "W" | "W*" => {
                run.operations.push(PdfOperation::Clip {
                    path: self.path.clone(),
                    even_odd: operator == "W*",
                    transform: self.current_transform(pen_x, pen_y),
                });
                self.clip_depth += 1;
                run.supported.push("pdf-code-clip");
            }
            "ri" | "i" | "BMC" | "BDC" | "EMC" | "MP" | "DP" | "BX" | "EX" | "d0" | "d1"
            | "(str)" | "(hex)" => run.supported.push("pdf-code-noop"),
            "BT" | "ET" | "Tj" | "TJ" | "Tf" | "Td" | "TD" | "Tm" | "T*" | "Tc" | "Tw" | "Tz"
            | "TL" | "Tr" | "Ts" | "'" | "\"" => run.unsupported.push("pdf-code-text".into()),
            "Do" => run.unsupported.push("pdf-code-xobject".into()),
            "sh" => run.unsupported.push("pdf-code-shading".into()),
            "gs" => run.unsupported.push("pdf-code-extgstate".into()),
            "cs" | "CS" | "sc" | "SC" | "scn" | "SCN" => {
                run.unsupported.push("pdf-code-colorspace".into())
            }
            other => run.unsupported.push(format!("pdf-code-op-{other}")),
        }
    }
}

enum Token {
    Number(f32),
    Array(Vec<f32>),
    Name,
    Operator(String),
}

fn lex(bytes: &[u8]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut index = 0;
    let whitespace = |byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x00);
    let delimiter = |byte| {
        matches!(
            byte,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    };
    while index < bytes.len() {
        let byte = bytes[index];
        if whitespace(byte) {
            index += 1;
        } else if byte == b'%' {
            while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                index += 1;
            }
        } else if byte == b'[' {
            index += 1;
            let mut values = Vec::new();
            while index < bytes.len() && bytes[index] != b']' {
                if whitespace(bytes[index]) {
                    index += 1;
                    continue;
                }
                let start = index;
                while index < bytes.len() && !whitespace(bytes[index]) && bytes[index] != b']' {
                    index += 1;
                }
                if let Ok(value) = String::from_utf8_lossy(&bytes[start..index]).parse() {
                    values.push(value);
                }
            }
            index += usize::from(index < bytes.len());
            tokens.push(Token::Array(values));
        } else if byte == b'(' {
            let mut depth = 1;
            index += 1;
            while index < bytes.len() && depth > 0 {
                match bytes[index] {
                    b'\\' => index += 1,
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                index += 1;
            }
            tokens.push(Token::Operator("(str)".into()));
        } else if byte == b'<' {
            index += 1;
            while index < bytes.len() && bytes[index] != b'>' {
                index += 1;
            }
            index += usize::from(index < bytes.len());
            tokens.push(Token::Operator("(hex)".into()));
        } else if byte == b'/' {
            index += 1;
            while index < bytes.len() && !whitespace(bytes[index]) && !delimiter(bytes[index]) {
                index += 1;
            }
            tokens.push(Token::Name);
        } else if matches!(byte, b')' | b']' | b'>' | b'{' | b'}') {
            index += 1;
        } else {
            let start = index;
            while index < bytes.len() && !whitespace(bytes[index]) && !delimiter(bytes[index]) {
                index += 1;
            }
            let text = String::from_utf8_lossy(&bytes[start..index]);
            if let Ok(value) = text.parse() {
                tokens.push(Token::Number(value));
            } else {
                tokens.push(Token::Operator(text.into_owned()));
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_and_transform_persist_across_fragments() {
        let mut graphics = PdfGraphics::default();
        graphics.run(b"2 0 0 2 0 0 cm", 0, 0);
        graphics.run(b"0 0 m", 0, 0);
        graphics.run(b"10 20 l", 0, 0);
        let run = graphics.run(b"S", 0, 0);
        let PdfOperation::Stroke {
            path, transform, ..
        } = &run.operations[0]
        else {
            panic!("expected stroke");
        };
        assert_eq!(path.commands.len(), 2);
        assert_eq!(
            (transform.a, transform.d, transform.e, transform.f),
            (2.0, -2.0, 72.0, 72.0)
        );
    }

    #[test]
    fn restore_balances_nested_clip() {
        let mut graphics = PdfGraphics::default();
        graphics.run(b"q", 0, 0);
        graphics.run(b"0 0 10 10 re W n", 0, 0);
        let run = graphics.run(b"Q", 0, 0);
        assert!(matches!(run.operations.as_slice(), [PdfOperation::PopClip]));
    }
}
