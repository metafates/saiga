use std::{
    collections::HashSet,
    simd::{cmp::SimdPartialEq, num::SimdUint, u8x16, Simd},
    sync::LazyLock,
};

use swiftty_vte::ansi::c0;

static C0_SET: LazyLock<HashSet<u8>> = LazyLock::new(|| c0::ALL.into_iter().collect());

static C0_SPLATS: LazyLock<[Simd<u8, 16>; 33]> = LazyLock::new(|| c0::ALL.map(u8x16::splat));

fn first_index_of_c0_scalar(haystack: &[u8]) -> Option<usize> {
    for (i, b) in haystack.iter().enumerate() {
        if C0_SET.contains(b) {
            return Some(i);
        }
    }

    None
}

pub fn first_index_of_c0(haystack: &[u8]) -> Option<usize> {
    const LANES: usize = 16;

    let indices = u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    let nulls = u8x16::splat(u8::MAX);

    let mut pos = 0;
    let mut left = haystack.len();

    while left > 0 {
        if left < LANES {
            return first_index_of_c0_scalar(haystack);
        }

        let h = u8x16::from_slice(&haystack[pos..pos + LANES]);

        let index = C0_SPLATS
            .into_iter()
            .filter_map(|splat| {
                let matches = h.simd_eq(splat);

                if matches.any() {
                    let result = matches.select(indices, nulls);

                    Some(result.reduce_min() as usize + pos)
                } else {
                    None
                }
            })
            .min();

        if index.is_some() {
            return index;
        }

        pos += LANES;
        left -= LANES;
    }

    None
}

#[cfg(test)]
mod bench {
    use super::*;

    extern crate test;

    const SAMPLE: &[u8] = b"this is a test for benchmarking processor\x07\x1b[38:2:255:0:255;1m\xD0\x96\xE6\xBC\xA2\xE6\xBC";

    #[bench]
    fn first_index_of_scalar(b: &mut test::Bencher) {
        b.iter(|| {
            first_index_of_c0_scalar(SAMPLE);
        })
    }

    #[bench]
    fn first_index_of_simd(b: &mut test::Bencher) {
        b.iter(|| {
            first_index_of_c0(SAMPLE);
        })
    }
}
