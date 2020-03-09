use crate::config::QTableProps;
use crate::data::DataTable;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum GroupBy {
    ColNumber(usize),
    ColName(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub(crate) name: String,
    pub(crate) group: String,
    pub(crate) indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Groups {
    pub groups: Vec<Group>,
}

impl Groups {
    pub fn new(datatable: &DataTable, qtableprops: &QTableProps) -> Result<Groups, String> {
        let mut group_by_colnumber = vec![];
        let mut datheaders: HashMap<String, usize> = HashMap::new();
        for (k, v) in datatable.iter() {
            datheaders.insert(v.name.clone(), k.clone());
        }

        for g in qtableprops.group_by.iter() {
            match g {
                GroupBy::ColNumber(u) => {
                    if !group_by_colnumber.contains(u) {
                        group_by_colnumber.push(*u);
                    }
                }
                GroupBy::ColName(p) => match datheaders.contains_key(p) {
                    true => {
                        let u = *datheaders.get(p).unwrap() + 1;
                        if !group_by_colnumber.contains(&(u)) {
                            group_by_colnumber.push(u)
                        }
                    }
                    false => {
                        return Err(format!("groupby column '{}' not found in datafile.", p));
                    }
                },
            }
        }

        let mut groups: Vec<Group> = vec![];
        let mut groupsvec: Vec<String> = vec![];

        for (i, g) in group_by_colnumber.iter().enumerate() {
            if *g < 1 {
                return Err(format!(
                    "group_by '{}' is lower than 1,
       allowed is from 1 to length of number of columns of the data file.",
                    g
                ));
            }
            let k = *g - 1;
            if k > datatable.len() - 1 {
                return Err(format!(
                    "group_by '{}' is greater than the numbers of parameters,
       allowed is from 1 to length of number of columns of the data file.",
                    g
                ));
            }
            let data = datatable.get(&k).unwrap();
            if i == 0 {
                for j in &data.vals {
                    groupsvec.push(format!["{}={}", data.name, j]);
                }
            } else {
                for it in data.vals.iter().zip(groupsvec.iter_mut()) {
                    let (ai, bi) = it;
                    bi.push_str(format![", {}={}", data.name, ai].as_ref());
                }
            }
        }

        let mut splits = vec![];

        for k in groupsvec.iter() {
            if !splits.contains(k) {
                splits.push(k.clone());
            }
        }

        for (i, k) in splits.iter().enumerate() {
            let mut indices: Vec<usize> = vec![];
            for (m, n) in groupsvec.iter().enumerate() {
                if n == k {
                    indices.push(m);
                }
            }

            groups.push(Group {
                name: format!["{number:>width$}>", number = i + 1, width = 2],
                group: k.clone(),
                indices,
            });
        }

        Ok(Groups { groups })
    }
}
