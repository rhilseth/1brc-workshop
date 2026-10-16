use memmap::MmapOptions;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::Result;

//use rayon::prelude::*;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
struct Measurement(i16);

impl Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (i, d) = ((self.0 / 10).abs(), (self.0 % 10).abs());
        let sign = if self.0.is_negative() { "-" } else { "" };
        write!(f, "{}{}.{}", sign, i, d)
    }
}

fn main() -> Result<()> {
    let mut measurements: HashMap<&str, Vec<Measurement>> = HashMap::new();

    let name = std::env::args().nth(1).expect("Expected filename");
    let file = File::open(name)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    let buffer = &mmap[..];

    for line in buffer.split(|&b| b == b'\n').map(|line| line) {
        let offset_opt = line.iter().position(|x| *x == b';');
        if offset_opt.is_none() {
            continue;
        }
        let offset = offset_opt.unwrap();

        let name = unsafe { str::from_utf8_unchecked(&line[..offset]) };

        let measurement_raw = &line[offset + 1..];
        let dot_offset = measurement_raw.iter().position(|x| *x == b'.').unwrap();

        let fixed_int: i16 = unsafe { str::from_utf8_unchecked(&measurement_raw[..dot_offset]) }
            .parse()
            .unwrap();

        let fixed_dec: i16 =
            unsafe { str::from_utf8_unchecked(&measurement_raw[dot_offset + 1..]) }
                .parse()
                .unwrap();

        let fixed_measurement = fixed_int * 10 + fixed_dec;

        measurements
            .entry(name)
            .or_default()
            .push(Measurement(fixed_measurement));
    }

    for (name, measurements) in measurements {
        let min = measurements.iter().min().unwrap();
        let avg =
            Measurement(measurements.iter().map(|m| m.0).sum::<i16>() / measurements.len() as i16);
        let max = measurements.iter().max().unwrap();

        println!("{};{};{};{}", name, min, avg, max);
    }

    Ok(())
}
