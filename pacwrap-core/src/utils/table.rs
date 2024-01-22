/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fmt::{Display, Error as FmtError, Formatter};

use dialoguer::console::Term;

use crate::{
    constants::{BOLD, BOLD_YELLOW, RESET, YELLOW},
    err,
    impl_error,
    utils::whitespace,
    Error,
    ErrorTrait,
};

#[derive(Debug)]
pub enum TableError {
    Empty,
    NoColumns,
}

impl_error!(TableError);

impl Display for TableError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            TableError::Empty => write!(fmt, "Table is empty"),
            TableError::NoColumns => write!(fmt, "Table contains no columns"),
        }
    }
}

pub struct Entry<'a> {
    contents: &'a str,
    whitespace: &'a str,
    width: usize,
    position: usize,
    margin: usize,
    overflow: bool,
}

pub struct Table<'a> {
    rows: Vec<Vec<String>>,
    columns: Vec<Vec<Entry<'a>>>,
    margins: Vec<usize>,
    marker: Vec<usize>,
    column_max: Vec<usize>,
    width_max: usize,
    whitespace: String,
    spacing: usize,
    dimensions: (usize, usize),
    built: bool,
}

impl<'a> Table<'a> {
    pub fn new() -> Self {
        let width = Term::size(&Term::stdout()).1 as usize;

        Self {
            rows: Vec::new(),
            columns: Vec::new(),
            margins: Vec::new(),
            marker: Vec::new(),
            column_max: Vec::new(),
            width_max: width,
            whitespace: whitespace(width),
            spacing: 2,
            dimensions: (0, 0),
            built: false,
        }
    }

    pub fn mark(&mut self, col: usize) {
        self.marker.push(col);
    }

    pub fn spacing(mut self, spacing: usize) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn col_width(mut self, col: usize, width: usize) -> Self {
        if self.column_max.len() <= self.dimensions.1 {
            for i in 0 .. self.dimensions.1 {
                self.column_max.insert(i, 0);
            }
        }

        self.column_max.insert(col, width);
        self
    }

    pub fn marked(&self) -> bool {
        self.marker.len() > 0
    }

    pub fn header(mut self, vec: &'a Vec<&'a str>) -> Self {
        self.rows.push(vec.iter().map(|a| a.to_string()).collect());
        self.dimensions = (self.rows.len(), vec.len());
        self
    }

    pub fn insert(&mut self, vec: Vec<String>) -> usize {
        self.rows.push(vec);
        self.dimensions = (self.rows.len(), self.dimensions.1);
        self.rows.len() - 1
    }

    pub fn build(&'a mut self) -> Result<&Self, Error> {
        if let (0, 0) = self.dimensions {
            err!(TableError::Empty)?
        } else if let (_, 0) = self.dimensions {
            err!(TableError::NoColumns)?
        }

        for row in 0 .. self.dimensions.0 {
            for col in 0 .. self.dimensions.1 {
                let item = match self.rows[row].get(col) {
                    Some(val) => match self.column_max.get(col) {
                        Some(max) =>
                            if *max > 0 && max < &val.len() {
                                Entry::new(val.split_at(*max).0)
                            } else {
                                Entry::new(val)
                            },
                        None => Entry::new(val),
                    },
                    None => Entry::new(""),
                };
                let margin = match self.margins.get(col) {
                    Some(margin) => *margin,
                    None => {
                        self.margins.insert(col, 0);
                        0
                    }
                };

                if margin <= item.width {
                    self.margins[col] = item.width
                }

                match self.columns.get_mut(col) {
                    Some(vec) => vec.push(item),
                    None => self.columns.insert(col, vec![item]),
                }
            }
        }

        for row in 0 .. self.dimensions.0 {
            let mut position = 0;

            for col in 0 .. self.dimensions.1 {
                let margin = self.margins[col];
                let item = &mut self.columns[col][row];

                item.margin = margin + self.spacing;
                item.position = position;
                item.overflow = self.width_max <= (margin + position + self.spacing);

                if !item.overflow && item.width <= margin {
                    item.whitespace = self.whitespace.split_at(item.margin - item.width).0;
                }

                if position + (self.spacing * col) >= self.width_max {
                    item.contents = "";
                } else if item.overflow {
                    let truncate = position + (self.spacing * col);
                    let truncate_idx = self.width_max - truncate;

                    if item.width >= truncate_idx {
                        item.contents = item.contents.split_at(truncate_idx).0;
                    }
                }

                position += margin;
            }
        }

        self.built = true;
        Ok(self)
    }
}

impl<'a> Entry<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            contents: content,
            width: content.len(),
            whitespace: "",
            position: 0,
            margin: 0,
            overflow: false,
        }
    }
}

impl<'a> Display for Table<'a> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), FmtError> {
        if !self.built {
            return writeln!(fmt, "Table object not built");
        }

        if self.columns[0][0].overflow && self.dimensions.1 > 1 {
            return writeln!(fmt, "{}warning:{} Insufficient terminal columns available to display table.", *BOLD_YELLOW, *RESET);
        }

        for row in 0 .. self.dimensions.0 {
            let marker = self.marker.contains(&row);
            let first = row == 0;
            let reset = marker || first;

            if first {
                write!(fmt, "{}", *BOLD)?;
            } else if marker {
                write!(fmt, "{}", *YELLOW)?
            }

            for col in 0 .. self.dimensions.1 {
                if let None = self.columns.get(col) {
                    continue;
                }

                if let Some(item) = self.columns[col].get(row) {
                    write!(fmt, "{}{}", item.contents, item.whitespace)?;
                }
            }

            match reset {
                true => writeln!(fmt, "{}", *RESET),
                false => writeln!(fmt),
            }?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    static RESULT: &'static str = "[1mLorem              ipsum     dolor                        sit[0m\nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \nLorem ipsum dolor  sit amet  consectetur adipiscing elit  sed do eiusmod tempor \n";
    static TEST_DATA: [&'static str; 5] = [
        "Lorem ipsum dolor",
        "sit amet",
        "consectetur adipiscing elit",
        "sed do eiusmod tempor incididunt ut labore et dolore magna aliqua",
        "Ut enim ad minim veniam",
    ];

    #[test]
    fn width_80() {
        let header = vec!["Lorem", "ipsum", "dolor", "sit", "amet"];
        let test_data = TEST_DATA.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        let mut table = Table {
            rows: Vec::new(),
            columns: Vec::new(),
            margins: Vec::new(),
            marker: Vec::new(),
            column_max: Vec::new(),
            width_max: 80,
            whitespace: whitespace(80),
            spacing: 2,
            dimensions: (0, 0),
            built: false,
        }
        .header(&header);

        for _ in 0 .. 10 {
            table.insert(test_data.clone());
        }

        let table = table.build().unwrap();
        assert_eq!(RESULT, &table.to_string());
    }
}
