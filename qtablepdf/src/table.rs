//! PDF table types and methods
use crate::config::QTableProps;
use crate::limits::Limits;
use crate::numbers::Numbers;
use crate::pdf::{sub_strings, tint, Pdf, Pos, Tint};
use itertools_num::ItertoolsNum;
use printpdf::*;

pub const ONEPOINT: f64 = 0.3527777778;

pub struct TableColors {
    pub header_color: Tint,
    pub fill_color: Tint,
    pub outline_color: Tint,
    pub good_color: Tint,
    pub bad_color: Tint,
    pub neutral_color: Tint,
}

pub struct Table<'a> {
    /// pdf to put table in
    pub pdf: &'a mut Pdf,
    /// cell font size
    pub fnt: i64,
    /// cell left padding
    pub lef: f64,
    /// cell bottom padding
    pub bot: f64,
    /// 1 line height in cell
    pub hei: f64,
    /// max chars per column
    pub max: Vec<usize>,
    /// left x of cell
    pub pos: Vec<f64>,
    /// column width in mm
    pub wid: Vec<f64>,
    //    /// cumulative column width all columns
    pub all: f64,
    /// column names
    pub hea: &'a Vec<String>,
    /// row colors
    pub col: TableColors,
    /// caption
    pub cap: &'a str,
}

type PlotFunc = fn(&Table, &Numbers, &Limits, &QTableProps, &Pos, f64, f64) -> ();

pub enum CellContent<'a> {
    String(String),
    Chart(PlotFunc, &'a Numbers, &'a Limits, QTableProps),
}

impl<'a> Table<'a> {
    pub fn new(
        pdf: &'a mut Pdf,
        fnt: i64,
        cow: &Vec<f64>,
        hea: &'a Vec<String>,
        cap: &'a str,
    ) -> Self {
        let sum: f64 = cow.iter().sum();
        if sum > 100. {
            println!(
                "Warning: {} == sum of column widths {:?} is > 100",
                sum, cow
            )
        }

        let w = pdf.siz.wid - pdf.mar.lef - pdf.mar.rig;

        let wid: Vec<f64> = cow.into_iter().map(|x| x * w / 100.0).collect();

        let mut pos = vec![0.0];
        let widsum: Vec<f64> = wid.iter().cumsum::<f64>().collect();

        let all = widsum[widsum.len() - 1];
        for i in widsum {
            pos.push(i);
        }
        let lef = 0.3 * fnt as f64 * ONEPOINT;
        let bot = 0.3 * fnt as f64 * ONEPOINT;
        let hei = 1.3 * fnt as f64 * ONEPOINT;

        let mut max = vec![];
        for w in wid.iter() {
            max.push((*w / (0.6 * (fnt as f64) * ONEPOINT) - lef) as usize);
        }

        let col = TableColors {
            header_color: Tint::DarkGrey,
            fill_color: Tint::White,
            outline_color: Tint::Black,
            good_color: Tint::Green,
            bad_color: Tint::Red,
            neutral_color: Tint::White,
        };

        let hea2 = hea.clone();
        let mut header = hea2
            .iter()
            .map(|x| CellContent::String(x.parse().unwrap()))
            .collect();

        let mut pdftable = Table {
            pdf,
            fnt,
            lef,
            bot,
            hei,
            max,
            pos,
            wid,
            all,
            hea,
            col,
            cap,
        };
        let hea_col = tint(&pdftable.col.header_color);
        &pdftable.caption(true);
        &pdftable.row(&mut header, &hea_col, false, false);
        pdftable
    }

    pub fn caption(&mut self, add: bool) {
        if self.cap == "" || !add {
            return;
        }
        let w = (self.all / (0.6 * (self.fnt as f64) * ONEPOINT) - self.lef) as usize;

        let vecstr = sub_strings(self.cap, w); // string wrapped
        let nlines = vecstr.len();

        self.pdf
            .lay
            .set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

        let len = vecstr.len() as f64;
        for (i, s) in vecstr.into_iter().rev().enumerate() {
            let x = self.pdf.mar.lef + self.lef;
            let y =
                self.pdf.siz.hei - self.pdf.pos.y - self.pdf.mar.top - (len - i as f64) * self.hei
                    + self.bot;
            self.pdf
                .lay
                .use_text(s, self.fnt, Mm(x), Mm(y), &self.pdf.fnt);
        }

        self.pdf.pos.y += (nlines as f64) * self.hei;
    }

    pub fn cell(
        &self,
        pos: &Pos,
        wid: f64,
        nls: f64,
        fnt: i64,
        str: &Vec<&str>,
        fill_color: &Color,
    ) {
        self.pdf.rect(
            true,
            Pos { x: pos.x, y: pos.y },
            Pos {
                x: pos.x + wid,
                y: pos.y,
            },
            Pos {
                x: pos.x + wid,
                y: pos.y + nls * self.hei,
            },
            Pos {
                x: pos.x,
                y: pos.y + nls * self.hei,
            },
        );

        self.pdf
            .lay
            .set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

        let len = str.len() as f64;
        for (i, s) in str.into_iter().rev().enumerate() {
            let x = pos.x + self.pdf.mar.lef + self.lef;
            let y = self.pdf.siz.hei - pos.y - self.pdf.mar.top - (len - i as f64) * self.hei
                + self.bot;
            self.pdf.lay.use_text(*s, fnt, Mm(x), Mm(y), &self.pdf.fnt);
        }

        self.pdf.lay.set_fill_color(fill_color.clone());
    }

    pub fn row(
        &mut self,
        line: &mut Vec<CellContent>,
        fill_color: &Color,
        indent: bool,
        add_caption: bool,
    ) {
        let mut nlines = 1;
        let mut vecstr = vec![]; // string wrapped

        for (i, _) in self.wid.iter().enumerate() {
            match &line[i] {
                CellContent::String(str) => {
                    vecstr.push(sub_strings(str, self.max[i]));
                    let vecstrlen = vecstr[i].len();
                    if vecstrlen > nlines {
                        nlines = vecstrlen;
                    }
                }
                _ => {
                    vecstr.push(vec![""]);
                }
            }
        }

        // add new page, if necessary
        if self.pdf.pos.y + (nlines as f64) * self.hei
            > self.pdf.siz.hei - self.pdf.mar.top - self.pdf.mar.bot
        {
            // final table line at old page
            self.table_full_line();

            // start new page
            self.pdf.pos.y = 0.0;
            let (page, layer) =
                self.pdf
                    .doc
                    .add_page(Mm(self.pdf.siz.wid), Mm(self.pdf.siz.hei), "layer");
            self.pdf.lay = self.pdf.doc.get_page(page).get_layer(layer);

            self.pdf.lay.set_fill_color(tint(&self.col.fill_color));
            self.pdf
                .lay
                .set_outline_color(tint(&self.col.outline_color));
            self.pdf.lay.set_outline_thickness(0.7);

            // ad caption to new page
            self.caption(add_caption);

            // add header to new page
            let mut nlines = 1;
            let mut vecstr = vec![];
            for (i, _) in self.wid.iter().enumerate() {
                vecstr.push(sub_strings(&self.hea[i], self.max[i]));
                let vecstrlen = vecstr[i].len();
                if vecstrlen > nlines {
                    nlines = vecstrlen;
                }
            }
            self.pdf.lay.set_fill_color(tint(&self.col.header_color));
            for (i, w) in self.wid.iter().enumerate() {
                self.cell(
                    &Pos {
                        x: self.pos[i],
                        y: self.pdf.pos.y,
                    },
                    *w,
                    nlines as f64,
                    self.fnt,
                    &vecstr[i],
                    &tint(&self.col.header_color),
                );
            }

            self.pdf.pos.y += (nlines as f64) * self.hei;
        }

        self.pdf.lay.set_fill_color(fill_color.clone());

        for (i, w) in self.wid.iter().enumerate() {
            if *w == 0.0_f64 {
                continue;
            }
            let mut pos = Pos {
                x: self.pos[i],
                y: self.pdf.pos.y,
            };
            if i == 0 && indent {
                pos = Pos {
                    x: self.pos[i] + 0.20 * (self.fnt as f64),
                    y: self.pdf.pos.y,
                };
            }
            match &line[i] {
                CellContent::Chart(plot, numbers, limits, chartmode) => {
                    plot(&self, numbers, limits, chartmode, &pos, *w, nlines as f64)
                }
                _ => self.cell(&pos, *w, nlines as f64, self.fnt, &vecstr[i], &fill_color),
            }
        }

        self.pdf.pos.y += (nlines as f64) * self.hei;
    }

    pub fn table_full_line(&self) {
        self.pdf.line(
            Pos {
                x: self.pdf.pos.x,
                y: self.pdf.pos.y,
            },
            Pos {
                x: self.all,
                y: self.pdf.pos.y,
            },
        );
    }
}
