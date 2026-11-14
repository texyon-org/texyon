//! The random numbers of `\\uniformdeviate` and `\\normaldeviate`, as in MetaPost.

/// `SPEC_LOG[k]` is `2^27 * ln(1 / (1 - 2^-k))`, rounded.
const SPEC_LOG: [i32; 29] = [
    0, 93_032_640, 38_612_034, 17_922_280, 8_662_214, 4_261_238, 2_113_709, 1_052_693, 525_315,
    262_400, 131_136, 65_552, 32_772, 16_385, 8192, 4096, 2048, 1024, 512, 256, 128, 64, 32, 16, 8,
    4, 2, 1, 1,
];

/// The random number generator and the seed it started from.
#[derive(Clone)]
pub(crate) struct Random {
    /// `\\randomseed`.
    pub seed: i32,
    randoms: [i32; 55],
    j: u8,
}

impl Default for Random {
    fn default() -> Self {
        Random {
            seed: 0,
            randoms: [0; 55],
            j: 0,
        }
    }
}

/// The sign of `a*b - c*d`, computed without overflow.
fn ab_vs_cd(mut a: i32, mut b: i32, mut c: i32, mut d: i32) -> i32 {
    if a < 0 {
        a = -a;
        b = -b;
    }
    if c < 0 {
        c = -c;
        d = -d;
    }

    if d <= 0 {
        if b >= 0 {
            return if (a == 0 || b == 0) && (c == 0 || d == 0) {
                0
            } else {
                1
            };
        }
        if d == 0 {
            return if a == 0 { 0 } else { -1 };
        }
        std::mem::swap(&mut a, &mut c);
        let tmp = -b;
        b = -d;
        d = tmp;
    } else if b <= 0 {
        if b < 0 && a > 0 {
            return -1;
        }
        return if c == 0 { 0 } else { -1 };
    }

    loop {
        let q = a / d;
        let r = c / b;
        if q != r {
            return if q > r { 1 } else { -1 };
        }

        let q = a % d;
        let r = c % b;
        if r == 0 {
            return if q == 0 { 0 } else { 1 };
        }
        if q == 0 {
            return -1;
        }

        a = b;
        b = q;
        c = d;
        d = r;
    }
}

/// `p/q` as a 28-bit fraction (1.0 == 0x10000000), saturating at |p/q| >= 8.
fn make_frac(mut p: i32, mut q: i32) -> i32 {
    let mut negative = p < 0;
    if negative {
        p = -p;
    }
    if q <= 0 {
        q = -q;
        negative = !negative;
    }

    let n = p / q;
    p %= q;
    if n >= 8 {
        return if negative { -0x7FFF_FFFF } else { 0x7FFF_FFFF };
    }

    let n = (n - 1).wrapping_mul(0x1000_0000);
    let mut f = 1_i32;
    loop {
        let be_careful = p.wrapping_sub(q);
        p = be_careful.wrapping_add(p);
        if p >= 0 {
            f = f + f + 1;
        } else {
            f = f + f;
            p += q;
        }
        if f >= 0x1000_0000 {
            break;
        }
    }

    let be_careful = p.wrapping_sub(q);
    if be_careful.wrapping_add(p) >= 0 {
        f += 1;
    }

    if negative { -(f + n) } else { f + n }
}

/// `q` times the 28-bit fraction `f`. A `q` of `i32::MIN` wraps as in C.
fn take_frac(mut q: i32, mut f: i32) -> i32 {
    let mut negative = f < 0;
    if negative {
        f = -f;
    }
    if q < 0 {
        q = q.wrapping_neg();
        negative = !negative;
    }

    let mut n = if f < 0x1000_0000 {
        0_i32
    } else {
        let nn = f / 0x1000_0000;
        f %= 0x1000_0000;
        if q <= 0x7FFF_FFFF / nn {
            nn * q
        } else {
            0x7FFF_FFFF
        }
    };

    f += 0x1000_0000;
    let mut p = 0x0800_0000_i32;
    if q < 0x4000_0000 {
        loop {
            if f & 1 != 0 {
                p = p.wrapping_add(q) / 2;
            } else {
                p /= 2;
            }
            f /= 2;
            if f == 1 {
                break;
            }
        }
    } else {
        loop {
            if f & 1 != 0 {
                p += q.wrapping_sub(p) / 2;
            } else {
                p /= 2;
            }
            f /= 2;
            if f == 1 {
                break;
            }
        }
    }

    let be_careful = n - 0x7FFF_FFFF;
    if be_careful.wrapping_add(p) > 0 {
        n = 0x7FFF_FFFF_i32.wrapping_sub(p);
    }

    if negative {
        n.wrapping_add(p).wrapping_neg()
    } else {
        n.wrapping_add(p)
    }
}

/// `ln(x / 2^30) * 2^24` for positive `x`.
fn m_log(mut x: i32) -> i32 {
    // TeX reports an error and gives 0 here. norm_rand never passes such an x.
    if x <= 0 {
        return 0;
    }

    let mut y = 1_302_456_860_i32;
    let mut z = 6_581_195_i32;
    while x < 0x4000_0000 {
        x += x;
        y -= 93_032_639;
        z -= 48_782;
    }

    y += z / 65536;
    let mut k: usize = 2;
    while x > 0x4000_0004 {
        let mut z = ((x - 1) >> k) + 1;
        while x < 0x4000_0000 + z {
            z = (z + 1) / 2;
            k += 1;
        }
        y += SPEC_LOG[k];
        x -= z;
    }

    y / 8
}

impl Random {
    fn new_randoms(&mut self) {
        for k in 0_usize..24 {
            let x = self.randoms[k].wrapping_sub(self.randoms[k + 31]);
            self.randoms[k] = if x < 0 { x + 0x1000_0000 } else { x };
        }
        for k in 24_usize..55 {
            let x = self.randoms[k].wrapping_sub(self.randoms[k - 24]);
            self.randoms[k] = if x < 0 { x + 0x1000_0000 } else { x };
        }
        self.j = 54;
    }

    fn next_random(&mut self) -> i32 {
        if self.j == 0 {
            self.new_randoms();
        } else {
            self.j -= 1;
        }
        self.randoms[self.j as usize]
    }

    /// Restarts the sequence from `seed`.
    pub(crate) fn reseed(&mut self, seed: i32) {
        self.seed = seed;
        let mut j = seed.wrapping_abs();
        while j >= 0x1000_0000 {
            j /= 2;
        }

        let mut k = 1_i32;
        for i in 0_usize..55 {
            let jj = k;
            k = j.wrapping_sub(k);
            j = jj;
            if k < 0 {
                k += 0x1000_0000;
            }
            self.randoms[(i * 21) % 55] = j;
        }

        self.new_randoms();
        self.new_randoms();
        self.new_randoms();
    }

    /// TeX's `unif_rand`: a uniform random integer between 0 and `x` (exclusive, sign of `x`).
    pub(crate) fn unif_rand(&mut self, x: i32) -> i32 {
        let r = self.next_random();
        let y = take_frac(x.wrapping_abs(), r);
        if y == x.wrapping_abs() {
            0
        } else if x > 0 {
            y
        } else {
            -y
        }
    }

    /// TeX's `norm_rand`: a normally distributed random integer.
    pub(crate) fn norm_rand(&mut self) -> i32 {
        loop {
            let (x, u) = loop {
                let r = self.next_random() - 0x0800_0000;
                let x = take_frac(112429, r);
                let u = self.next_random();
                if x.abs() < u {
                    break (x, u);
                }
            };

            let xi = make_frac(x, u);
            let l = 139_548_960_i32 - m_log(u);
            if ab_vs_cd(1024, l, xi, xi) >= 0 {
                return xi;
            }
        }
    }
}
