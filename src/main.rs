use fxhash::FxHashMap;
use memmap::MmapOptions;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::Result;
use std::ops::AddAssign;

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
        let dot_offset = buf.iter().position(|x| *x == b'.').unwrap();

        let fixed_int: i16 = unsafe { str::from_utf8_unchecked(&buf[..dot_offset]) }
            .parse()
            .unwrap();

        let fixed_dec: i16 = unsafe { str::from_utf8_unchecked(&buf[dot_offset + 1..]) }
            .parse()
            .unwrap();

        Measurement(fixed_int * 10 + fixed_dec)
    }
}

#[derive(Clone, Copy)]
struct Accumulated {
    min: Measurement,
    max: Measurement,
    total: Measurement,
    count: usize,
}

impl AddAssign<Measurement> for Measurement {
    fn add_assign(&mut self, rhs: Measurement) {
        self.0 += rhs.0;
    }
}

impl AddAssign<Measurement> for Accumulated {
    fn add_assign(&mut self, rhs: Measurement) {
        if rhs < self.min {
            self.min = rhs;
        }
        if rhs > self.max {
            self.max = rhs;
        }
        self.total += rhs;
        self.count += 1;
    }
}

impl AddAssign<Accumulated> for Accumulated {
    fn add_assign(&mut self, rhs: Accumulated) {
        if rhs.min < self.min {
            self.min = rhs.min;
        }
        if rhs.max > self.max {
            self.max = rhs.max;
        }
        self.total += rhs.total;
        self.count += rhs.count;
    }
}

fn main() -> Result<()> {
    let measurements: HashMap<&str, Vec<Measurement>> = HashMap::new();

    let name = if std::env::args().len() == 1 {
        "measurements.txt".to_string()
    } else {
        std::env::args().nth(1).expect("Expected filename")
    };

    let file = File::open(name)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    let buffer = &mmap[..];

    buffer
        .par_split(|&b| b == b'\n')
        .filter(|line| line.len() > 0)
        .fold(FxHashMap::default, |mut acc, line| {
            let offset_opt = line.iter().position(|x| *x == b';');

            let offset = offset_opt.unwrap();

            let name = unsafe { str::from_utf8_unchecked(&line[..offset]) };

            let measurement = Measurement::parse(&line[offset + 1..]);

            *acc.entry(name).or_insert(Accumulated {
                min: measurement,
                max: measurement,
                total: measurement,
                count: 0,
            }) += measurement;
            acc
        })
        .reduce_with(|mut acc1, acc2| {
            for (k, v) in acc2 {
                acc1.entry(k).and_modify(|acc| *acc += v).or_insert(v);
            }
            acc1
        });

    for (name, measurements) in measurements {
        let min = measurements.iter().min().unwrap();
        let avg =
            Measurement(measurements.iter().map(|m| m.0).sum::<i16>() / measurements.len() as i16);
        let max = measurements.iter().max().unwrap();

        println!("{};{};{};{}", name, min, avg, max);
    }

    Ok(())
}
