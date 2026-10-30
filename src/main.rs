#![feature(int_from_ascii)]
use fxhash::FxHashMap;
use memmap2::MmapOptions;
use std::fmt::Display;
use std::fs::File;
use std::io::Result;

use rayon::prelude::*;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
struct Measurement(i16);

impl Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (i, d) = ((self.0 / 10).abs(), (self.0 % 10).abs());
        let sign = if self.0.is_negative() { "-" } else { "" };
        write!(f, "{}{}.{}", sign, i, d)
    }
}

impl Measurement {
    fn parse(buf: &[u8]) -> Measurement {
        let dot_offset = memchr::memchr(b'.', &buf).unwrap();
        let fixed_int = i16::from_ascii_bytes(&buf[..dot_offset]).unwrap();
        let fixed_dec = (buf[dot_offset + 1] - b'0') as i16;

        Measurement(fixed_int * 10 + fixed_dec)
    }
}

#[derive(Clone, Copy)]
struct Accumulated {
    min: Measurement,
    max: Measurement,
    total: i32,
    count: usize,
}

#[inline]
fn add_measurement(lhs: &mut Accumulated, rhs: Measurement) {
    if rhs < lhs.min {
        lhs.min = rhs;
    }
    if rhs > lhs.max {
        lhs.max.0 = rhs.0;
    }
    lhs.total += rhs.0 as i32;
    lhs.count += 1;
}

#[inline]
fn add_assign(lhs: &mut Accumulated, rhs: &Accumulated) {
    if rhs.min < lhs.min {
        lhs.min = rhs.min;
    }
    if rhs.max > lhs.max {
        lhs.max = rhs.max;
    }
    lhs.total += rhs.total;
    lhs.count += rhs.count;
}

fn main() -> Result<()> {
    let name = if std::env::args().len() == 1 {
        "measurements.txt".to_string()
    } else {
        std::env::args().nth(1).expect("Expected filename")
    };

    let file = File::open(name)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    let buffer = &mmap[..];

    let mut chunks: Vec<&[u8]> = Vec::new();
    let threads = std::thread::available_parallelism().unwrap().into();

    let chunk_size = buffer.len() / threads;

    let mut next_start = 0;
    for _ in 0..threads {
        let chunk = &buffer[next_start..];
        let mut end = chunk_size;
        let extra = chunk[end..].iter().position(|c| *c == b'\n').unwrap();
        end += extra;
        let chunk = &chunk[..end];
        chunks.push(chunk);
        next_start = end;
    }

    let measurements = chunks
        .par_iter()
        .map(|chunk| {
            let mut acc = FxHashMap::default();
            for line in chunk.split(|&b| b == b'\n').map(|line| line) {
                if line.len() == 0 {
                    continue;
                }
                if let Some(offset) = memchr::memchr(b';', &line) {
                    let name = unsafe { str::from_utf8_unchecked(&line[..offset]) };

                    let measurement = Measurement::parse(&line[offset + 1..]);

                    add_measurement(
                        acc.entry(name).or_insert(Accumulated {
                            min: measurement,
                            max: measurement,
                            total: measurement.0 as i32,
                            count: 0,
                        }),
                        measurement,
                    );
                }
            }
            acc
        })
        .reduce_with(|mut acc1, acc2| {
            for (k, v) in acc2 {
                acc1.entry(k)
                    .and_modify(|acc| add_assign(acc, &v))
                    .or_insert(v);
            }
            acc1
        })
        .unwrap();

    let mut sorted: Vec<(&&str, &Accumulated)> = measurements.iter().collect();
    sorted.sort_by_key(|&(key, _value)| key);

    for (name, accumulated) in sorted {
        let avg = Measurement((accumulated.total / accumulated.count as i32) as i16);
        println!("{};{};{};{}", name, accumulated.min, avg, accumulated.max);
    }

    Ok(())
}
