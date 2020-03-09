//! check input
extern crate chrono;
extern crate csv;
extern crate enumflags2;
extern crate itertools_num;
extern crate num;
extern crate printpdf;

use self::enumflags2::BitFlags;
use crate::data::create_limits_file;
use crate::group::GroupBy;
use crate::pdf::Paper;
use crate::qtable::{default_columns, Align, Column, Filter, Mark, Order, Show};
use crate::sample;
use crate::sample::write_sample_file;
use csv::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct QTableProps {
    pub caption: String,
    pub captioneverypage: bool,
    pub pdffolder: String,
    pub pdffile: String,
    pub pdftimestamp: PDFTimestamp,
    pub paper: Paper,
    pub order: Order,
    pub margin: f64,
    pub fontsize: i64,
    pub align: Align,
    pub show: BitFlags<Show>,
    pub mark: BitFlags<Mark>,
    pub nanstring: String,
    pub sig_digits: usize,
    pub filter: Filter,
    pub float_limit: f64,
    pub spec_yield_limit: f64,
    pub ctrl_yield_limit: f64,
    pub cpk_limit: f64,
    pub group_by: Vec<GroupBy>,
    pub longgroupnames: bool,
    pub histogram_bins: usize,
}

pub fn default_props() -> QTableProps {
    QTableProps {
        caption: "".into(),
        captioneverypage: true,
        paper: Paper::A4Landscape,
        pdffolder: "".to_string(),
        pdffile: "".to_string(),
        pdftimestamp: PDFTimestamp::None,
        order: Order::ByBadGood,
        margin: 7.,
        fontsize: 7,
        align: Align::SpecLimits,
        show: Show::SpecLimits | Show::ControlLimits | Show::Targets,
        mark: Mark::SpecYield | Mark::ControlYield | Mark::Cpk,
        nanstring: "".to_string(),
        sig_digits: 4,
        filter: Filter::None,
        float_limit: std::f64::MAX,
        spec_yield_limit: 100.0,
        ctrl_yield_limit: 100.0,
        cpk_limit: 1.67,
        group_by: vec![],
        longgroupnames: false,
        histogram_bins: 11,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PDFTimestamp {
    None,
    Local,
    UTC,
}

pub fn info(err: String) {
    if err.starts_with("HINT") {
        println!(" {}", err);
    } else {
        println!("ERROR: {}", err);
    }
}

pub fn check_infiles(
    infiles: Vec<String>,
    prognam: &str,
) -> Result<(String, String, String), String> {
    let mut datpath = "";
    let mut limpath = "";
    let mut cfgpath = "";
    let mut want_limits_file = false;

    for filepath in infiles.iter() {
        if filepath == "samples" {
            write_sample_file(".", "dat")?;
            write_sample_file(".", "lim")?;
            write_sample_file(".", "cfg")?;
            std::process::exit(0);
        }
        if filepath == "limits" {
            want_limits_file = true;
            continue;
        }
        if filepath == "help" {
            sample::help(prognam);
            std::process::exit(0);
        }

        let path = Path::new(filepath);
        if !(path.exists() && path.is_file()) {
            return Err(format!("file '{}' is not existent.", filepath));
        }

        let file = match File::open(&path) {
            Err(_) => {
                return Err(format!("CSV file '{}' is not existent.", path.display()));
            }
            Ok(file) => file,
        };

        let mut rdr: Reader<File> = Reader::from_reader(file);

        let mut is_use = false;
        let mut is_par = false;
        let mut is_lim = false;
        let mut is_opt = false;
        let mut is_val = false;

        let headers = match rdr.headers() {
            Ok(record) => record,
            Err(e) => {
                return Err(format!(
                    "could not read CSV file '{}': '{}'.",
                    &path.display(),
                    e
                ));
            }
        };

        for h in headers.iter() {
            let v = h.to_uppercase();
            let w = &v;
            let u: &str = &w;
            match u {
                "USE" => is_use = true,
                "PAR" => is_par = true,
                "LSL" => is_lim = true,
                "USL" => is_lim = true,
                "LCL" => is_lim = true,
                "UCL" => is_lim = true,
                "TGT" => is_lim = true,
                "OPT" => is_opt = true,
                "VAL" => is_val = true,
                _ => (),
            }
        }

        if is_use && is_par && is_lim {
            if limpath == "" {
                limpath = &filepath;
            } else {
                return Err(format!(
                    "can only read one CSV limit file '{}', limit file '{}' cannot be loaded.",
                    &limpath, &filepath
                ));
            }
        } else {
            if is_use && is_opt && is_val {
                if cfgpath == "" {
                    cfgpath = &filepath;
                } else {
                    return Err(format!("can only read one CSV config file '{}', config file '{}' cannot be loaded.", &limpath, &filepath));
                }
            } else {
                if datpath == "" {
                    datpath = &filepath;
                } else {
                    return Err(format!(
                        "can only read one CSV data file '{}', data file '{}' cannot be loaded.",
                        &datpath, &filepath
                    ));
                }
            }
        }
    }

    if datpath == "" {
        return Err(format!(
            "HINT: at least one CSV data file must be provided."
        ));
    }

    if want_limits_file {
        create_limits_file(&datpath.to_string(), 5)?;
        std::process::exit(0);
    }

    Ok((
        datpath.to_string(),
        limpath.to_string(),
        cfgpath.to_string(),
    ))
}

pub fn read_config(cfgpath: &String) -> Result<(QTableProps, Vec<Column>), String> {
    let empty_val_allowed = vec!["pdffolder", "longgroupnames", "nanstring", "groupby"];

    let mut allowed_vals: HashMap<String, Vec<String>> = HashMap::new();

    let mut cfgrdr = Reader::from_reader(sample::SAMPLE_CFG.as_bytes());
    for result in cfgrdr.records() {
        let record = match result {
            Err(_) => {
                continue;
            }
            Ok(record) => record,
        };

        let opt = record.get(1).unwrap().to_string().to_lowercase();
        let val = record.get(2).unwrap().to_string().to_lowercase();
        if !opt.is_empty() && !val.is_empty() {
            if !allowed_vals.contains_key(&opt) {
                allowed_vals.insert(opt.clone(), vec![]);
            }
            allowed_vals.get_mut(&opt).unwrap().push(val);
        }
    }

    let unknown_val = |opt: &str, val: &str| -> String {
        format!(
            "Unknown VAL '{}' for OPT '{}' in configfile '{}'.
       Allowed VAL: {}",
            val,
            opt,
            cfgpath,
            allowed_vals.get(opt).unwrap().join(", ")
        )
    };

    let mut qtableprops = default_props();

    let mut columns: Vec<Column> = vec![];

    if *cfgpath != "".to_string() {
        let mut rdr: Reader<File> = match Reader::from_path(cfgpath) {
            Err(e) => {
                return Err(format!(
                    "cannot not use CSV configfile '{}': '{}'.",
                    cfgpath, e
                ));
            }
            Ok(rdr) => rdr,
        };

        let mut cfgcolumns: HashMap<String, usize> = HashMap::new();
        let mut columnscfg: HashMap<usize, String> = HashMap::new();

        let cfgcolnames = vec!["use", "opt", "val", "nam", "wid", "arg"];

        let headers = match rdr.headers() {
            Ok(record) => record,
            Err(e) => {
                return Err(format!(
                    "could not read CSV configfile '{}', '{}'.",
                    cfgpath, e
                ));
            }
        };

        for (i, h) in headers.iter().enumerate() {
            let v = h.to_uppercase();
            let w = &v;
            let u: &str = &w;

            match u {
                "USE" => {
                    cfgcolumns.insert("use".to_string(), i);
                    columnscfg.insert(i, "use".to_string());
                }
                "OPT" => {
                    cfgcolumns.insert("opt".to_string(), i);
                    columnscfg.insert(i, "opt".to_string());
                }
                "VAL" => {
                    cfgcolumns.insert("val".to_string(), i);
                    columnscfg.insert(i, "val".to_string());
                }
                "NAM" => {
                    cfgcolumns.insert("nam".to_string(), i);
                    columnscfg.insert(i, "nam".to_string());
                }
                "WID" => {
                    cfgcolumns.insert("wid".to_string(), i);
                    columnscfg.insert(i, "wid".to_string());
                }
                "ARG" => {
                    cfgcolumns.insert("arg".to_string(), i);
                    columnscfg.insert(i, "arg".to_string());
                }
                _ => (),
            };
        }

        for v in cfgcolnames.iter() {
            if !cfgcolumns.contains_key(*v) {
                return Err(format!(
                    "column '{}' is missing in CSV configfile '{}'.",
                    v.to_uppercase(),
                    cfgpath
                ));
            }
        }

        let mut maxi = 0;
        for (_, i) in cfgcolumns.iter() {
            if i > &maxi {
                maxi = *i;
            }
        }

        if cfgcolumns.get("arg").unwrap() < &maxi {
            return Err(format!(
                "ARG column in CSV configfile '{}' must be the rightmost column.",
                cfgpath
            ));
        }

        let mut show: BitFlags<Show> = BitFlags::empty();
        let mut mark: BitFlags<Mark> = BitFlags::empty();

        for result in rdr.records() {
            let record = match result {
                Err(_) => {
                    continue;
                }
                Ok(record) => record,
            };

            let optstr = record.get(cfgcolumns["opt"]).unwrap();
            if record.get(cfgcolumns["use"]).unwrap().is_empty() {
                continue;
            }

            if optstr.is_empty() {
                return Err(format!(
                    "column OPT in row '{:?}'
       in CSV configfile '{}' is checked and cannot be empty.",
                    record, cfgpath
                ));
            }

            if record.get(cfgcolumns["val"]).unwrap().is_empty()
                && !empty_val_allowed.contains(&optstr)
            {
                return Err(format!(
                    "column VAL for OPT '{}' in CSV configfile '{}' cannot be empty.
              allowed VAL: {:?}",
                    optstr,
                    cfgpath,
                    allowed_vals.get(optstr).unwrap().join(", ")
                ));
            }

            let mut opt: &str = Default::default();
            let mut val_ori: &str = Default::default();
            let mut nam: &str = Default::default();
            let mut wid: &str = Default::default();
            let mut args: Vec<&str> = Default::default();

            for (i, v) in record.iter().enumerate() {
                for cfg in cfgcolnames.iter() {
                    if i == cfgcolumns[*cfg] {
                        match cfg {
                            &"opt" => {
                                opt = v;
                                if opt == "groupby" {
                                    let mut c = HashMap::new();
                                    for (i, p) in record.iter().enumerate() {
                                        if columnscfg.contains_key(&i)
                                            && (columnscfg.get(&i).unwrap() == "use"
                                                || columnscfg.get(&i).unwrap() == "opt")
                                            || p.is_empty()
                                            || c.contains_key(p)
                                        {
                                            continue;
                                        }
                                        c.insert(p, true);
                                        match p.parse::<usize>() {
                                            Ok(u) => {
                                                qtableprops.group_by.push(GroupBy::ColNumber(u));
                                            }
                                            Err(_) => {
                                                qtableprops
                                                    .group_by
                                                    .push(GroupBy::ColName(p.into()));
                                            }
                                        }
                                    }
                                }
                            }
                            &"val" => val_ori = v,
                            &"nam" => nam = v,
                            &"wid" => wid = v,
                            &"arg" => {
                                for (i, x) in record.iter().enumerate() {
                                    if i < *cfgcolumns.get("arg").unwrap() {
                                        continue;
                                    }
                                    if !x.is_empty() {
                                        args.push(x)
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }

            let opt = opt.to_lowercase();
            let val = val_ori.to_lowercase();

            let opt = opt.trim();
            let val = val.trim();
            let nam = nam.trim().to_string();
            let wid = wid.trim();

            match opt {
                "caption" => {
                    qtableprops.caption = val.to_string();
                }
                "captionwhere" => match val {
                    "firstpage" => qtableprops.captioneverypage = false,
                    "everypage" => qtableprops.captioneverypage = true,
                    _ => return Err(unknown_val(opt, val)),
                },
                "paper" => match val {
                    "a4portrait" => qtableprops.paper = Paper::A4Portrait,
                    "a4landscape" => qtableprops.paper = Paper::A4Landscape,
                    "letterportrait" => qtableprops.paper = Paper::LetterPortrait,
                    "letterlandscape" => qtableprops.paper = Paper::LetterLandscape,
                    _ => return Err(unknown_val(opt, val)),
                },
                "pdffolder" => match val_ori.is_empty() {
                    true => qtableprops.pdffolder = ".".to_string(),
                    false => qtableprops.pdffolder = val_ori.to_string(),
                },
                "pdffile" => match val_ori.is_empty() {
                    true => qtableprops.pdffile = "".to_string(),
                    false => qtableprops.pdffile = val_ori.to_string(),
                },
                "pdftimestamp" => match val {
                    "local" => qtableprops.pdftimestamp = PDFTimestamp::Local,
                    "utc" => qtableprops.pdftimestamp = PDFTimestamp::UTC,
                    _ => qtableprops.pdftimestamp = PDFTimestamp::None,
                },
                "order" => match val {
                    "bybadgood" => qtableprops.order = Order::ByBadGood,
                    "bynumber" => qtableprops.order = Order::ByNumber,
                    _ => return Err(unknown_val(opt, val)),
                },
                "fontsize" => {
                    if !val.is_empty() {
                        qtableprops.fontsize = match val.parse::<i64>() {
                            Ok(v) => v,
                            Err(_) => {
                                let v = 6;
                                println!(
                                    "VAL '{}' for OPT '{}' is invalid, using '{}' instead.",
                                    val, opt, v
                                );
                                v
                            }
                        };
                    }
                }
                "histobins" => {
                    if !val.is_empty() {
                        qtableprops.histogram_bins = match val.parse::<usize>() {
                            Ok(v) => {
                                if v < 3 {
                                    println!(
                                        "number of bins for histogram must be >= 3. '{}' is too low, using default value '11' instead.",
                                        v
                                    );
                                    11
                                } else {
                                    v
                                }
                            }
                            Err(_) => {
                                let v = 11;
                                println!(
                                    "VAL '{}' for OPT '{}' is invalid, using '{}' instead.",
                                    val, opt, v
                                );
                                v
                            }
                        };
                    }
                }
                "filter" => {
                    match val {
                        "iqr" => {
                            let f = match nam.parse::<f64>() {
                                Ok(f) => f,
                                Err(_) => {
                                    println!(
                                        "In outliers iqr: '{}' is not a number, using 1.5 instead",
                                        nam
                                    );
                                    1.5
                                }
                            };
                            qtableprops.filter = Filter::IQR(f);
                        }
                        "zscore" => {
                            let f = match nam.parse::<f64>() {
                                Ok(f) => f,
                                Err(_) => {
                                    println!("In outliers zscore: '{}' is not a number, using 2.5 instead", nam);
                                    1.5
                                }
                            };
                            qtableprops.filter = Filter::ZScore(f);
                        }
                        _ => return Err(unknown_val(opt, val)),
                    }
                }
                "groupnames" => match val {
                    "numbers" => qtableprops.longgroupnames = false,
                    "longnames" => qtableprops.longgroupnames = true,
                    _ => return Err(unknown_val(opt, val)),
                },
                "align" => match val {
                    "control" => qtableprops.align = Align::ControlLimits,
                    "fit" => qtableprops.align = Align::FitValues,
                    "spec" => qtableprops.align = Align::SpecLimits,
                    "target" => qtableprops.align = Align::Targets,
                    _ => return Err(unknown_val(opt, val)),
                },
                "show" => {
                    let valvec: Vec<&str> = val.split('|').collect();
                    for v in valvec.iter() {
                        let vval = v.trim();
                        match vval {
                            "control" => show |= Show::ControlLimits,
                            "spec" => show |= Show::SpecLimits,
                            "target" => show |= Show::Targets,
                            _ => return Err(unknown_val(opt, val)),
                        }
                    }
                    if !show.is_empty() {
                        qtableprops.show = show;
                    }
                }
                "mark" => {
                    let valvec: Vec<&str> = val.split('|').collect();
                    for v in valvec.iter() {
                        let vval = v.trim();
                        match vval {
                            "specyield" => mark |= Mark::SpecYield,
                            "ctrlyield" => mark |= Mark::ControlYield,
                            "cpk" => mark |= Mark::Cpk,
                            _ => return Err(unknown_val(opt, val)),
                        }
                    }
                    if !mark.is_empty() {
                        qtableprops.mark = mark;
                    }
                }
                "nanstring" => {
                    qtableprops.nanstring = val.to_string();
                }
                "sigdigits" => {
                    if !val.is_empty() {
                        qtableprops.sig_digits = match val.parse::<usize>() {
                            Ok(v) => v,
                            Err(_) => {
                                let v = 4;
                                println!(
                                    "VAL '{}' for OPT '{}' is invalid, using '{}' instead.",
                                    val, opt, v
                                );
                                v
                            }
                        };
                    }
                }
                "floatlimit" => {
                    if !val.is_empty() {
                        qtableprops.float_limit = match val.parse::<f64>() {
                            Ok(v) => v.abs(),
                            Err(_) => {
                                let v = std::f64::MAX;
                                println!(
                                    "VAL '{}' for OPT '{}' is invalid, using '{}' instead.",
                                    val, opt, v
                                );
                                v
                            }
                        };
                    }
                }
                "specyieldlimit" => {
                    qtableprops.spec_yield_limit = match val.parse::<f64>() {
                        Ok(v) => {
                            if v.abs() > 100.0 {
                                100.0
                            } else {
                                v.abs()
                            }
                        }
                        Err(_) => {
                            let v = 100.;
                            println!(
                                "VAL '{}' for OPT '{}' is invalid, using '{}' instead.",
                                val, opt, v
                            );
                            v
                        }
                    }
                }
                "ctrlyieldlimit" => {
                    qtableprops.ctrl_yield_limit = match val.parse::<f64>() {
                        Ok(v) => {
                            if v.abs() > 100.0 {
                                100.0
                            } else {
                                v.abs()
                            }
                        }
                        Err(_) => {
                            let v = 100.;
                            println!(
                                "VAL '{}' for OPT '{}' is invalid, using '{}' instead.",
                                val, opt, v
                            );
                            v
                        }
                    }
                }
                "cpklimit" => {
                    qtableprops.cpk_limit = match val.parse::<f64>() {
                        Ok(v) => {
                            if v.abs() > 10.0 {
                                10.0
                            } else {
                                v.abs()
                            }
                        }
                        Err(_) => {
                            let v = 1.67;
                            println!(
                                "VAL '{}' for OPT '{}' is invalid, using '{}' instead.",
                                val, opt, v
                            );
                            v
                        }
                    }
                }
                "groupby" => {
                    // see "opt" above
                }
                "longgroupnames" => {
                    qtableprops.longgroupnames = true;
                }
                "column" => {
                    if !nam.is_empty() && !wid.is_empty() {
                        let w = match wid.parse::<f64>() {
                            Ok(v) => v.abs(),
                            Err(_) => {
                                let v = 7.0;
                                println!(
                                    "WID '{}' for column   '{}' is invalid, using '{}' instead.",
                                    wid, nam, v
                                );
                                v
                            }
                        };
                        match val {
                            "number" => columns.push(Column::Number(nam, w)),
                            "parameter" => columns.push(Column::Parameter(nam, w)),
                            "count" => columns.push(Column::Count(nam, w)),
                            "mean" => columns.push(Column::Mean(nam, w)),
                            "median" => columns.push(Column::Median(nam, w)),
                            "variance" => columns.push(Column::Variance(nam, w)),
                            "stddev" => columns.push(Column::Sdev(nam, w)),
                            "min" => columns.push(Column::Min(nam, w)),
                            "max" => columns.push(Column::Max(nam, w)),
                            "range" => columns.push(Column::Range(nam, w)),
                            "specyield" => columns.push(Column::SpecYield(nam, w)),
                            "ctrlyield" => columns.push(Column::CtrlYield(nam, w)),
                            "k" => columns.push(Column::K(nam, w)),
                            "cpk" => columns.push(Column::Cpk(nam, w)),
                            "cp" => columns.push(Column::Cp(nam, w)),
                            "percentile" => {
                                if !args.is_empty() {
                                    let f = match args[0].parse::<f64>() {
                                        Ok(v) => v,
                                        Err(_) => {
                                            return Err(format!(
                                            "invalid ARG '{}' for column '{}' in configfile {}.",
                                            args[0], opt, cfgpath
                                        ))
                                        }
                                    };
                                    columns.push(Column::Percentile(nam, w, f));
                                }
                            }
                            "p25" => columns.push(Column::P25(nam, w)),
                            "p75" => columns.push(Column::P75(nam, w)),
                            "lsl" => columns.push(Column::LSL(nam, w)),
                            "tgt" => columns.push(Column::TGT(nam, w)),
                            "usl" => columns.push(Column::USL(nam, w)),
                            "lcl" => columns.push(Column::LCL(nam, w)),
                            "ucl" => columns.push(Column::UCL(nam, w)),
                            "boxplot" => columns.push(Column::Boxplot(nam, w)),
                            "histogram" => columns.push(Column::Histogram(nam, w)),
                            "cpkplot" => columns.push(Column::Cpkplot(nam, w)),
                            _ => return Err(unknown_val(opt, val)),
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "unknown OPT '{}' in configfile '{}'.",
                        opt, cfgpath
                    ))
                }
            }
        }
    }

    if columns.len() < 3 {
        columns = default_columns();
    }

    Ok((qtableprops, columns))
}
