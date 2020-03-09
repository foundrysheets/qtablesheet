//! data table types and methods
use crate::limits::LimitsTable;
use crate::limits::LimitsTableExt;
use crate::numbers;
use crate::qtable::Filter;
extern crate csv;
use crate::numbers::F64Ext;
use csv::Reader;
use csv::Writer;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct Data {
    pub name: String,
    pub filt: Filter,
    pub vals: Vec<String>,
}

pub type DataTable = BTreeMap<usize, Data>;

impl fmt::Debug for Data {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Data({:?}, len:{:?}), values:{:?}",
            self.name,
            self.vals.len(),
            &self.vals
        )
    }
}

impl fmt::Display for Data {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let l = match self.vals.len() < 3 {
            true => self.vals.len(),
            false => 3,
        };
        write!(
            formatter,
            "Data({:?}, len:{:?}), values:{:#?}",
            self.name,
            self.vals.len(),
            &self.vals[0..l]
        )
    }
}

pub trait DataTableExt<T> {
    fn add_data(
        &mut self,
        datpath: &String,
        limitstable: &LimitsTable,
        filter: &Filter,
    ) -> Result<(), String>;
}

impl DataTableExt<DataTable> for DataTable {
    fn add_data(
        &mut self,
        datpath: &String,
        limitstable: &LimitsTable,
        filter: &Filter,
    ) -> Result<(), String> {
        let datfile = match File::open(datpath) {
            Err(e) => {
                return Err(format!("could not open CSV datafile '{}': {}.", datpath, e));
            }
            Ok(datfile) => datfile,
        };
        let mut rdr: Reader<File> = Reader::from_reader(datfile);

        let headers = match rdr.headers() {
            Ok(record) => record,
            Err(e) => {
                return Err(format!("could not read CSV datafile '{}': {}.", datpath, e));
            }
        };

        let mut empty_header: HashMap<usize, usize> = HashMap::new();
        let mut k = 0;
        for (i, h) in headers.iter().enumerate() {
            if h.is_empty() {
                empty_header.insert(i, i);
                continue;
            }

            let filter_outliers = limitstable.get_filter(h, &filter);
            &self.insert(
                k,
                Data {
                    name: h.to_string(),
                    filt: filter_outliers,
                    vals: vec![],
                },
            );
            k += 1;
        }

        for result in rdr.records() {
            let record = match result {
                Err(_) => {
                    continue;
                }
                Ok(record) => record,
            };
            let mut k = 0;
            let mut empty_line = true;
            for v in record.iter() {
                if !v.is_empty() {
                    empty_line = false;
                    break;
                }
            }
            if empty_line {
                continue;
            }
            for (i, v) in record.iter().enumerate() {
                if empty_header.contains_key(&i) {
                    continue;
                }
                &self.get_mut(&k).unwrap().vals.push(v.parse().unwrap());
                k += 1;
            }
        }
        Ok(())
    }
}

pub fn create_limits_file(datpathstr: &String, sigdigits: usize) -> Result<(), String> {
    let mut datatable = DataTable::new();
    let limitstable = LimitsTable::new();
    datatable.add_data(&datpathstr, &limitstable, &Filter::IQR(1.5))?;

    let datpath = Path::new(datpathstr);
    let mut pathbuf = PathBuf::new();
    pathbuf.push(datpath.parent().unwrap());
    pathbuf.push(datpath.file_stem().unwrap());
    pathbuf.set_extension("lim.csv");

    let mut wtr = match Writer::from_path(&pathbuf) {
        Ok(wtr) => wtr,
        Err(e) => return Err(format!("{:?}", e)),
    };

    match wtr.write_record(&[
        "USE", "PAR", "LSL", "TGT", "USL", "LCL", "UCL", "", "<FIL", "TER>",
    ]) {
        Ok(_) => (),
        Err(e) => return Err(format!("{:?}", e)),
    };

    for (_k, v) in datatable.iter() {
        if v.vals.len() < 1 {
            continue;
        }
        let numbers = numbers::Numbers::new(&v.vals, std::f64::MAX, &v.filt);
        let med = numbers.med();
        let std = numbers.std();
        match wtr.write_record(&[
            "x",
            &v.name,
            &(med - 6.0 * std).frmtf64(sigdigits, ""),
            &med.frmtf64(sigdigits, ""),
            &(med + 6.0 * std).frmtf64(sigdigits, ""),
            &(med - 3.0 * std).frmtf64(sigdigits, ""),
            &(med + 3.0 * std).frmtf64(sigdigits, ""),
            "",
            "iqr",
            "1.5",
        ]) {
            Ok(_) => (),
            Err(e) => return Err(format!("{:?}", e)),
        };
    }
    match wtr.flush() {
        Ok(_) => (),
        Err(e) => return Err(format!("{:?}", e)),
    };

    println!(
        "Limits file '{:?}' generated from '{}'.",
        pathbuf, datpathstr
    );
    Ok(())
}
