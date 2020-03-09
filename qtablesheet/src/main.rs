extern crate enumflags2;
extern crate qtablepdf;

use qtablepdf::config::{check_infiles, info, read_config};
use qtablepdf::pdf::Pdf;
use qtablepdf::qtable::QTable;
use qtablepdf::sample;
use std::path::Path;
use std::{env, time::Instant};

fn main() {
    const RELEASE: bool = true;
    let prognam: String;
    let mut infiles: Vec<String>;

    if RELEASE {
        // release
        infiles = env::args().collect();
        prognam = infiles.remove(0);
    } else {
        // debug
        prognam = "qtablesheet".to_string();
        infiles = vec![
            sample::fpath(vec!["data", "sample.dat.csv"]),
            sample::fpath(vec!["data", "sample.cfg.csv"]),
            sample::fpath(vec!["data", "sample.lim.csv"]),
            //            sample::fpath(vec!["samples"]),
            //            sample::fpath(vec!["limits"]),
            //            sample::fpath(vec!["help"]),
        ];
    }

    match run_app(&prognam, infiles) {
        Ok(_) => 0,
        Err(err) => {
            info(err);
            sample::help(&prognam);
            1
        }
    };
}

fn run_app(prognam: &str, infiles: Vec<String>) -> Result<(), String> {
    // set start for duration output
    let start = Instant::now();

    // check infiles, return error, if something goes wrong. At least a data file must be provided
    let (datpath, limpath, cfgpath) = check_infiles(infiles, &prognam)?;

    // info for the user, which files are used
    println!("starting .....: {}", prognam);
    println!("using dat file: {}", datpath);
    println!("using lim file: {}", limpath);
    println!("using cfg file: {}", cfgpath);

    // read qtableprops and columns from config file, if any, otherwise use defaults
    let (qtableprops, columns) = read_config(&cfgpath)?;

    // set pdf paper with input from qtableprops
    let mut pdf = Pdf::new(&qtableprops.paper);

    // create the qtable
    QTable::new(&mut pdf, &datpath, &limpath, &columns, &qtableprops)?;

    // get output folder from config or data file
    let path = Path::new(&datpath);
    let pdffolder = match qtableprops.pdffolder.is_empty() {
        true => path.parent().unwrap().to_str().unwrap(),
        false => &qtableprops.pdffolder,
    };

    // get output file name from config or data file
    let pdffile = match qtableprops.pdffile.is_empty() {
        true => path.file_name().unwrap().to_str().unwrap(),
        false => &qtableprops.pdffile,
    };

    // write output
    let pdfpath = pdf.save(&pdffolder, &pdffile, &qtableprops.pdftimestamp)?;

    // info for the user
    println!["written to pdf: {}", pdfpath];
    println!["time to finish: {:#?}", start.elapsed()];

    Ok(())
}
