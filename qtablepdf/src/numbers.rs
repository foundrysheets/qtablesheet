//! numbers and statistics types and methods

use crate::qtable::Filter;
use num::Float;
use std::cmp::Ordering;
use std::f64::NAN;

/// f64 extensions trait
pub trait F64Ext<T> {
    fn frmtf64(&self, sig: usize, nan: &str) -> String;
    fn frmtint(&self, nan: &str) -> String;
}

impl F64Ext<f64> for f64 {
    fn frmtf64(&self, mut sig: usize, nan: &str) -> String {
        if self.is_nan() || self.is_infinite() {
            return nan.to_string();
        }

        if sig < 1 {
            sig = 1;
        }

        let mut prc = sig - 1;

        let lgx = self.abs().log10();

        if (lgx >= -3. && lgx <= (sig as f64)) || *self == 0. {
            if *self != 0. {
                let a = prc as isize;
                let b = lgx.trunc() as isize;
                if a <= b {
                    prc = 0;
                } else {
                    prc = (a - b) as usize;
                }
                if lgx < 0. {
                    prc += 1
                }
            }
            format!("{:.1$}", self, prc)
        } else {
            let f1 = format!("{:.1$e}", self, prc);
            let f2: Vec<&str> = f1.split('e').collect();
            match f2[1].starts_with('-') {
                true => {
                    let e = f2[1].trim_start_matches('-');
                    match e.len() {
                        1 => format!("{}e-0{}", f2[0], e),
                        _ => format!("{}e-{}", f2[0], e),
                    }
                }
                false => match f2[1].len() {
                    1 => format!("{}e+0{}", f2[0], f2[1]),
                    _ => format!("{}e+{}", f2[0], f2[1]),
                },
            }
        }
    }

    fn frmtint(&self, nan: &str) -> String {
        if self.is_nan() || self.is_infinite() {
            return nan.to_string();
        }
        format!("{:.0}", self)
    }
}

/// sort function positioning NaN values at the end
fn value_nans_last<T: Float>(a: &T, b: &T) -> Ordering {
    match (a, b) {
        (x, y) if x.is_nan() && y.is_nan() => Ordering::Equal,
        (x, _) if x.is_nan() => Ordering::Greater,
        (_, y) if y.is_nan() => Ordering::Less,
        (_, _) => a.partial_cmp(b).unwrap(),
    }
}

/// vector of f64
#[derive(Debug, Clone, PartialEq)]
pub struct Numbers {
    pub(crate) data: Vec<f64>,
}

impl Numbers {
    /// create new Numbers vector of f64 from vector of String,
    /// skipping invalid values, float_limits, outliers
    pub fn new(vals: &Vec<String>, float_limit: f64, filter_by: &Filter) -> Self {
        let data = vals
            .iter()
            .filter_map(|s| s.parse::<f64>().ok())
            .filter(|v| !v.is_nan())
            .filter(|v| *v < float_limit)
            .collect::<Vec<f64>>();
        let numbers = Numbers::from_f64(data);
        match filter_by {
            Filter::None => numbers,
            Filter::IQR(k) => {
                let kiqr = *k * numbers.iqr();
                let p25 = numbers.p25();
                let p75 = numbers.p75();
                let data = numbers
                    .data
                    .clone()
                    .into_iter()
                    .filter(|x| *x > p25 - kiqr && *x < p75 + kiqr)
                    .collect();
                Numbers::from_f64(data)
            }
            Filter::ZScore(k) => {
                let mea = numbers.mea();
                let std = numbers.std();
                let data = numbers
                    .data
                    .clone()
                    .into_iter()
                    .filter(|x| (*x - mea / std).abs() < *k)
                    .collect();
                Numbers::from_f64(data)
            }
            Filter::Lower(f) => {
                let data = numbers
                    .data
                    .clone()
                    .into_iter()
                    .filter(|x| *x > *f)
                    .collect();
                Numbers::from_f64(data)
            }
            Filter::Upper(g) => {
                let data = numbers
                    .data
                    .clone()
                    .into_iter()
                    .filter(|x| *x < *g)
                    .collect();
                Numbers::from_f64(data)
            }
            Filter::Between(f, g) => {
                let data = numbers
                    .data
                    .clone()
                    .into_iter()
                    .filter(|x| *x > *f && *x < *g)
                    .collect();
                Numbers::from_f64(data)
            }
        }
    }

    pub fn from_f64(mut data: Vec<f64>) -> Self {
        data.sort_by(value_nans_last);
        Numbers { data }
    }

    /// mean of Numbers vector of f64
    pub fn mea(&self) -> f64 {
        let mut i = 0.0;
        let mut mean = 0.0;
        for x in &self.data {
            if !x.is_nan() {
                i += 1.0;
                mean += (x - mean) / i;
            }
        }
        if i > 0.0 {
            mean
        } else {
            NAN
        }
    }

    /// yield of Numbers vector of f64
    pub fn yld(&self, lowlim: &f64, upplim: &f64) -> f64 {
        if lowlim.is_nan() && upplim.is_nan() {
            return NAN;
        }
        let cnt = self.cnt() as usize;
        if cnt == 0 {
            return NAN;
        }
        let mut lo = 0;
        let mut hi = 0;
        if !lowlim.is_nan() {
            lo = self.data.iter().filter(|&x| x < lowlim).count();
        }
        if !upplim.is_nan() {
            hi = self.data.iter().filter(|&x| x > upplim).count();
        }
        100.0 * ((cnt - lo - hi) as f64 / cnt as f64)
    }

    // k
    pub fn k(&self, lsl: &f64, tgt: &f64, usl: &f64) -> f64 {
        let mut k = NAN;
        let mea = &self.mea();
        if !tgt.is_nan() {
            if !lsl.is_nan() && !usl.is_nan() {
                if mea <= tgt {
                    k = (mea - tgt) / (tgt - lsl)
                } else {
                    k = (mea - tgt) / (usl - tgt)
                }
            }
            if lsl.is_nan() && !usl.is_nan() {
                k = (mea - tgt) / (usl - tgt)
            }
            if !lsl.is_nan() && usl.is_nan() {
                k = (mea - tgt) / (tgt - lsl)
            }
        }
        k
    }

    // cpk
    pub fn cpk(&self, lsl: &f64, usl: &f64) -> f64 {
        let mut cpk = NAN;
        let mea = &self.mea();
        let std = &self.std();
        if !lsl.is_nan() && !usl.is_nan() {
            let n = Numbers::from_f64(vec![usl - mea, mea - lsl]);
            cpk = n.min() / (3.0 * std)
        }
        if lsl.is_nan() && !usl.is_nan() {
            cpk = (usl - mea) / (3.0 * std)
        }
        if !lsl.is_nan() && usl.is_nan() {
            cpk = (mea - lsl) / (3.0 * std)
        }
        cpk
    }

    // cp
    pub fn cp(&self, lsl: &f64, usl: &f64) -> f64 {
        let mut cp = NAN;
        let mea = &self.mea();
        let std = &self.std();

        if !lsl.is_nan() && !usl.is_nan() {
            cp = (usl - lsl) / (6.0 * std)
        }
        if lsl.is_nan() && !usl.is_nan() {
            cp = (usl - mea) / (3.0 * std)
        }
        if !lsl.is_nan() && usl.is_nan() {
            cp = (mea - lsl) / (3.0 * std)
        }
        cp
    }

    /// variance of Numbers vector of f64
    pub fn var(&self) -> f64 {
        let mut sum = match &self.data.iter().next() {
            None => NAN,
            Some(x) => **x,
        };
        let mut i = 1.0;
        let mut variance = 0.0;

        for x in &self.data {
            if !x.is_nan() {
                i += 1.0;
                sum += *x;
                let diff = i * x - sum;
                variance += diff * diff / (i * (i - 1.0))
            }
        }
        if i > 1.0 {
            variance / (i - 1.0)
        } else {
            NAN
        }
    }
    /// standard deviation of Numbers vector of f64
    pub fn std(&self) -> f64 {
        self.var().sqrt()
    }
    /// minimum of Numbers vector of f64
    pub fn min(&self) -> f64 {
        match self.data.len() {
            0 => NAN,
            _ => self.data[0],
        }
    }
    /// maximum of Numbers vector of f64
    pub fn max(&self) -> f64 {
        match self.data.len() {
            0 => NAN,
            _ => self.data[self.data.len() - 1],
        }
    }
    /// range(min,max) of Numbers vector of f64
    pub fn range(&self) -> (f64, f64) {
        match self.data.len() {
            0 => (NAN, NAN),
            _ => (self.data[0], self.data[self.data.len() - 1]),
        }
    }
    /// percentile of Numbers vector of f64
    pub fn prc(&self, proc: f64) -> f64 {
        let mut p = proc.abs();

        if p >= 1.0 {
            p = p / 100.0;
        }
        if p >= 1.0 {
            return NAN;
        }

        match self.data.len() {
            0 => NAN,
            1 => self.data[0],
            2 => self.mea(),
            _ => {
                let i = (p * self.cnt()) as usize;
                self.data[i]
            }
        }
    }

    /// p25
    pub fn p25(&self) -> f64 {
        self.prc(0.25)
    }

    /// p75
    pub fn p75(&self) -> f64 {
        self.prc(0.75)
    }

    /// iqr
    pub fn iqr(&self) -> f64 {
        self.prc(0.75) - self.prc(0.25)
    }

    /// median of Numbers vector of f64
    pub fn med(&self) -> f64 {
        match self.data.len() {
            0 => NAN,
            1 => self.data[0],
            2 => self.mea(),
            _ => self.prc(0.5),
        }
    }
    /// count of Numbers vector of f64
    pub fn cnt(&self) -> f64 {
        let l = self.data.len();
        match l {
            0 => NAN,
            _ => l as f64,
        }
    }

    pub fn bins_delta(&self, mut n: usize) -> (Vec<usize>, f64) {
        if n < 3 {
            n = 11;
        }
        let min = self.min();
        let max = self.max();

        let d = (max - min) / 11.0;

        let mut a = min;
        let mut b = min + d;

        let mut bins: Vec<usize> = vec![0; n];
        let mut i = 0;
        for v in self.data.iter() {
            if v > &b && i < bins.len() - 2 {
                a = a + d;
                b = b + d;
                i = i + 1;
            }

            bins[i] += 1;
        }
        if min.is_nan() || d == 0.0 {
            bins = vec![];
        }
        (bins, d)
    }
}
