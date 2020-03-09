//! limits table types and methods
extern crate csv;
extern crate itertools_num;
extern crate num;
extern crate printpdf;

use crate::numbers::{F64Ext, Numbers};

use crate::qtable::{Filter, Mark};
use csv::Reader;
use enumflags2::BitFlags;
use std::collections::{BTreeMap, HashMap};
use std::f64::NAN;
use std::fs::File;

/// limitscheck cases
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum YieldOk {
    Yes,
    SpecYieldNot,
    CtrlYieldNot,
    CpkNot,
    NoLimits,
}

/// limits for one parameter: Limits(lsl,tgt,usl)
pub type Limits = BTreeMap<String, f64>;

pub trait LimitsExt<T> {
    fn getnum(&self, key: &str) -> f64;
    fn getstr(&self, key: &str, sig: usize, nan: &str) -> String;
}

impl LimitsExt<Limits> for Limits {
    fn getnum(&self, key: &str) -> f64 {
        if !self.contains_key(key) {
            return NAN;
        }
        self[key]
    }
    fn getstr(&self, key: &str, sig: usize, nan: &str) -> String {
        self.getnum(key).frmtf64(sig, nan)
    }
}

fn marker(yieldok: YieldOk, markit: bool) -> YieldOk {
    if markit {
        yieldok
    } else {
        YieldOk::NoLimits
    }
}

/// lookup table: parameter -> Limits(lsl,tgt,usl)
pub type LimitsTable = BTreeMap<String, Limits>;

/// f64 extensions trait
pub trait LimitsTableExt<T> {
    fn read_limits(&mut self, limpath: &String) -> Result<(), String>;
    fn check_limits(
        &self,
        par: &str,
        numbers: &Numbers,
        spec_yld_lim: f64,
        ctrl_yld_lim: f64,
        cpk_lim: f64,
        marknot: BitFlags<Mark>,
    ) -> (YieldOk, Limits);
    fn get_filter(&self, par: &str, flt: &Filter) -> Filter;
}

impl LimitsTableExt<LimitsTable> for LimitsTable {
    /// add limits from limits file .csv to LimitsTable
    fn read_limits(&mut self, limpath: &String) -> Result<(), String> {
        if *limpath != "".to_string() {
            let limfile = match File::open(limpath) {
                Err(e) => {
                    return Err(format!(
                        "could not open CSV limitfile '{}': '{}'.",
                        limpath, e
                    ));
                }
                Ok(limfile) => limfile,
            };
            let mut rdr: Reader<File> = Reader::from_reader(limfile);

            let mut limcolumns: HashMap<String, usize> = HashMap::new();
            limcolumns.insert("use".to_string(), 9999);
            limcolumns.insert("par".to_string(), 9999);
            limcolumns.insert("lsl".to_string(), 9999);
            limcolumns.insert("tgt".to_string(), 9999);
            limcolumns.insert("usl".to_string(), 9999);
            limcolumns.insert("lcl".to_string(), 9999);
            limcolumns.insert("ucl".to_string(), 9999);

            limcolumns.insert("<fil".to_string(), 9999);
            limcolumns.insert("ter>".to_string(), 9999);

            let headers = match rdr.headers() {
                Ok(record) => record,
                Err(e) => {
                    return Err(format!(
                        "could not read CSV limitfile '{}', '{}'.",
                        limpath, e
                    ));
                }
            };

            for (i, h) in headers.iter().enumerate() {
                let v = h.to_uppercase();
                let w = &v;
                let u: &str = &w;
                match u {
                    "USE" => limcolumns.insert("use".to_string(), i),
                    "PAR" => limcolumns.insert("par".to_string(), i),
                    "LSL" => limcolumns.insert("lsl".to_string(), i),
                    "TGT" => limcolumns.insert("tgt".to_string(), i),
                    "USL" => limcolumns.insert("usl".to_string(), i),
                    "LCL" => limcolumns.insert("lcl".to_string(), i),
                    "UCL" => limcolumns.insert("ucl".to_string(), i),
                    "<FIL" => limcolumns.insert("<fil".to_string(), i),
                    "TER>" => limcolumns.insert("ter>".to_string(), i),
                    _ => None,
                };
            }

            for result in rdr.records() {
                let record = match result {
                    Err(_) => {
                        continue;
                    }
                    Ok(record) => record,
                };

                let mut use_it: bool = false;
                let mut par: &str = Default::default();
                let mut limits = Limits::new();
                limits.insert("lsl".to_string(), NAN);
                limits.insert("tgt".to_string(), NAN);
                limits.insert("usl".to_string(), NAN);
                limits.insert("lcl".to_string(), NAN);
                limits.insert("ucl".to_string(), NAN);

                limits.insert("flt_iqr".to_string(), NAN);
                limits.insert("flt_zsc".to_string(), NAN);
                limits.insert("flt_low".to_string(), NAN);
                limits.insert("flt_upp".to_string(), NAN);

                let mut iqr_next = false;
                let mut zsc_next = false;
                for (i, v) in record.iter().enumerate() {
                    for lim in [
                        "use", "par", "lsl", "tgt", "usl", "lcl", "ucl", "<fil", "ter>",
                    ]
                    .iter()
                    {
                        if limcolumns[*lim] == 9999 {
                            continue;
                        }
                        if i == limcolumns[*lim] {
                            match lim {
                                &"use" => {
                                    if !v.is_empty() {
                                        use_it = true
                                    }
                                }
                                &"par" => par = v,
                                &"<fil" => match v {
                                    "iqr" => iqr_next = true,
                                    "zscore" => zsc_next = true,
                                    _ => {
                                        let f = match v.parse::<f64>() {
                                            Ok(v) => v,
                                            Err(_) => NAN,
                                        };
                                        limits.insert("flt_low".to_string(), f);
                                    }
                                },
                                &"ter>" => {
                                    let f = match v.parse::<f64>() {
                                        Ok(v) => v,
                                        Err(_) => NAN,
                                    };
                                    if iqr_next {
                                        limits.insert("flt_iqr".to_string(), f);
                                    } else {
                                        if zsc_next {
                                            limits.insert("flt_zsc".to_string(), f);
                                        } else {
                                            limits.insert("flt_upp".to_string(), f);
                                        }
                                    }
                                }
                                _ => {
                                    let f = match v.parse::<f64>() {
                                        Ok(v) => v,
                                        Err(_) => NAN,
                                    };
                                    limits.insert(lim.to_string(), f);
                                }
                            }
                        }
                    }
                }
                if use_it {
                    &self.insert(par.to_string(), limits);
                }
            }
        }
        Ok(())
    }

    /// check range of values against LimitsTable by parameter name
    fn check_limits(
        &self,
        par: &str,
        numbers: &Numbers,
        spec_yld_lim: f64,
        ctrl_yld_lim: f64,
        cpk_lim: f64,
        mark: BitFlags<Mark>,
    ) -> (YieldOk, Limits) {
        let range = numbers.range();
        let limits = self.get(par);

        let mut ok: (YieldOk, Limits);

        let mut markit = mark.contains(Mark::SpecYield);

        let lsl: &f64;
        let usl: &f64;
        let lcl: &f64;
        let ucl: &f64;

        let parlim;

        match limits {
            Some(lim) => {
                parlim = lim.clone();
                lsl = lim.get("lsl").unwrap();
                usl = lim.get("usl").unwrap();
                lcl = lim.get("lcl").unwrap();
                ucl = lim.get("ucl").unwrap();
            }
            None => {
                return (YieldOk::NoLimits, Limits::new());
            }
        }

        if numbers.cnt().is_nan() || numbers.cnt() < 1.0 {
            return (YieldOk::NoLimits, limits.unwrap().clone());
        }

        // check spec, if available
        ok = match (lsl.is_nan(), usl.is_nan()) {
            (false, false) => {
                let mut check = *lsl <= range.0 && *usl >= range.1;
                if spec_yld_lim < 100.0 {
                    if numbers.yld(lsl, usl) > spec_yld_lim {
                        check = true
                    } else {
                        check = false
                    }
                }
                match check {
                    true => (marker(YieldOk::Yes, markit), parlim.clone()),
                    false => (marker(YieldOk::SpecYieldNot, markit), parlim.clone()),
                }
            }
            (false, true) => {
                let mut check = *lsl <= range.1;
                if spec_yld_lim < 100.0 {
                    if numbers.yld(lsl, usl) > spec_yld_lim {
                        check = true
                    } else {
                        check = false
                    }
                }
                match check {
                    true => (marker(YieldOk::Yes, markit), parlim.clone()),
                    false => (marker(YieldOk::SpecYieldNot, markit), parlim.clone()),
                }
            }
            (true, false) => {
                let mut check = *usl >= range.0;
                if spec_yld_lim < 100.0 {
                    if numbers.yld(lsl, usl) > spec_yld_lim {
                        check = true
                    } else {
                        check = false
                    }
                }
                match check {
                    true => (marker(YieldOk::Yes, markit), parlim.clone()),
                    false => (marker(YieldOk::SpecYieldNot, markit), parlim.clone()),
                }
            }
            (true, true) => (YieldOk::NoLimits, parlim.clone()),
        };

        // check ctrl, if available
        markit = mark.contains(Mark::ControlYield);
        if ok.0 == YieldOk::NoLimits || ok.0 == YieldOk::Yes {
            ok = match (lcl.is_nan(), ucl.is_nan()) {
                (false, false) => {
                    let mut check = *lcl <= range.0 && *ucl >= range.1;
                    if ctrl_yld_lim < 100.0 {
                        if numbers.yld(lcl, ucl) > ctrl_yld_lim {
                            check = true
                        } else {
                            check = false
                        }
                    }
                    match check {
                        true => (marker(YieldOk::Yes, markit), parlim.clone()),
                        false => (marker(YieldOk::CtrlYieldNot, markit), parlim.clone()),
                    }
                }
                (false, true) => {
                    let mut check = *lcl <= range.1;
                    if ctrl_yld_lim < 100.0 {
                        if numbers.yld(lcl, ucl) > ctrl_yld_lim {
                            check = true
                        } else {
                            check = false
                        }
                    }
                    match check {
                        true => (marker(YieldOk::Yes, markit), parlim.clone()),
                        false => (marker(YieldOk::CtrlYieldNot, markit), parlim.clone()),
                    }
                }
                (true, false) => {
                    let mut check = *ucl >= range.0;
                    if ctrl_yld_lim < 100.0 {
                        if numbers.yld(lcl, ucl) > ctrl_yld_lim {
                            check = true
                        } else {
                            check = false
                        }
                    }
                    match check {
                        true => (marker(YieldOk::Yes, markit), parlim.clone()),
                        false => (marker(YieldOk::CtrlYieldNot, markit), parlim.clone()),
                    }
                }
                (true, true) => (YieldOk::NoLimits, parlim.clone()),
            }
        }

        // check cpk, if available
        markit = mark.contains(Mark::Cpk);
        if ok.0 == YieldOk::NoLimits || ok.0 == YieldOk::Yes {
            ok = if numbers.cpk(lsl, usl) < cpk_lim {
                (marker(YieldOk::CpkNot, markit), parlim.clone())
            } else {
                if lcl.is_nan() && ucl.is_nan() {
                    (marker(YieldOk::NoLimits, markit), parlim.clone())
                } else {
                    (marker(YieldOk::Yes, markit), parlim.clone())
                }
            };
        }

        ok
    }

    fn get_filter(&self, par: &str, flt: &Filter) -> Filter {
        if self.len() < 1 {
            return flt.clone();
        }
        match self.get(par) {
            None => return flt.clone(),
            Some(p) => {
                let iqr = p.get("flt_iqr").unwrap();
                let zsc = p.get("flt_zsc").unwrap();
                let low = p.get("flt_low").unwrap();
                let upp = p.get("flt_upp").unwrap();
                if !iqr.is_nan() {
                    return Filter::IQR(*iqr);
                }
                if !zsc.is_nan() {
                    return Filter::ZScore(*zsc);
                }
                if !low.is_nan() && upp.is_nan() {
                    return Filter::Lower(*low);
                }
                if low.is_nan() && !upp.is_nan() {
                    return Filter::Upper(*upp);
                }
                if !low.is_nan() && !upp.is_nan() {
                    return Filter::Between(*low, *upp);
                }
            }
        }
        flt.clone()
    }
}
