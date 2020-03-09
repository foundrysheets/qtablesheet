use crate::config::QTableProps;
use crate::data::DataTable;
use crate::data::DataTableExt;
use crate::group::Groups;
use crate::limits::LimitsTableExt;
use crate::limits::{Limits, LimitsExt, LimitsTable, YieldOk};
use crate::numbers::{F64Ext, Numbers};
use crate::pdf::{tint, Pdf, Pos, Tint};
use crate::table::{CellContent, Table};
use crate::{limits, numbers};
use enumflags2::BitFlags;
use printpdf::{Color, Rgb};

pub struct QTable<'a> {
    pub table: Table<'a>,
    pub qtableprops: &'a QTableProps,
    pub groups: &'a Groups,
}

impl QTable<'_> {
    /// Adds a Qtable(aka quality table)
    ///
    /// # Arguments
    ///
    /// * `datpath` - path to .csv data file.
    /// * `limpath` - path to .csv limits file.
    /// * `mar` - top margin in mm.
    /// * `fnt` - font size in points.
    /// * `hea` - column headers as `&mut Vec<String>`.
    /// * `col` - column widths in mm as `&Vec<f64>`.
    /// * `col` - column widths in mm as `&Vec<f64>`.
    ///
    /// # Remarks
    ///
    /// This function adds a statistical table to the PDF file`.
    ///
    /// *Note*: the path to the .csv limits file is optional.
    /// If missing, only simple stats are inserted into the table.
    pub fn new<'a>(
        pdf: &mut Pdf,
        datpath: &String,
        limpath: &String,
        columns_in: &'a Vec<Column>,
        qtableprops: &QTableProps,
    ) -> Result<(), String> {
        if columns_in.len() < 1 {
            return Err(format!("no columns defined for Qtable::new(...)."));
        }

        let mut columns = vec![
            Column::Number("num".to_string(), 5.),
            Column::Parameter("parameter".to_string(), 10.),
        ];

        for c in columns_in {
            match c {
                Column::Number(_, _) => columns[0] = c.clone(),
                Column::Parameter(_, _) => columns[1] = c.clone(),
                _ => {
                    if !columns.contains(c) {
                        columns.push(c.clone())
                    }
                }
            }
        }

        let column_headers: Vec<String> = columns
            .iter()
            .map(|x| x.column_name().to_string())
            .collect();
        let column_widths = columns.iter().map(|x| *(x.column_width())).collect();

        let limitstable = &mut LimitsTable::new();
        limitstable.read_limits(&limpath)?;

        let mut datatable = DataTable::new();
        datatable.add_data(&datpath, &limitstable, &qtableprops.filter)?;

        pdf.pos.y += qtableprops.margin;

        let groups = Groups::new(&datatable, &qtableprops)?;

        let mut qcaption = qtableprops.caption.clone();
        for (i, g) in groups.groups.iter().enumerate() {
            if i == 0 {
                qcaption = format!["{}: {} {} |", qcaption, g.name, g.group];
            } else {
                qcaption = format!["{} {} {} |", qcaption, g.name, g.group];
            }
        }

        let mut qtable = QTable {
            table: Table::new(
                pdf,
                qtableprops.fontsize,
                &column_widths,
                &column_headers,
                qcaption.as_ref(),
            ),
            qtableprops: qtableprops,
            groups: &groups,
        };

        let mut badspec: Vec<Par> = vec![];
        let mut badctrl: Vec<Par> = vec![];
        let mut badcpk: Vec<Par> = vec![];
        let mut good: Vec<Par> = vec![];
        let mut nolimits: Vec<Par> = vec![];

        let numwidth = (datatable.len() as f64).log10().abs() as usize + 1;

        for (k, v) in datatable.iter() {
            let numbers = numbers::Numbers::new(&v.vals, qtableprops.float_limit, &v.filt);
            let (limok, limits) = limitstable.check_limits(
                &v.name,
                &numbers,
                qtable.qtableprops.spec_yield_limit,
                qtable.qtableprops.ctrl_yield_limit,
                qtable.qtableprops.cpk_limit,
                qtable.qtableprops.mark,
            );

            let par = Par {
                number: *k,
                group: "".to_string(),
                name: v.name.clone(),
                limitsok: limok,
                numbers,
                limits,
            };

            if qtableprops.order == Order::ByBadGood
                && limitstable.len() > 0
                && limok != YieldOk::Yes
            {
                match limok {
                    YieldOk::SpecYieldNot => badspec.push(par),
                    YieldOk::CtrlYieldNot => badctrl.push(par),
                    YieldOk::CpkNot => badcpk.push(par),
                    _ => nolimits.push(par),
                }
            } else {
                good.push(par);
            }
        }

        //badspec
        for par in &badspec {
            qtable.group_ruler(&groups);
            let (mut line, rowcolor) = qtable.qtable_line(par, &columns, numwidth);
            qtable
                .table
                .row(&mut line, &rowcolor, false, qtableprops.captioneverypage);
            qtable.group_lines(par, &datatable, &groups, &limitstable, &columns);
        }

        //badctrl
        for par in &badctrl {
            qtable.group_ruler(&groups);
            let (mut line, rowcolor) = qtable.qtable_line(par, &columns, numwidth);
            qtable
                .table
                .row(&mut line, &rowcolor, false, qtableprops.captioneverypage);
            qtable.group_lines(par, &datatable, &groups, &limitstable, &columns);
        }

        //badcpk
        for par in &badcpk {
            qtable.group_ruler(&groups);
            let (mut line, rowcolor) = qtable.qtable_line(par, &columns, numwidth);
            qtable
                .table
                .row(&mut line, &rowcolor, false, qtableprops.captioneverypage);
            qtable.group_lines(par, &datatable, &groups, &limitstable, &columns);
        }

        //good
        for par in &good {
            qtable.group_ruler(&groups);
            let (mut line, rowcolor) = qtable.qtable_line(par, &columns, numwidth);
            qtable
                .table
                .row(&mut line, &rowcolor, false, qtableprops.captioneverypage);
            qtable.group_lines(par, &datatable, &groups, &limitstable, &columns);
        }

        //nolimits
        for par in &nolimits {
            qtable.group_ruler(&groups);
            let (mut line, rowcolor) = qtable.qtable_line(par, &columns, numwidth);
            qtable
                .table
                .row(&mut line, &rowcolor, false, qtableprops.captioneverypage);
            qtable.group_lines(par, &datatable, &groups, &limitstable, &columns);
        }

        //last line of table
        qtable.table.table_full_line();

        Ok(())
    }

    pub fn group_ruler(&self, groups: &Groups) {
        if groups.groups.len() > 0 {
            self.table.pdf.lay.set_outline_color(tint(&Tint::Blue));
            self.table.table_full_line();
        }
    }

    pub fn group_lines<'a>(
        &mut self,
        par: &Par,
        datatable: &DataTable,
        by_groups: &Groups,
        limitstable: &LimitsTable,
        columns: &'a Vec<Column>,
    ) {
        if par.numbers.cnt() == 0.0 {
            return;
        }
        if by_groups.groups.len() == 0 {
            return;
        }

        let mut numwid = 0.0;
        let mut parwid = 0.0;
        let mut nummax = 0;

        if by_groups.groups.len() > 1 && self.qtableprops.longgroupnames && columns.len() > 1 {
            numwid = self.table.wid[0];
            parwid = self.table.wid[1];
            nummax = self.table.max[0];
            let parmax = self.table.max[1];
            self.table.wid[0] = numwid + parwid;
            self.table.max[0] = nummax + parmax;
            self.table.wid[1] = 0.0;
        }

        let data = &datatable.get(&par.number).unwrap().vals;
        let filt = &datatable.get(&par.number).unwrap().filt;
        for g in by_groups.groups.iter() {
            let gvals = g
                .indices
                .iter()
                .map(|i| data[*i].clone())
                .collect::<Vec<_>>();
            let numbers = numbers::Numbers::new(&gvals, self.qtableprops.float_limit, &filt);
            let (limitsok, limits) = limitstable.check_limits(
                &par.name,
                &numbers,
                self.qtableprops.spec_yield_limit,
                self.qtableprops.ctrl_yield_limit,
                self.qtableprops.cpk_limit,
                self.qtableprops.mark,
            );

            let groupname = match self.qtableprops.longgroupnames {
                true => format!("{} {}", g.name, g.group),
                false => format!("{}", g.name),
            };

            let par = Par {
                number: par.number.clone(),
                group: groupname,
                name: par.name.clone(),
                limitsok,
                numbers,
                limits,
            };

            let (mut line, rowcolor) = self.qtable_line(&par, columns, 0);
            let groupcolor = self.dimm_color(&rowcolor);
            self.table.row(
                &mut line,
                &groupcolor,
                true,
                self.qtableprops.captioneverypage,
            );
        }
        if by_groups.groups.len() > 1 && self.qtableprops.longgroupnames && columns.len() > 1 {
            self.table.wid[0] = numwid;
            self.table.max[0] = nummax;
            self.table.wid[1] = parwid;
        }
    }

    pub fn dimm_color(&self, color: &Color) -> Color {
        let mut color_vec = color.clone().into_vec();
        if color_vec[0] == 1.0 {
            color_vec[0] -= 0.1;
        } else {
            color_vec[0] += 0.5;
        }
        if color_vec[1] == 1.0 {
            color_vec[1] -= 0.1;
        } else {
            color_vec[1] += 0.5;
        }
        if color_vec[2] == 1.0 {
            color_vec[2] -= 0.1;
        } else {
            color_vec[2] += 0.5;
        }
        Color::Rgb(Rgb::new(color_vec[0], color_vec[1], color_vec[2], None))
    }

    pub fn qtable_line<'a>(
        &mut self,
        par: &'a Par,
        columns: &'a Vec<Column>,
        numwidth: usize,
    ) -> (Vec<CellContent<'a>>, Color) {
        let k = par.number;
        let name = &par.name;
        let group = &par.group;
        let limok = par.limitsok;
        let param = name.clone();
        let numbers = &par.numbers;
        let limits = &par.limits;
        let mut rowcolor = self.color_by_limits(&limok);
        if numbers.data.len() < 1 {
            rowcolor = tint(&Tint::White);
        }
        let line: Vec<CellContent> = columns
            .iter()
            .map(|x| {
                x.column_value(
                    &k,
                    &group,
                    &param,
                    &numbers,
                    &limits,
                    &self.qtableprops,
                    numwidth,
                )
            })
            .collect();
        (line, rowcolor)
    }

    pub fn color_by_limits(&mut self, limok: &YieldOk) -> Color {
        match limok {
            YieldOk::Yes => tint(&Tint::Green),
            YieldOk::SpecYieldNot => tint(&Tint::Red),
            YieldOk::CtrlYieldNot => tint(&Tint::YellowOrange),
            YieldOk::CpkNot => tint(&Tint::Fuchsia),
            YieldOk::NoLimits => tint(&Tint::White),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Par {
    number: usize,
    group: String,
    name: std::string::String,
    limitsok: limits::YieldOk,
    numbers: numbers::Numbers,
    limits: std::collections::BTreeMap<std::string::String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    ByNumber,
    ByBadGood,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    None,
    IQR(f64),
    ZScore(f64),
    Lower(f64),
    Upper(f64),
    Between(f64, f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Align {
    SpecLimits,
    ControlLimits,
    Targets,
    FitValues,
}

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Show {
    SpecLimits = 0b0001,
    ControlLimits = 0b0010,
    Targets = 0b0100,
}

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Mark {
    SpecYield = 0b0001,
    ControlYield = 0b0010,
    Cpk = 0b0100,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    Number(String, f64),
    Parameter(String, f64),
    Count(String, f64),
    Mean(String, f64),
    Median(String, f64),
    Variance(String, f64),
    Sdev(String, f64),
    Min(String, f64),
    Max(String, f64),
    Range(String, f64),
    SpecYield(String, f64),
    CtrlYield(String, f64),
    K(String, f64),
    Cpk(String, f64),
    Cp(String, f64),
    Percentile(String, f64, f64),
    P25(String, f64),
    P75(String, f64),
    LSL(String, f64),
    TGT(String, f64),
    USL(String, f64),
    LCL(String, f64),
    UCL(String, f64),
    Boxplot(String, f64),
    Histogram(String, f64),
    Cpkplot(String, f64),
}

pub trait ColumnsExt<T> {
    fn delete(&mut self, elem: usize) -> Result<(), String>;
    fn replace(&mut self, elem: usize, column: Column) -> Result<(), String>;
}
impl ColumnsExt<Vec<Column>> for Vec<Column> {
    fn delete(&mut self, elem: usize) -> Result<(), String> {
        match elem > self.len() - 1 {
            true => Err(format!(
                "canot delete column '{}', because it is > {}.",
                elem,
                self.len() - 1
            )),
            false => {
                self.remove(elem);
                Ok(())
            }
        }
    }
    fn replace(&mut self, elem: usize, column: Column) -> Result<(), String> {
        match elem > self.len() - 1 {
            true => Err(format!(
                "canot replace column '{}', because it is > {}.",
                elem,
                self.len() - 1
            )),
            false => {
                self[elem] = column;
                Ok(())
            }
        }
    }
}

pub fn default_columns() -> Vec<Column> {
    vec![
        Column::Number("num".to_string(), 4.),
        Column::Parameter("parameter".to_string(), 10.),
        Column::LSL("lsl".to_string(), 6.),
        Column::TGT("tgt".to_string(), 6.),
        Column::USL("usl".to_string(), 6.),
        Column::LCL("lcl".to_string(), 6.),
        Column::UCL("ucl".to_string(), 6.),
        Column::Count("cnt".to_string(), 3.),
        Column::Min("min".to_string(), 6.),
        Column::Mean("mea".to_string(), 6.),
        Column::Max("max".to_string(), 6.),
        Column::CtrlYield("ctrl yld".to_string(), 3.),
        Column::SpecYield("spec yld".to_string(), 3.),
        Column::Cpk("cpk".to_string(), 3.),
        Column::Histogram("histogram".to_string(), 10.),
        Column::Boxplot("boxplot".to_string(), 10.),
        Column::Cpkplot("cpkchart".to_string(), 6.),
    ]
}

impl Column {
    pub fn column_name_width(&self) -> (&str, &f64) {
        match &self {
            Column::Number(name, width) => (name, width),
            Column::Parameter(name, width) => (name, width),
            Column::Count(name, width) => (name, width),
            Column::Mean(name, width) => (name, width),
            Column::Median(name, width) => (name, width),
            Column::Variance(name, width) => (name, width),
            Column::Sdev(name, width) => (name, width),
            Column::Min(name, width) => (name, width),
            Column::Max(name, width) => (name, width),
            Column::Range(name, width) => (name, width),
            Column::SpecYield(name, width) => (name, width),
            Column::CtrlYield(name, width) => (name, width),
            Column::K(name, width) => (name, width),
            Column::Cpk(name, width) => (name, width),
            Column::Cp(name, width) => (name, width),
            Column::LSL(name, width) => (name, width),
            Column::TGT(name, width) => (name, width),
            Column::USL(name, width) => (name, width),
            Column::LCL(name, width) => (name, width),
            Column::UCL(name, width) => (name, width),
            Column::Percentile(name, width, _) => (name, width),
            Column::P25(name, width) => (name, width),
            Column::P75(name, width) => (name, width),
            Column::Boxplot(name, width) => (name, width),
            Column::Histogram(name, width) => (name, width),
            Column::Cpkplot(name, width) => (name, width),
        }
    }
    pub fn column_name(&self) -> &str {
        self.column_name_width().0
    }
    pub fn column_width(&self) -> &f64 {
        self.column_name_width().1
    }
    pub fn column_value<'a>(
        &self,
        num: &usize,
        group: &String,
        par: &String,
        numbers: &'a Numbers,
        limits: &'a Limits,
        qtableprops: &QTableProps,
        numwidth: usize,
    ) -> CellContent<'a> {
        match &self {
            Column::Number(_, _) => match *group != "".to_string() {
                false => CellContent::String(format!("{:0width$}", num + 1, width = numwidth)),
                true => CellContent::String(format!("{:}", group)),
            },
            Column::Parameter(_, _) => CellContent::String(format!("{:}", par)),
            Column::Count(_, _) => {
                CellContent::String(numbers.cnt().frmtint(&qtableprops.nanstring))
            }
            Column::Mean(_, _) => CellContent::String(
                numbers
                    .mea()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::Median(_, _) => CellContent::String(
                numbers
                    .med()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::Variance(_, _) => CellContent::String(
                numbers
                    .var()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::Sdev(_, _) => CellContent::String(
                numbers
                    .std()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::Min(_, _) => CellContent::String(
                numbers
                    .min()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::Max(_, _) => CellContent::String(
                numbers
                    .max()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::Range(_, _) => {
                let min = numbers
                    .min()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring);
                let max = numbers
                    .max()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring);
                CellContent::String(format!("{}...{}", min, max))
            }
            Column::SpecYield(_, _) => {
                let lsl = limits.getnum("lsl");
                let usl = limits.getnum("usl");
                CellContent::String(
                    numbers
                        .yld(&lsl, &usl)
                        .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
                )
            }
            Column::CtrlYield(_, _) => {
                let lcl = limits.getnum("lcl");
                let ucl = limits.getnum("ucl");
                CellContent::String(
                    numbers
                        .yld(&lcl, &ucl)
                        .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
                )
            }
            Column::K(_, _) => {
                let lsl = limits.getnum("lsl");
                let tgt = limits.getnum("tgt");
                let usl = limits.getnum("usl");
                CellContent::String(
                    numbers
                        .k(&lsl, &tgt, &usl)
                        .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
                )
            }
            Column::Cpk(_, _) => {
                let lsl = limits.getnum("lsl");
                let usl = limits.getnum("usl");
                CellContent::String(
                    numbers
                        .cpk(&lsl, &usl)
                        .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
                )
            }
            Column::Cp(_, _) => {
                let lsl = limits.getnum("lsl");
                let usl = limits.getnum("usl");
                CellContent::String(
                    numbers
                        .cp(&lsl, &usl)
                        .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
                )
            }
            Column::Percentile(_, _, perc) => CellContent::String(
                numbers
                    .prc(*perc)
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::P25(_, _) => CellContent::String(
                numbers
                    .p25()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::P75(_, _) => CellContent::String(
                numbers
                    .p75()
                    .frmtf64(qtableprops.sig_digits, &qtableprops.nanstring),
            ),
            Column::LSL(_, _) => CellContent::String(limits.getstr(
                "lsl",
                qtableprops.sig_digits,
                &qtableprops.nanstring,
            )),
            Column::TGT(_, _) => CellContent::String(limits.getstr(
                "tgt",
                qtableprops.sig_digits,
                &qtableprops.nanstring,
            )),
            Column::USL(_, _) => CellContent::String(limits.getstr(
                "usl",
                qtableprops.sig_digits,
                &qtableprops.nanstring,
            )),
            Column::LCL(_, _) => CellContent::String(limits.getstr(
                "lcl",
                qtableprops.sig_digits,
                &qtableprops.nanstring,
            )),
            Column::UCL(_, _) => CellContent::String(limits.getstr(
                "ucl",
                qtableprops.sig_digits,
                &qtableprops.nanstring,
            )),
            Column::Boxplot(_, _) => {
                CellContent::Chart(boxplot, numbers, limits, qtableprops.clone())
            }
            Column::Histogram(_, _) => {
                CellContent::Chart(histogram, numbers, limits, qtableprops.clone())
            }
            Column::Cpkplot(_, _) => {
                CellContent::Chart(cpkplot, numbers, limits, qtableprops.clone())
            }
        }
    }
}

pub fn reset_color_and_thickness(table: &Table) {
    table.pdf.lay.set_outline_thickness(table.pdf.thk);
    table
        .pdf
        .lay
        .set_outline_color(tint(&table.col.outline_color));
    table.pdf.lay.set_fill_color(tint(&table.col.fill_color));
}

pub fn add_limits<F>(
    table: &Table,
    numbers: &Numbers,
    limits: &Limits,
    qtableprops: &QTableProps,
    x: F,
    pos: &Pos,
    wid: f64,
) where
    F: Fn(f64) -> f64,
{
    if numbers.data.len() < 1 {
        return;
    }
    let lsl = limits.getnum("lsl");
    let tgt = limits.getnum("tgt");
    let usl = limits.getnum("usl");
    let lcl = limits.getnum("lcl");
    let ucl = limits.getnum("ucl");

    table.pdf.lay.set_outline_thickness(2.0);

    // ctrl_limits
    if qtableprops.show.contains(Show::ControlLimits) {
        if !lcl.is_nan() {
            if numbers.min() >= lcl {
                table.pdf.lay.set_outline_color(tint(&Tint::LightBlue));
            } else {
                table.pdf.lay.set_outline_color(tint(&Tint::YellowOrange));
            }
            let xlcl = x(lcl);
            if xlcl > pos.x && xlcl < pos.x + wid {
                table.pdf.line(
                    Pos {
                        x: xlcl + 0.5,
                        y: pos.y + 0.4 * table.hei,
                    },
                    Pos {
                        x: xlcl + 0.5,
                        y: pos.y + 0.6 * table.hei,
                    },
                );
                table.pdf.line(
                    Pos { x: xlcl, y: pos.y },
                    Pos {
                        x: xlcl,
                        y: pos.y + table.hei,
                    },
                );
            }
        }
        if !ucl.is_nan() {
            if numbers.max() <= ucl {
                table.pdf.lay.set_outline_color(tint(&Tint::LightBlue));
            } else {
                table.pdf.lay.set_outline_color(tint(&Tint::YellowOrange));
            }
            let xucl = x(ucl);
            if xucl > pos.x && xucl < pos.x + wid {
                table.pdf.line(
                    Pos {
                        x: xucl - 0.5,
                        y: pos.y + 0.4 * table.hei,
                    },
                    Pos {
                        x: xucl - 0.5,
                        y: pos.y + 0.6 * table.hei,
                    },
                );
                table.pdf.line(
                    Pos { x: xucl, y: pos.y },
                    Pos {
                        x: xucl,
                        y: pos.y + table.hei,
                    },
                );
            }
        }
    }

    //spec_limits
    if qtableprops.show.contains(Show::SpecLimits) {
        if !lsl.is_nan() {
            if numbers.min() >= lsl {
                table.pdf.lay.set_outline_color(tint(&Tint::Blue));
            } else {
                table.pdf.lay.set_outline_color(tint(&Tint::Red));
            }
            let xlsl = x(lsl);
            if xlsl > pos.x && xlsl < pos.x + wid {
                table.pdf.line(
                    Pos {
                        x: xlsl + 0.5,
                        y: pos.y + 0.4 * table.hei,
                    },
                    Pos {
                        x: xlsl + 0.5,
                        y: pos.y + 0.6 * table.hei,
                    },
                );
                table.pdf.line(
                    Pos { x: xlsl, y: pos.y },
                    Pos {
                        x: xlsl,
                        y: pos.y + table.hei,
                    },
                );
            }
        }
        if !usl.is_nan() {
            if numbers.max() <= usl {
                table.pdf.lay.set_outline_color(tint(&Tint::Blue));
            } else {
                table.pdf.lay.set_outline_color(tint(&Tint::Red));
            }
            let xusl = x(usl);
            if xusl > pos.x && xusl < pos.x + wid {
                table.pdf.line(
                    Pos {
                        x: xusl - 0.5,
                        y: pos.y + 0.4 * table.hei,
                    },
                    Pos {
                        x: xusl - 0.5,
                        y: pos.y + 0.6 * table.hei,
                    },
                );
                table.pdf.line(
                    Pos { x: xusl, y: pos.y },
                    Pos {
                        x: xusl,
                        y: pos.y + table.hei,
                    },
                );
            }
        }
    }

    // targets
    if qtableprops.show.contains(Show::Targets) {
        let xtgt = x(tgt);
        if xtgt > pos.x && xtgt < pos.x + wid {
            if !tgt.is_nan() {
                table.pdf.lay.set_outline_color(tint(&Tint::Green));
                table.pdf.line(
                    Pos { x: xtgt, y: pos.y },
                    Pos {
                        x: xtgt,
                        y: pos.y + table.hei,
                    },
                );
            }
        }
    }

    reset_color_and_thickness(table);
}

pub fn compute_lef_rig_x(
    numbers: &Numbers,
    limits: &Limits,
    qtableprops: &QTableProps,
    posx: f64,
    wid: f64,
) -> (f64, f64, Box<dyn Fn(f64) -> f64>) {
    let min = numbers.min();
    let max = numbers.max();

    let lsl = limits.getnum("lsl");
    let tgt = limits.getnum("tgt");
    let usl = limits.getnum("usl");
    let lcl = limits.getnum("lcl");
    let ucl = limits.getnum("ucl");

    let mut lef = min;
    let mut rig = max;

    match qtableprops.align {
        Align::SpecLimits => {
            if !lsl.is_nan() {
                lef = lsl;
            }
            if !usl.is_nan() {
                rig = usl;
            }
        }
        Align::ControlLimits => {
            if !lcl.is_nan() {
                lef = lcl;
            }
            if !ucl.is_nan() {
                rig = ucl;
            }
        }
        Align::FitValues | Align::Targets => {
            let mut v = vec![min, max];
            if qtableprops.show.contains(Show::SpecLimits) {
                v.push(lsl);
                v.push(usl);
            }
            if qtableprops.show.contains(Show::ControlLimits) {
                v.push(lcl);
                v.push(ucl);
            }
            if qtableprops.show.contains(Show::Targets) {
                v.push(tgt);
            }
            // n contains data plus limits, target
            let n = Numbers::from_f64(v);
            lef = n.min();
            rig = n.max();
            if qtableprops.align == Align::Targets
                && !tgt.is_nan()
                && !lef.is_nan()
                && !rig.is_nan()
            {
                let dlef = tgt - lef;
                let drig = rig - tgt;
                let d = vec![dlef, drig];
                let dmax = Numbers::from_f64(d).max();
                lef = tgt - dmax;
                rig = tgt + dmax;
            }
        }
    }

    let x = move |mut x: f64| -> f64 {
        x = posx + 0.05 * wid + 0.9 * wid * (x - lef) / (rig - lef);
        if x < posx {
            x = posx;
        }
        if x > posx + wid {
            x = posx + wid;
        }
        x
    };

    (lef, rig, Box::new(x))
}

pub fn plot_borders_and_check_empty(
    table: &Table,
    numbers: &Numbers,
    pos: &Pos,
    wid: f64,
    nls: f64,
) -> bool {
    // cell borders left, right
    table.pdf.lay.set_outline_color(tint(&Tint::Black));
    table.pdf.line(
        Pos { x: pos.x, y: pos.y },
        Pos {
            x: pos.x,
            y: pos.y + nls * table.hei,
        },
    );
    table.pdf.line(
        Pos {
            x: pos.x + wid,
            y: pos.y,
        },
        Pos {
            x: pos.x + wid,
            y: pos.y + nls * table.hei,
        },
    );

    // cell is empty
    if numbers.data.iter().any(|x| x.is_nan()) {
        return true;
    }
    false
}

pub fn boxplot(
    table: &Table,
    numbers: &Numbers,
    limits: &Limits,
    qtableprops: &QTableProps,
    pos: &Pos,
    wid: f64,
    nls: f64,
) -> () {
    if plot_borders_and_check_empty(table, numbers, pos, wid, nls) {
        return;
    }

    let min = numbers.min();
    let p25 = numbers.p25();
    let med = numbers.med();
    let mea = numbers.mea();
    let p75 = numbers.p75();
    let max = numbers.max();

    // x -> compute x-position inside cell
    let (lef, rig, x) = compute_lef_rig_x(numbers, limits, qtableprops, pos.x, wid);

    table.pdf.lay.set_fill_color(tint(&Tint::Grey));

    let boxy = pos.y + table.hei * 0.2;
    let boxh = table.hei * 0.6;
    let mima = wid;
    let boxw = mima * (p75 - p25) / (rig - lef);
    if boxw.is_nan() {
        return;
    }

    // whiskers
    table.pdf.lay.set_outline_thickness(0.6);
    let minx = x(min);
    let maxx = x(max);
    table.pdf.line(
        Pos { x: minx, y: boxy },
        Pos {
            x: minx,
            y: boxy + boxh,
        },
    );
    table.pdf.line(
        Pos {
            x: minx,
            y: boxy + boxh / 2.0,
        },
        Pos {
            x: maxx,
            y: boxy + boxh / 2.0,
        },
    );
    table.pdf.line(
        Pos { x: maxx, y: boxy },
        Pos {
            x: maxx,
            y: boxy + boxh,
        },
    );

    let p25x = x(p25);
    let p75x = x(p75);

    table.pdf.rect(
        true,
        Pos { x: p25x, y: boxy },
        Pos { x: p75x, y: boxy },
        Pos {
            x: p75x,
            y: boxy + boxh,
        },
        Pos {
            x: p25x,
            y: boxy + boxh,
        },
    );

    let medx = x(med);
    let meax = x(mea);

    table.pdf.lay.set_outline_thickness(2.0);
    table.pdf.lay.set_outline_color(tint(&Tint::Black));
    table.pdf.line(
        Pos { x: medx, y: boxy },
        Pos {
            x: medx,
            y: boxy + boxh,
        },
    );

    table.pdf.lay.set_outline_thickness(0.6);
    table.pdf.lay.set_fill_color(tint(&Tint::White));
    table.pdf.lay.set_outline_color(tint(&Tint::Black));
    table.pdf.rect(
        true,
        Pos {
            x: meax - 0.2 * boxh,
            y: boxy + 0.3 * boxh,
        },
        Pos {
            x: meax + 0.2 * boxh,
            y: boxy + 0.3 * boxh,
        },
        Pos {
            x: meax + 0.2 * boxh,
            y: boxy + 0.7 * boxh,
        },
        Pos {
            x: meax - 0.2 * boxh,
            y: boxy + 0.7 * boxh,
        },
    );

    // limits
    add_limits(table, numbers, limits, qtableprops, x, pos, wid);
    reset_color_and_thickness(table);
}

pub fn histogram(
    table: &Table,
    numbers: &Numbers,
    limits: &Limits,
    qtableprops: &QTableProps,
    pos: &Pos,
    wid: f64,
    nls: f64,
) -> () {
    if plot_borders_and_check_empty(table, numbers, pos, wid, nls) {
        return;
    }

    let (bins, d) = numbers.bins_delta(qtableprops.histogram_bins);

    // x -> compute x-position inside cell
    let (_lef, _rig, x) = compute_lef_rig_x(numbers, limits, qtableprops, pos.x, wid);

    table.pdf.lay.set_fill_color(tint(&Tint::Grey));

    let vbins = Numbers::from_f64(bins.iter().map(|x| *x as f64).collect());

    let min = numbers.min();

    for (i, v) in bins.iter().enumerate() {
        let boxy = pos.y;
        let boxh = table.hei;
        let j = i as f64;
        let xl = x(min + j * d);
        let xr = x(min + (j + 1.0) * d);
        let ytop = boxh - ((*v as f64) / vbins.max()) * boxh;
        table.pdf.lay.set_outline_thickness(0.6);
        table.pdf.rect(
            true,
            Pos {
                x: xl,
                y: boxy + ytop,
            },
            Pos {
                x: xr,
                y: boxy + ytop,
            },
            Pos {
                x: xr,
                y: boxy + boxh,
            },
            Pos {
                x: xl,
                y: boxy + boxh,
            },
        );
    }

    // limits
    add_limits(table, numbers, limits, qtableprops, x, pos, wid);
    reset_color_and_thickness(table);
}

pub fn cpkplot(
    table: &Table,
    numbers: &Numbers,
    limits: &Limits,
    qtableprops: &QTableProps,
    pos: &Pos,
    wid: f64,
    nls: f64,
) -> () {
    if plot_borders_and_check_empty(table, numbers, pos, wid, nls) {
        return;
    }
    let lef = 0.0;
    let rig = 5.0;
    let posx = pos.x;

    let x = move |mut x: f64| -> f64 {
        x = posx + 0.05 * wid + 0.9 * wid * (x - lef) / (rig - lef);
        if x < posx {
            x = posx;
        }
        if x > posx + wid {
            x = posx + wid;
        }
        x
    };

    let cpklim = x(qtableprops.cpk_limit);

    table.pdf.lay.set_outline_thickness(0.0);
    table.pdf.lay.set_fill_color(tint(&Tint::Plum));
    table.pdf.lay.set_outline_color(tint(&Tint::Plum));
    table.pdf.rect(
        true,
        Pos { x: pos.x, y: pos.y },
        Pos {
            x: cpklim,
            y: pos.y,
        },
        Pos {
            x: cpklim,
            y: pos.y + nls * table.hei,
        },
        Pos {
            x: pos.x,
            y: pos.y + nls * table.hei,
        },
    );

    table.pdf.lay.set_fill_color(tint(&Tint::PaleGreen));
    table.pdf.lay.set_outline_color(tint(&Tint::PaleGreen));
    table.pdf.rect(
        true,
        Pos {
            x: cpklim,
            y: pos.y,
        },
        Pos {
            x: posx + wid,
            y: pos.y,
        },
        Pos {
            x: posx + wid,
            y: pos.y + nls * table.hei,
        },
        Pos {
            x: cpklim,
            y: pos.y + nls * table.hei,
        },
    );

    table.pdf.lay.set_outline_thickness(2.0);
    table.pdf.lay.set_fill_color(tint(&Tint::Green));
    table.pdf.line(
        Pos {
            x: cpklim,
            y: pos.y,
        },
        Pos {
            x: cpklim,
            y: pos.y + nls * table.hei,
        },
    );

    let lsl = limits.getnum("lsl");
    let usl = limits.getnum("usl");
    let cpk = numbers.cpk(&lsl, &usl);
    if cpk < qtableprops.cpk_limit {
        table.pdf.lay.set_outline_color(tint(&Tint::Fuchsia));
    } else {
        table.pdf.lay.set_outline_color(tint(&Tint::DarkGreen));
    }

    let xcpk = x(cpk);
    if !cpk.is_nan() {
        table.pdf.line(
            Pos { x: xcpk, y: pos.y },
            Pos {
                x: xcpk,
                y: pos.y + nls * table.hei,
            },
        );
    }
    reset_color_and_thickness(table);
}
