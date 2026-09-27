//! Minimal XDV writer for tests.

const NUMERATOR: u32 = 25_400_000;
const DENOMINATOR: u32 = 473_628_672;

pub(crate) struct XdvBuilder {
    bytes: Vec<u8>,
    mag: u32,
    pages: u16,
    last_bop: i32,
}

impl XdvBuilder {
    pub(crate) fn new(mag: u32) -> Self {
        let mut bytes = vec![247, 7];
        for value in [NUMERATOR, DENOMINATOR, mag] {
            bytes.extend(value.to_be_bytes());
        }
        bytes.push(0);
        Self {
            bytes,
            mag,
            pages: 0,
            last_bop: -1,
        }
    }

    /// Start a page, returning its absolute begin-page offset.
    pub(crate) fn begin_page(&mut self) -> u32 {
        let start = u32::try_from(self.bytes.len()).unwrap();
        self.bytes.push(139);
        self.bytes.extend([0; 40]);
        self.bytes.extend(self.last_bop.to_be_bytes());
        self.last_bop = i32::try_from(start).unwrap();
        self.pages += 1;
        start
    }

    pub(crate) fn end_page(&mut self) -> &mut Self {
        self.bytes.push(140);
        self
    }

    pub(crate) fn special(&mut self, text: &str) -> &mut Self {
        self.bytes.push(239);
        self.bytes.push(u8::try_from(text.len()).unwrap());
        self.bytes.extend(text.as_bytes());
        self
    }

    /// `set_rule`, which also advances the cursor by `width`.
    pub(crate) fn set_rule(&mut self, height: i32, width: i32) -> &mut Self {
        self.bytes.push(132);
        self.bytes.extend(height.to_be_bytes());
        self.bytes.extend(width.to_be_bytes());
        self
    }

    pub(crate) fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.bytes.extend(bytes);
        self
    }

    pub(crate) fn finish(mut self) -> Vec<u8> {
        let postamble = u32::try_from(self.bytes.len()).unwrap();
        self.bytes.push(248);
        self.bytes
            .extend(u32::try_from(self.last_bop.max(0)).unwrap().to_be_bytes());
        for value in [NUMERATOR, DENOMINATOR, self.mag] {
            self.bytes.extend(value.to_be_bytes());
        }
        self.bytes.extend([0; 8]);
        self.bytes.extend(2u16.to_be_bytes());
        self.bytes.extend(self.pages.to_be_bytes());
        self.bytes.push(249);
        self.bytes.extend(postamble.to_be_bytes());
        self.bytes.push(7);
        self.bytes.extend([223; 4]);
        self.bytes
    }

    /// One page holding `specials` in order.
    pub(crate) fn specials_page(specials: &[&str]) -> Vec<u8> {
        let mut xdv = Self::new(1000);
        xdv.begin_page();
        for special in specials {
            xdv.special(special);
        }
        xdv.end_page();
        xdv.finish()
    }
}
