type R<T> = Result<T, &'static str>;

const UNITY: f64 = 65536.0;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Item {
    Char { byte: u8, other: bool },
    Dimen(i32),
    Count(i32),
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Font {
    pub quad: i32,
    pub x_height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Fixed,
    Float,
}

pub(crate) struct Env<'a> {
    pub font: Font,
    pub radians: bool,
    pub format: Format,
    pub constant: &'a dyn Fn(&[u8]) -> Option<Vec<u8>>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Outcome {
    pub result: Vec<u8>,
    pub units: bool,
}

pub(crate) fn print_scaled(mut s: i32) -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    if s < 0 {
        out.push(b'-');
        s = s.wrapping_neg();
    }
    out.extend_from_slice((s / 0x1_0000).to_string().as_bytes());
    out.push(b'.');
    s = 10 * (s % 0x1_0000) + 5;
    let mut delta: i32 = 10;
    loop {
        if delta > 0x1_0000 {
            s = s + 0x8000 - 50000;
        }
        out.push(b'0' + (s / 0x1_0000) as u8);
        s = 10 * (s % 0x1_0000);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    out
}

pub(crate) fn fixed(v: f64) -> R<Vec<u8>> {
    let sp = (v * UNITY).round();
    if !sp.is_finite() || sp.abs() >= 1_073_741_824.0 {
        return Err("dimension too large");
    }
    Ok(print_scaled(sp as i32))
}

pub(crate) fn float(v: f64) -> Vec<u8> {
    if v.is_nan() {
        return b"3Y0.0e0]".to_vec();
    }
    if v.is_infinite() {
        return if v > 0.0 {
            b"4Y0.0e0]".to_vec()
        } else {
            b"5Y0.0e0]".to_vec()
        };
    }
    if v == 0.0 {
        return b"0Y0.0e0]".to_vec();
    }
    let text = format!("{:.10e}", v.abs());
    let (mantissa, exponent) = text.split_once('e').expect("exponent");
    let mantissa = mantissa.trim_end_matches('0');
    let mantissa = if mantissa.ends_with('.') {
        format!("{mantissa}0")
    } else {
        mantissa.to_owned()
    };
    format!("{}Y{mantissa}e{exponent}]", if v > 0.0 { 1 } else { 2 }).into_bytes()
}

pub(crate) fn parse_number(text: &[u8]) -> Option<f64> {
    if text.len() > 2 && text[1] == b'Y' && text.last() == Some(&b']') {
        let body = std::str::from_utf8(&text[2..text.len() - 1]).ok()?;
        return match text[0] {
            b'0' => Some(0.0),
            b'1' => body.parse().ok(),
            b'2' => body.parse::<f64>().ok().map(|v| -v),
            b'3' => Some(f64::NAN),
            b'4' => Some(f64::INFINITY),
            b'5' => Some(f64::NEG_INFINITY),
            _ => None,
        };
    }
    let s = std::str::from_utf8(text).ok()?;
    let mut negative = false;
    let mut rest = s;
    while let Some(sign) = rest.chars().next().filter(|c| *c == '-' || *c == '+') {
        negative ^= sign == '-';
        rest = &rest[1..];
    }
    if rest.is_empty()
        || !rest.starts_with(|c: char| c.is_ascii_digit() || c == '.')
        || !rest
            .bytes()
            .all(|c| c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'-' | b'+'))
    {
        return None;
    }
    let v: f64 = if rest.starts_with('.') {
        format!("0{rest}").parse().ok()?
    } else {
        rest.strip_suffix('.').unwrap_or(rest).parse().ok()?
    };
    Some(if negative { -v } else { v })
}

/// `text` is what pgf would hold when that is not the fixed-point form of `v`.
#[derive(Clone, Debug)]
enum Value {
    Num { v: f64, text: Option<Vec<u8>> },
    List(Vec<Value>),
}

impl Value {
    fn of(v: f64) -> Value {
        Value::Num { v, text: None }
    }

    fn texted(v: f64, text: &[u8]) -> Value {
        Value::Num {
            v,
            text: Some(text.to_vec()),
        }
    }

    fn num(&self) -> R<f64> {
        match self {
            Value::Num { v, .. } => Ok(*v),
            Value::List(_) => Err("list where a number was expected"),
        }
    }

    fn list(&self, n: usize) -> R<Vec<f64>> {
        match self {
            Value::List(items) if n == 0 || items.len() == n => {
                items.iter().map(Value::num).collect()
            }
            _ => Err("wrong argument count"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Less,
    Greater,
    Equal,
    NotEqual,
    NotLess,
    NotGreater,
    And,
    Or,
    Question,
    Colon,
    Comma,
    Factorial,
    Degrees,
    Neg,
    Not,
    Open,
    Call(Func),
}

impl Op {
    fn precedence(self) -> i32 {
        match self {
            Op::Add | Op::Subtract | Op::Neg => 500,
            Op::Multiply | Op::Divide => 700,
            Op::Power => 900,
            Op::Less | Op::Greater | Op::Equal | Op::NotEqual | Op::NotLess | Op::NotGreater => 250,
            Op::And | Op::Or => 200,
            Op::Question => 100,
            Op::Colon => 101,
            Op::Comma => 10,
            Op::Factorial => 800,
            Op::Degrees => 600,
            Op::Not => 1000,
            Op::Open => 5,
            Op::Call(_) => 1000,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Incoming {
    Op(Op),
    Close,
    End,
}

impl Incoming {
    fn precedence(self) -> i32 {
        match self {
            Incoming::Op(op) => op.precedence(),
            Incoming::Close => 4,
            Incoming::End => 0,
        }
    }
}

macro_rules! functions {
    ($($name:ident = $text:literal,)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub(crate) enum Func { $($name,)* }

        impl Func {
            pub(crate) fn from_name(name: &[u8]) -> Option<Func> {
                match name {
                    $($text => Some(Func::$name),)*
                    _ => None,
                }
            }
        }

    };
}

functions! {
    Add = b"add",
    Subtract = b"subtract",
    Neg = b"neg",
    Multiply = b"multiply",
    Divide = b"divide",
    Pow = b"pow",
    Reciprocal = b"reciprocal",
    Abs = b"abs",
    Sign = b"sign",
    Sqrt = b"sqrt",
    Exp = b"exp",
    Ln = b"ln",
    Log10 = b"log10",
    Log2 = b"log2",
    Veclen = b"veclen",
    Max = b"max",
    Min = b"min",
    Round = b"round",
    Floor = b"floor",
    Ceil = b"ceil",
    Int = b"int",
    Frac = b"frac",
    Mod = b"mod",
    ModPositive = b"Mod",
    Div = b"div",
    Factorial = b"factorial",
    Sin = b"sin",
    Cos = b"cos",
    Tan = b"tan",
    Sec = b"sec",
    Cosec = b"cosec",
    Cot = b"cot",
    Asin = b"asin",
    Acos = b"acos",
    Atan = b"atan",
    Atan2 = b"atan2",
    Deg = b"deg",
    Rad = b"rad",
    Sinh = b"sinh",
    Cosh = b"cosh",
    Tanh = b"tanh",
    Less = b"less",
    Greater = b"greater",
    Equal = b"equal",
    NotEqual = b"notequal",
    NotLess = b"notless",
    NotGreater = b"notgreater",
    And = b"and",
    Or = b"or",
    Not = b"not",
    IfThenElse = b"ifthenelse",
    Pi = b"pi",
    E = b"e",
    True = b"true",
    False = b"false",
}

fn truth(b: bool) -> Value {
    if b {
        Value::texted(1.0, b"1")
    } else {
        Value::texted(0.0, b"0")
    }
}

struct Parser<'a> {
    items: &'a [(Item, bool)],
    env: &'a Env<'a>,
    operands: Vec<Value>,
    operations: Vec<Op>,
    last: Option<Value>,
    units: bool,
}

const UNITS: [(&[u8; 2], f64); 9] = [
    (b"pt", 1.0),
    (b"in", 72.27),
    (b"pc", 12.0),
    (b"cm", 72.27 / 2.54),
    (b"mm", 72.27 / 25.4),
    (b"bp", 72.27 / 72.0),
    (b"dd", 1238.0 / 1157.0),
    (b"cc", 14856.0 / 1157.0),
    (b"sp", 1.0 / UNITY),
];

impl Parser<'_> {
    fn at(&self, i: usize) -> Item {
        self.items.get(i).map_or(
            Item::Char {
                byte: b'@',
                other: true,
            },
            |&(item, _)| item,
        )
    }

    fn char_at(&self, i: usize) -> Option<u8> {
        match self.at(i) {
            Item::Char { byte: c, .. } => Some(c),
            _ => None,
        }
    }

    fn space_before(&self, i: usize) -> bool {
        self.items.get(i).is_some_and(|&(_, space)| space)
    }

    fn push_result(&mut self, value: Value) {
        self.operands.push(value.clone());
        self.last = Some(value);
    }

    fn pop(&mut self) -> R<Value> {
        self.operands.pop().ok_or("operand stack underflow")
    }

    fn trig_in(&self, x: f64) -> f64 {
        if self.env.radians { x } else { x.to_radians() }
    }

    fn trig_out(&self, x: f64) -> f64 {
        if self.env.radians { x } else { x.to_degrees() }
    }

    fn call(&self, func: Func, arg: Value) -> R<Value> {
        let x = || arg.num();
        let two = || arg.list(2).map(|v| (v[0], v[1]));
        let nonzero = |b: f64| {
            if b == 0.0 {
                Err("division by zero")
            } else {
                Ok(b)
            }
        };
        Ok(match func {
            Func::Add => {
                let (a, b) = two()?;
                Value::of(a + b)
            }
            Func::Subtract => {
                let (a, b) = two()?;
                Value::of(a - b)
            }
            Func::Multiply => {
                let (a, b) = two()?;
                Value::of(a * b)
            }
            Func::Divide => {
                let (a, b) = two()?;
                Value::of(a / nonzero(b)?)
            }
            Func::Pow => {
                let (a, b) = two()?;
                Value::of(a.powf(b))
            }
            Func::Neg => Value::of(-x()?),
            Func::Reciprocal => Value::of(1.0 / nonzero(x()?)?),
            Func::Abs => Value::of(x()?.abs()),
            Func::Sign => {
                let v = x()?;
                if v > 0.0 {
                    Value::texted(1.0, b"1")
                } else if v == 0.0 {
                    Value::texted(0.0, b"0")
                } else {
                    Value::texted(-1.0, b"-1")
                }
            }
            Func::Sqrt => Value::of(x()?.sqrt()),
            Func::Exp => Value::of(x()?.exp()),
            Func::Ln => Value::of(x()?.ln()),
            Func::Log10 => Value::of(x()?.log10()),
            Func::Log2 => Value::of(x()?.log2()),
            Func::Veclen => {
                let (a, b) = two()?;
                Value::of(a.hypot(b))
            }
            Func::Max | Func::Min => {
                let items = arg.list(0)?;
                let pick = if func == Func::Max {
                    f64::max
                } else {
                    f64::min
                };
                Value::of(items.into_iter().reduce(pick).ok_or("empty list")?)
            }
            Func::Round => {
                let v = x()?;
                Value::of(if v < 0.0 {
                    -(v.abs() + 0.5).floor()
                } else {
                    (v + 0.5).floor()
                })
            }
            Func::Floor => Value::of(x()?.floor()),
            Func::Ceil => Value::of(x()?.ceil()),
            Func::Int => {
                let v = x()?.trunc();
                Value::texted(v, format!("{}", v as i64).as_bytes())
            }
            Func::Frac => {
                let f = x()?.abs().fract();
                Value::texted(f, &fixed(f)?)
            }
            Func::Mod | Func::ModPositive => {
                let (a, b) = two()?;
                let mut r = a % nonzero(b)?;
                if func == Func::ModPositive && r < 0.0 {
                    r += b;
                }
                Value::of(r)
            }
            Func::Div => {
                let (a, b) = two()?;
                let q = (a / nonzero(b)?).trunc();
                Value::texted(q, format!("{}", q as i64).as_bytes())
            }
            Func::Factorial => {
                let n = x()?.trunc();
                if !(0.0..=170.0).contains(&n) {
                    return Err("factorial range");
                }
                Value::of((1..=n as u32).map(f64::from).product())
            }
            Func::Sin => Value::of(self.trig_in(x()?).sin()),
            Func::Cos => Value::of(self.trig_in(x()?).cos()),
            Func::Tan => Value::of(self.trig_in(x()?).tan()),
            Func::Sec => Value::of(1.0 / self.trig_in(x()?).cos()),
            Func::Cosec => Value::of(1.0 / self.trig_in(x()?).sin()),
            Func::Cot => {
                let r = self.trig_in(x()?);
                Value::of(r.cos() / r.sin())
            }
            Func::Asin => Value::of(self.trig_out(x()?.asin())),
            Func::Acos => Value::of(self.trig_out(x()?.acos())),
            Func::Atan => Value::of(self.trig_out(x()?.atan())),
            Func::Atan2 => {
                let (y, x) = two()?;
                Value::of(self.trig_out(y.atan2(x)))
            }
            Func::Deg => Value::of(x()?.to_degrees()),
            Func::Rad => Value::of(x()?.to_radians()),
            Func::Sinh => Value::of(x()?.sinh()),
            Func::Cosh => Value::of(x()?.cosh()),
            Func::Tanh => Value::of(x()?.tanh()),
            Func::Less => {
                let (a, b) = two()?;
                truth(a < b)
            }
            Func::Greater => {
                let (a, b) = two()?;
                truth(a > b)
            }
            Func::Equal => {
                let (a, b) = two()?;
                truth(a == b)
            }
            Func::NotEqual => {
                let (a, b) = two()?;
                truth(a != b)
            }
            Func::NotLess => {
                let (a, b) = two()?;
                truth(a >= b)
            }
            Func::NotGreater => {
                let (a, b) = two()?;
                truth(a <= b)
            }
            Func::And => {
                let (a, b) = two()?;
                truth(a != 0.0 && b != 0.0)
            }
            Func::Or => {
                let (a, b) = two()?;
                truth(a != 0.0 || b != 0.0)
            }
            Func::Not => truth(x()? == 0.0),
            Func::IfThenElse => match arg {
                Value::List(items) if items.len() == 3 => {
                    let pick = if items[0].num()? != 0.0 { 1 } else { 2 };
                    items[pick].clone()
                }
                _ => return Err("wrong argument count"),
            },
            Func::Pi => Value::texted(std::f64::consts::PI, b"3.141592654"),
            Func::E => Value::texted(std::f64::consts::E, b"2.718281828"),
            Func::True => truth(true),
            Func::False => truth(false),
        })
    }

    fn apply(&mut self, op: Op) -> R<()> {
        let a = self.pop()?;
        let value = match op {
            Op::Open => a,
            Op::Neg => self.call(Func::Neg, a)?,
            Op::Not => self.call(Func::Not, a)?,
            Op::Factorial => self.call(Func::Factorial, a)?,
            Op::Degrees => self.call(Func::Deg, a)?,
            Op::Call(func) => self.call(func, a)?,
            _ => {
                let b = self.pop()?;
                if op == Op::Comma {
                    match (b, a) {
                        (Value::List(mut items), a @ Value::Num { .. }) => {
                            items.push(a);
                            Value::List(items)
                        }
                        (b @ Value::Num { .. }, a @ Value::Num { .. }) => Value::List(vec![b, a]),
                        _ => return Err("nested list"),
                    }
                } else if op == Op::Question {
                    match a {
                        Value::List(mut items) if items.len() == 2 => {
                            items.insert(0, b);
                            self.call(Func::IfThenElse, Value::List(items))?
                        }
                        _ => return Err("malformed conditional"),
                    }
                } else {
                    let pair = Value::List(vec![b, a]);
                    let func = match op {
                        Op::Colon => {
                            self.push_result(pair);
                            return Ok(());
                        }
                        Op::Add => Func::Add,
                        Op::Subtract => Func::Subtract,
                        Op::Multiply => Func::Multiply,
                        Op::Divide => Func::Divide,
                        Op::Power => Func::Pow,
                        Op::Less => Func::Less,
                        Op::Greater => Func::Greater,
                        Op::Equal => Func::Equal,
                        Op::NotEqual => Func::NotEqual,
                        Op::NotLess => Func::NotLess,
                        Op::NotGreater => Func::NotGreater,
                        Op::And => Func::And,
                        Op::Or => Func::Or,
                        _ => unreachable!(),
                    };
                    self.call(func, pair)?
                }
            }
        };
        self.push_result(value);
        Ok(())
    }

    fn process(&mut self, incoming: Incoming) -> R<()> {
        while let Some(&top) = self.operations.last() {
            if incoming.precedence() > top.precedence() {
                break;
            }
            self.operations.pop();
            self.apply(top)?;
            if top == Op::Open && incoming == Incoming::Close {
                break;
            }
        }
        if let Incoming::Op(op) = incoming {
            self.operations.push(op);
        }
        Ok(())
    }

    fn function(&mut self, mut i: usize) -> R<(usize, bool)> {
        let mut name = Vec::new();
        while let Item::Char { byte: c, .. } = self.at(i) {
            if !(c == b'_' || c.is_ascii_alphanumeric())
                || (!name.is_empty() && self.space_before(i))
            {
                break;
            }
            name.push(c);
            i += 1;
        }
        let Some(func) = Func::from_name(&name) else {
            let text = (self.env.constant)(&name).ok_or("unsupported function")?;
            let v = parse_number(&text).ok_or("constant is not a number")?;
            self.push_result(Value::texted(v, &text));
            return Ok((i, false));
        };
        if matches!(func, Func::Pi | Func::E | Func::True | Func::False) {
            let value = self.call(func, Value::List(Vec::new()))?;
            self.push_result(value);
            return Ok((i, false));
        }
        self.operations.push(Op::Call(func));
        Ok((i, true))
    }

    fn operand(&mut self, mut i: usize) -> R<usize> {
        loop {
            match self.at(i) {
                Item::Char {
                    byte: b'+',
                    other: true,
                } => i += 1,
                Item::Char {
                    byte: b'-',
                    other: true,
                } => {
                    self.operations.push(Op::Neg);
                    i += 1;
                }
                Item::Char { byte: b'!', .. } => {
                    self.operations.push(Op::Not);
                    i += 1;
                }
                Item::Char { byte: b'(', .. } => {
                    self.operations.push(Op::Open);
                    i += 1;
                }
                Item::Char {
                    byte: b'"' | b'[' | b'<',
                    ..
                } => return Err("unsupported prefix operator"),
                Item::Char { byte: c, .. } if c == b'_' || c.is_ascii_alphabetic() => {
                    let (next, operand) = self.function(i)?;
                    if operand {
                        i = next;
                    } else {
                        return Ok(next);
                    }
                }
                _ => return self.number(i),
            }
        }
    }

    fn digits(&self, mut i: usize, radix: u32) -> (Vec<u8>, usize) {
        let mut out = Vec::new();
        while let Some(c) = self.char_at(i) {
            if !(c as char).is_digit(radix) {
                break;
            }
            out.push(c);
            i += 1;
        }
        (out, i)
    }

    fn number(&mut self, mut i: usize) -> R<usize> {
        let mut number: Vec<u8> = Vec::new();
        loop {
            match self.at(i) {
                Item::Dimen(d) => {
                    self.units = true;
                    let factor = if number.is_empty() {
                        1.0
                    } else {
                        parse_number(&number).ok_or("malformed number")?
                    };
                    self.push_result(Value::of(factor * f64::from(d) / UNITY));
                    return Ok(i + 1);
                }
                Item::Count(n) => {
                    if !number.is_empty() {
                        return Err("number before a count register");
                    }
                    if matches!(self.at(i + 1), Item::Dimen(_) | Item::Count(_)) {
                        return Err("box dimension");
                    }
                    self.push_result(Value::texted(f64::from(n), n.to_string().as_bytes()));
                    return Ok(i + 1);
                }
                Item::Char { byte: c, other } if c == b'.' || c.is_ascii_digit() => {
                    if !other {
                        return Err("digit with unusual catcode");
                    }
                    number.push(c);
                    i += 1;
                }
                Item::Char { byte: c, .. } => {
                    if number.is_empty() {
                        return Err("missing operand");
                    }
                    if let Some(n) = self.char_at(i + 1) {
                        let pair = [c, n];
                        let unit = match &pair {
                            b"em" => Some(f64::from(self.env.font.quad) / UNITY),
                            b"ex" => Some(f64::from(self.env.font.x_height) / UNITY),
                            _ => UNITS.iter().find(|(u, _)| **u == pair).map(|&(_, f)| f),
                        };
                        if let Some(unit) = unit {
                            self.units = true;
                            let v = parse_number(&number).ok_or("malformed number")?;
                            self.push_result(Value::of(v * unit));
                            return Ok(i + 2);
                        }
                        if &pair == b"mu" {
                            return Err("math unit");
                        }
                    }
                    if c == b'Y' && number.len() == 1 {
                        return self.float_literal(number[0], i + 1);
                    }
                    if c == b'e' || c == b'E' {
                        return self.exponent(&number, i + 1);
                    }
                    if number == b"0" && matches!(c, b'x' | b'X' | b'b' | b'B') {
                        let radix = if matches!(c, b'x' | b'X') { 16 } else { 2 };
                        let (digits, end) = self.digits(i + 1, radix);
                        let v = i64::from_str_radix(std::str::from_utf8(&digits).unwrap(), radix)
                            .map_err(|_| "base prefix")?;
                        self.push_result(Value::texted(v as f64, v.to_string().as_bytes()));
                        return Ok(end);
                    }
                    if number.len() > 1 && number[0] == b'0' && !number.contains(&b'.') {
                        let v = i64::from_str_radix(std::str::from_utf8(&number).unwrap(), 8)
                            .map_err(|_| "octal number")?;
                        self.push_result(Value::texted(v as f64, v.to_string().as_bytes()));
                        return Ok(i);
                    }
                    let v = parse_number(&number).ok_or("malformed number")?;
                    self.push_result(Value::texted(v, &number));
                    return Ok(i);
                }
            }
        }
    }

    fn exponent(&mut self, mantissa: &[u8], mut i: usize) -> R<usize> {
        let mut negative = false;
        while let Some(s @ (b'+' | b'-')) = self.char_at(i) {
            negative ^= s == b'-';
            i += 1;
        }
        let (digits, end) = self.digits(i, 10);
        let e: i32 = std::str::from_utf8(&digits)
            .unwrap()
            .parse()
            .map_err(|_| "scientific notation")?;
        let m = parse_number(mantissa).ok_or("malformed number")?;
        self.push_result(Value::of(m * 10f64.powi(if negative { -e } else { e })));
        Ok(end)
    }

    fn float_literal(&mut self, flags: u8, mut i: usize) -> R<usize> {
        let mut text = vec![flags, b'Y'];
        loop {
            let c = self.char_at(i).ok_or("float literal")?;
            text.push(c);
            i += 1;
            if c == b']' {
                break;
            }
            if text.len() > 64 {
                return Err("float literal");
            }
        }
        let v = parse_number(&text).ok_or("float literal")?;
        if !v.is_finite() {
            return Err("non-finite float");
        }
        self.push_result(Value::texted(v, &text));
        Ok(i)
    }

    fn run(mut self) -> R<Outcome> {
        let mut i = self.operand(0)?;
        loop {
            let c = self.char_at(i).ok_or("unsupported operator")?;
            let two = match self.char_at(i + 1).map(|n| [c, n]).as_ref() {
                Some(b"==") => Some(Op::Equal),
                Some(b"!=") => Some(Op::NotEqual),
                Some(b">=") => Some(Op::NotLess),
                Some(b"<=") => Some(Op::NotGreater),
                Some(b"&&") => Some(Op::And),
                Some(b"||") => Some(Op::Or),
                _ => None,
            };
            if let Some(op) = two {
                self.process(Incoming::Op(op))?;
                i = self.operand(i + 2)?;
                continue;
            }
            let incoming = match c {
                b'+' => Incoming::Op(Op::Add),
                b'-' => Incoming::Op(Op::Subtract),
                b'*' => Incoming::Op(Op::Multiply),
                b'/' => Incoming::Op(Op::Divide),
                b'^' => Incoming::Op(Op::Power),
                b'<' => Incoming::Op(Op::Less),
                b'>' => Incoming::Op(Op::Greater),
                b'?' => Incoming::Op(Op::Question),
                b':' => Incoming::Op(Op::Colon),
                b',' => Incoming::Op(Op::Comma),
                b'!' => Incoming::Op(Op::Factorial),
                b'r' => Incoming::Op(Op::Degrees),
                b')' => Incoming::Close,
                b'@' => Incoming::End,
                _ => return Err("unsupported operator"),
            };
            self.process(incoming)?;
            match incoming {
                Incoming::End => break,
                Incoming::Close | Incoming::Op(Op::Factorial | Op::Degrees) => i += 1,
                Incoming::Op(_) => i = self.operand(i + 1)?,
            }
        }
        let result = match (self.last.ok_or("no result")?, self.env.format) {
            (Value::List(_), _) => return Err("list result"),
            (
                Value::Num {
                    text: Some(text), ..
                },
                Format::Fixed,
            ) => text,
            (Value::Num { v, text: None }, Format::Fixed) => fixed(v)?,
            (Value::Num { v, .. }, Format::Float) if v.is_finite() => float(v),
            (Value::Num { .. }, Format::Float) => return Err("non-finite result"),
        };
        Ok(Outcome {
            result,
            units: self.units,
        })
    }
}

pub(crate) fn evaluate(items: &[(Item, bool)], env: &Env<'_>) -> R<Outcome> {
    Parser {
        items,
        env,
        operands: Vec::new(),
        operations: Vec::new(),
        last: None,
        units: false,
    }
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(s: &str) -> Vec<(Item, bool)> {
        let mut out = Vec::new();
        let mut space = false;
        for c in s.bytes() {
            if c == b' ' {
                space = true;
                continue;
            }
            out.push((
                Item::Char {
                    byte: c,
                    other: !c.is_ascii_alphabetic(),
                },
                space,
            ));
            space = false;
        }
        out
    }

    fn eval_in(s: &str, format: Format) -> String {
        let constant = |name: &[u8]| (name == b"x").then(|| b"1Y2.0e0]".to_vec());
        let env = Env {
            font: Font::default(),
            radians: false,
            format,
            constant: &constant,
        };
        String::from_utf8(evaluate(&items(s), &env).unwrap().result).unwrap()
    }

    fn eval(s: &str) -> String {
        eval_in(s, Format::Fixed)
    }

    #[test]
    fn print_scaled_matches_tex() {
        assert_eq!(print_scaled(0x1_0000), b"1.0");
        assert_eq!(print_scaled(1), b"0.00002");
        assert_eq!(print_scaled(-0x8000), b"-0.5");
    }

    #[test]
    fn fixed_results() {
        assert_eq!(eval("1+2"), "3.0");
        assert_eq!(eval("2*3+1"), "7.0");
        assert_eq!(eval("-2^2"), "-4.0");
        assert_eq!(eval("(1+2)*3"), "9.0");
        assert_eq!(eval("1/3"), "0.33333");
        assert_eq!(eval("3 "), "3");
        assert_eq!(eval("sqrt(2)"), "1.41422");
        assert_eq!(eval("max(1,5,3)"), "5.0");
        assert_eq!(eval("2 > 1 ? 7 : 8"), "7");
        assert_eq!(eval("int(-3.7)"), "-3");
        assert_eq!(eval("3!"), "6.0");
        assert_eq!(eval("x*2"), "4.0");
        assert_eq!(eval("1.5e2"), "150.0");
        assert_eq!(eval("Mod(-1,-3)"), "-4.0");
        assert_eq!(eval("Mod(-1,3)"), "2.0");
    }

    #[test]
    fn float_results() {
        assert_eq!(eval_in("1+2", Format::Float), "1Y3.0e0]");
        assert_eq!(eval_in("-x/4", Format::Float), "2Y5.0e-1]");
        assert_eq!(eval_in("0", Format::Float), "0Y0.0e0]");
    }
}
