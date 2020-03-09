//! PDF document types and methods
extern crate printpdf;

use crate::config::PDFTimestamp;
use chrono::{DateTime, Local, Utc};
use printpdf::*;
use regex::Regex;
use std::fs::File;
use std::io::BufWriter;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub enum Tint {
    Black,
    Blue,
    LightBlue,
    PaleGreen,
    White,
    Grey,
    DarkGrey,
    Carmine,
    Red,
    YellowOrange,
    Green,
    DarkGreen,
    Gold,
    Plum,
    Fuchsia,
}

pub fn tint(tint: &Tint) -> Color {
    match tint {
        Tint::Black => Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
        Tint::Blue => Color::Rgb(Rgb::new(0.0, 0.0, 1.0, None)),
        Tint::LightBlue => Color::Rgb(Rgb::new(120.0 / 255.0, 190.0 / 255.0, 1.0, None)),
        Tint::PaleGreen => Color::Rgb(Rgb::new(152.0 / 255.0, 251.0 / 255.0, 152.0 / 255.0, None)),
        Tint::White => Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)),
        Tint::Grey => Color::Rgb(Rgb::new(0.9, 0.9, 0.9, None)),
        Tint::DarkGrey => Color::Rgb(Rgb::new(0.7, 0.7, 0.7, None)),
        Tint::Red => Color::Rgb(Rgb::new(1.0, 0.0, 0.0, None)),
        Tint::YellowOrange => Color::Rgb(Rgb::new(1.0, 204.0 / 255.0, 0.0, None)),
        Tint::Carmine => Color::Rgb(Rgb::new(150.0 / 255.0, 0.0, 24.0 / 255.0, None)),
        Tint::Green => Color::Rgb(Rgb::new(0.0, 1.0, 0.0, None)),
        Tint::DarkGreen => Color::Rgb(Rgb::new(0.0, 150.0 / 255.0, 0.0, None)),
        Tint::Gold => Color::Rgb(Rgb::new(1.0, 215.0 / 255.0, 0.0, None)),
        Tint::Plum => Color::Rgb(Rgb::new(221.0 / 255.0, 160.0 / 255.0, 221.0 / 255.0, None)),
        Tint::Fuchsia => Color::Rgb(Rgb::new(1.0, 0.0, 1.0, None)),
    }
}

#[derive(Debug)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
}

pub struct Siz {
    pub wid: f64,
    pub hei: f64,
}

pub struct Mar {
    pub lef: f64,
    pub rig: f64,
    pub top: f64,
    pub bot: f64,
}

pub struct Pdf {
    pub doc: PdfDocumentReference,
    pub lay: PdfLayerReference,
    pub siz: Siz,
    pub mar: Mar,
    pub pos: Pos,
    pub thk: f64,
    // outline thickness
    pub fnt: IndirectFontRef,
}

pub fn sub_strings(string: &str, mut sub_len: usize) -> Vec<&str> {
    if sub_len < 1 {
        sub_len = 1;
    }
    let mut subs = Vec::with_capacity(string.len() / sub_len);
    let mut iter = string.chars();
    let mut pos = 0;

    while pos < string.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(sub_len) {
            len += ch.len_utf8();
        }
        subs.push(&string[pos..pos + len]);
        pos += len;
    }
    subs
}

#[derive(Debug, Clone, PartialEq)]
pub enum Paper {
    A4Portrait,
    A4Landscape,
    LetterPortrait,
    LetterLandscape,
}

impl Pdf {
    pub fn new(paper: &Paper) -> Self {
        let siz = match paper {
            Paper::A4Portrait => Siz {
                wid: 210.0,
                hei: 297.0,
            },
            Paper::A4Landscape => Siz {
                wid: 297.0,
                hei: 210.0,
            },
            Paper::LetterPortrait => Siz {
                wid: 215.9,
                hei: 279.4,
            },
            Paper::LetterLandscape => Siz {
                wid: 279.4,
                hei: 215.9,
            },
        };
        let (doc, page1, layer1) = PdfDocument::new("title", Mm(siz.wid), Mm(siz.hei), "Layer 1");
        let lay = doc.get_page(page1).get_layer(layer1);
        let fill_color = tint(&Tint::White);
        let outline_color = tint(&Tint::Black);
        lay.set_fill_color(fill_color);
        lay.set_outline_color(outline_color);
        let thk = 0.7;
        lay.set_outline_thickness(thk);
        let mar = Mar {
            lef: 5.4,
            rig: 5.4,
            top: 5.4,
            bot: 5.4,
        };
        let pos = Pos { x: 0.0, y: 0.0 };
        let fnt = doc.add_builtin_font(BuiltinFont::CourierBold).unwrap();
        Self {
            doc,
            lay,
            siz,
            mar,
            pos,
            thk,
            fnt,
        }
    }

    pub fn point(&self, pos: Pos) -> Point {
        Point::new(
            Mm(pos.x + self.mar.lef),
            Mm(self.siz.hei - (pos.y + self.mar.top)),
        )
    }

    pub fn line(&self, pos0: Pos, pos1: Pos) {
        let points = vec![(self.point(pos0), false), (self.point(pos1), false)];
        let line = Line {
            points,
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        self.lay.add_shape(line);
    }

    pub fn rect(&self, filled: bool, pos0: Pos, pos1: Pos, pos2: Pos, pos3: Pos) {
        let points = vec![
            (self.point(pos0), false),
            (self.point(pos1), false),
            (self.point(pos2), false),
            (self.point(pos3), false),
        ];

        let line = Line {
            points,
            is_closed: true,
            has_fill: filled,
            has_stroke: true,
            is_clipping_path: false,
        };

        self.lay.add_shape(line);
    }

    pub fn save(
        self,
        pdffolder: &str,
        pdffile: &str,
        timestamp: &PDFTimestamp,
    ) -> Result<String, String> {
        let path = make_path(pdffolder, pdffile, timestamp)?;

        let file = match File::create(&path) {
            Err(why) => {
                return Err(format!("PDF file '{}': {}", path, why.to_string()));
            }
            Ok(file) => file,
        };
        match self.doc.save(&mut BufWriter::new(file)) {
            Ok(_) => Ok(path),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

pub fn make_path(pdffold: &str, pdffile: &str, timestamp: &PDFTimestamp) -> Result<String, String> {
    let pdffolder = match pdffold.is_empty() {
        true => ".",
        false => pdffold,
    };

    if pdffile.is_empty() {
        return Err(format!("PDF filename cannot be empty."));
    }

    let ext = match Path::new(&pdffile).extension() {
        Some(e) => e.to_str().unwrap(),
        None => "",
    };

    let mut pathbuf = PathBuf::new();
    pathbuf.push(pdffolder);
    if !pathbuf.exists() {
        return Err(format!("Folder '{:?}' does not exist.", pathbuf));
    }

    let rg = Regex::new(format!("(?i)\\.{}?", ext).as_str()).unwrap();

    let timestr: String;
    let system_time = SystemTime::now();
    match *timestamp {
        PDFTimestamp::Local => {
            let datetime: DateTime<Local> = system_time.into();
            timestr = datetime.format("_%Y%m%d_%H%M%S").to_string();
        }
        PDFTimestamp::UTC => {
            let datetime: DateTime<Utc> = system_time.into();
            timestr = datetime.format("_%Y%m%d_%H%M%S").to_string();
        }
        PDFTimestamp::None => {
            timestr = "".to_string();
        }
    }

    let timpdf = format!("{}.pdf", timestr);
    let pdfout = rg.replace(&pdffile, "");
    let pdfoutfile = format!("{}{}", pdfout, timpdf);
    pathbuf.push(pdfoutfile);
    let outpath = pathbuf.as_os_str().to_str().unwrap().to_string();
    Ok(outpath)
}
