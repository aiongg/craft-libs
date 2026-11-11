use super::*;

#[test]
fn garbage_never_panics() {
    let mut d = Decoder::new();
    let mut x = 0xACu32;
    for len in [0usize, 5, 8, 100, 2000] {
        for _ in 0..200 {
            let mut f: Vec<u8> = (0..len)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 17;
                    x ^= x << 5;
                    x as u8
                })
                .collect();
            if len >= 6 {
                f[0] = 0x0B;
                f[1] = 0x77;
                f[4] &= 0x3F;
                f[5] = (f[5] & 7) | (8 << 3);
            }
            let _ = d.decode(&f);
        }
    }
}

/// MSB-first bit writer for synthetic E-AC-3 syncframes.
#[derive(Default)]
struct W {
    b: Vec<u8>,
    n: usize,
}

impl W {
    fn put(&mut self, bits: u32, v: u32) {
        for i in (0..bits).rev() {
            if self.n.is_multiple_of(8) {
                self.b.push(0);
            }
            let bit = ((v >> i) & 1) as u8;
            *self.b.last_mut().unwrap() |= bit << (7 - self.n % 8);
            self.n += 1;
        }
    }
}

/// An E-AC-3 syncframe of `words` 16-bit words: syncinfo and a bsi with no optional metadata
/// (fscod < 3 with `numblkscod`), then `audio` writes the rest; zero padded.
fn eac3_frame(strmtyp: u32, substreamid: u32, words: u32, numblkscod: u32, acmod: u32, audio: impl FnOnce(&mut W)) -> Vec<u8> {
    let mut w = W::default();
    w.put(16, 0x0B77);
    w.put(2, strmtyp);
    w.put(3, substreamid);
    w.put(11, words - 1);
    w.put(2, 0); // 48 kHz
    w.put(2, numblkscod);
    w.put(3, acmod);
    w.put(1, 0); // lfeon
    w.put(5, 16); // bsid
    w.put(5, 31); // dialnorm
    w.put(1, 0); // compre
    if strmtyp == 1 {
        w.put(1, 0); // chanmape
    }
    w.put(2, 0); // mixmdate, infomdate
    if strmtyp == 0 && numblkscod != 3 {
        w.put(1, 0); // convsync
    }
    w.put(1, 0); // addbsie
    audio(&mut w);
    let mut b = w.b;
    b.resize(words as usize * 2, 0);
    b
}

/// A mono E-AC-3 syncframe of one block (256 samples) that sends exponents (all 10, bandwidth
/// code 0) when `new_exps`, else reuses the last ones. Zero SNR offsets allocate no bits: every
/// coefficient is dither.
pub(crate) fn single_block_eac3(new_exps: bool) -> Vec<u8> {
    eac3_frame(0, 0, 32, 0, 1, |w| {
        // (expstre and ahte are not sent with fewer than six blocks)
        w.put(2, 0); // snroffststr
        w.put(8, 0); // transproce … spxattene
        w.put(2, new_exps as u32); // chexpstr[0]: D15 or reuse
        w.put(1, 0); // convexpstre
        w.put(10, 0); // frmcsnroffst, frmfsnroffst
        w.put(1, 0); // dynrnge
        w.put(1, 0); // spxinu
        if new_exps {
            w.put(6, 0); // chbwcod → 73 coefficients
            w.put(4, 10); // absolute exponent
            for _ in 0..24 {
                w.put(7, 62); // three zero deltas
            }
            w.put(2, 0); // gainrng
        }
        w.put(1, 0); // convsnroffste
    })
}

#[test]
fn eac3_single_block_frames_reuse_exponents_across_syncframes() {
    // the first sends exponents, the second reuses them
    let (first, second) = (single_block_eac3(true), single_block_eac3(false));
    let mut d = Decoder::new();
    let a = d.decode(&first).unwrap();
    assert_eq!((a.header.samples(), a.channels.len(), a.channels[0].len()), (256, 1, 256));
    let b = d.decode(&second).unwrap();
    let peak = |c: &[f32]| c.iter().fold(0f32, |m, v| m.max(v.abs()));
    assert!(peak(&b.channels[0]) > 1e-5 && peak(&b.channels[0]) < 0.1, "{}", peak(&b.channels[0]));
    // after a reset there are no exponents to reuse: no coefficients, silence
    d.reset();
    let c = d.decode(&second).unwrap();
    assert_eq!(peak(&c.channels[0]), 0.0);
}
