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

use alpm::Alpm;
use dialoguer::console::Term;
use serde::{Deserialize, Serialize};
use simplebyteunit::simplebyteunit::*;

use crate::{
    constants::{BOLD, DIM, RESET},
    sync::transaction::TransactionMode,
    utils::table::{ColumnAttribute, Table},
};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum SummaryKind {
    Sum,
    Basic,
    Table,
    SumForeign,
    BasicForeign,
    TableForeign,
}

pub struct Summary {
    mode: TransactionMode,
    pkgs: usize,
    pkgs_upgraded: usize,
    pkgs_new: usize,
    elements: Vec<(String, String, String, i64, i64)>,
    installed: i64,
    net_installed: i64,
    removed: i64,
    download_size: i64,
    download_files: u64,
    kind: SummaryKind,
}

enum TableColumns {
    OldNewNetDownload,
    NewNetDownload,
    OldNewNet,
    OldNet,
    NewNet,
    Version,
}

impl Summary {
    pub fn new() -> Self {
        Self {
            mode: TransactionMode::Local,
            pkgs: 0,
            pkgs_upgraded: 0,
            pkgs_new: 0,
            elements: Vec::new(),
            installed: 0,
            removed: 0,
            download_size: 0,
            download_files: 0,
            net_installed: 0,
            kind: SummaryKind::default(),
        }
    }

    pub fn mode(mut self, mode: &TransactionMode) -> Self {
        self.mode = *mode;
        self
    }

    pub fn kind(mut self, kind: &SummaryKind, database_only: bool) -> Self {
        self.kind = match database_only {
            true => match kind {
                SummaryKind::Table => SummaryKind::TableForeign,
                SummaryKind::Basic => SummaryKind::BasicForeign,
                SummaryKind::Sum => SummaryKind::SumForeign,
                _ => *kind,
            },
            false => *kind,
        };
        self
    }

    pub fn generate(mut self, handle: &Alpm) -> Self {
        for pkg in handle.trans_remove() {
            let pkg = match handle.localdb().pkg(pkg.name()) {
                Ok(pkg) => pkg,
                Err(_) => continue,
            };
            let removed = pkg.isize();

            self.removed += removed;
            self.elements.push((pkg.name().into(), pkg.version().to_string(), "".into(), removed, 0));
        }

        for pkg_sync in handle.trans_add() {
            let (pkg_this, installed_new, is_installed) = match handle.localdb().pkg(pkg_sync.name()) {
                Ok(pkg) => (pkg, pkg.isize(), true),
                Err(_) => (pkg_sync, pkg_sync.isize(), false),
            };
            let installed_old = pkg_sync.isize();
            let size_dnl = pkg_sync.download_size();
            let this_net = installed_old - installed_new;

            if pkg_this.version() != pkg_sync.version() {
                self.pkgs_upgraded += 1;
            }

            if !is_installed {
                self.pkgs_new += 1;
            }

            self.installed += installed_old;
            self.net_installed += this_net;
            self.elements.push((
                pkg_this.name().into(),
                match is_installed {
                    true => pkg_this.version().to_string(),
                    false => String::new(),
                },
                pkg_sync.version().to_string(),
                match installed_old == installed_new && !is_installed {
                    false => this_net,
                    true => installed_old,
                },
                size_dnl,
            ));

            if size_dnl > 0 {
                self.download_size += size_dnl;
                self.download_files += 1;
            }
        }

        self.pkgs = handle.trans_add().len() + handle.trans_remove().len();
        self
    }

    pub fn download(&self) -> (u64, u64) {
        (self.download_size as u64, self.download_files)
    }

    fn columns(&self) -> (bool, bool, bool, bool, bool) {
        (self.removed == 0, self.pkgs_upgraded > 0, self.pkgs_new > 0, self.net_installed != 0, self.download_files > 0)
    }

    fn table(&self, fmt: &mut Formatter<'_>) -> Result<(), FmtError> {
        let preface = format!("Packages ({}) ", self.pkgs);
        let table_columns = TableColumns::from(self);
        let table_header = table_columns.header(&preface);
        let table_elements = self.elements.clone();
        let mut table = match table_columns {
            TableColumns::OldNewNetDownload => Table::new()
                .header(&table_header)
                .new_line()
                .col_attribute(3, ColumnAttribute::AlignRight)
                .col_attribute(4, ColumnAttribute::AlignRight),
            TableColumns::NewNetDownload => Table::new()
                .header(&table_header)
                .new_line()
                .col_attribute(2, ColumnAttribute::AlignRight)
                .col_attribute(3, ColumnAttribute::AlignRight),
            TableColumns::OldNewNet => Table::new()
                .header(&table_header)
                .new_line()
                .col_attribute(3, ColumnAttribute::AlignRight),
            TableColumns::OldNet | TableColumns::NewNet => Table::new()
                .header(&table_header)
                .new_line()
                .col_attribute(2, ColumnAttribute::AlignRight),
            _ => Table::new().header(&table_header).new_line(),
        };

        for (name, old, new, net, dnl) in table_elements {
            let net = format!("{}{}", net.to_byteunit(IEC), if net > -1024 && net < 1024 { "  " } else { "" });
            let dnl = match dnl == 0 {
                false => format!("{}{}", dnl.to_byteunit(IEC), if dnl < 1024 { "  " } else { "" }),
                true => String::new(),
            };

            table.insert(match table_columns {
                TableColumns::OldNewNetDownload => vec![name, old, new, net, dnl],
                TableColumns::NewNetDownload => vec![name, new, net, dnl],
                TableColumns::OldNewNet => vec![name, old, new, net],
                TableColumns::OldNet => vec![name, old, net],
                TableColumns::NewNet => vec![name, new, net],
                TableColumns::Version => vec![name, new],
            });
        }

        write!(fmt, "\n{}\n", table.build().unwrap())?;
        self.footer(fmt)
    }

    fn basic(&self, fmt: &mut Formatter<'_>) -> Result<(), FmtError> {
        let size = Term::size(&Term::stdout());
        let preface = format!("Packages ({}) ", self.pkgs);
        let preface_newline = " ".repeat(preface.len());
        let line_delimiter = size.1 as usize - preface.len();
        let mut pkglist: String = String::new();
        let mut current_line_len: usize = 0;

        write!(fmt, "\n{}{preface}{}", *BOLD, *RESET)?;

        for pkg in self.elements.iter() {
            let ver = if pkg.2.is_empty() { &pkg.1 } else { &pkg.2 };
            let string_len = pkg.0.len() + ver.len() + 2;

            if current_line_len + string_len >= line_delimiter {
                writeln!(fmt, "{}", pkglist)?;
                pkglist = preface_newline.clone();
                current_line_len = pkglist.len();
            }

            current_line_len += string_len;
            pkglist.push_str(&format!("{}-{}{}{} ", pkg.0, *DIM, ver, *RESET));
        }

        write!(fmt, "{}\n\n", pkglist)?;
        self.footer(fmt)
    }

    fn footer(&self, fmt: &mut Formatter<'_>) -> Result<(), FmtError> {
        if self.installed != 0 {
            writeln!(fmt, "{}Total Installed Size{}: {}", *BOLD, *RESET, self.installed.to_byteunit(IEC))?;
        }

        if self.removed != 0 {
            writeln!(fmt, "{}Total Removed Size{}: {}", *BOLD, *RESET, self.removed.to_byteunit(IEC))?;
        }

        if self.download_size > 0 {
            writeln!(fmt, "{}Total Download Size{}: {}", *BOLD, *RESET, self.download_size.to_byteunit(IEC))?;
        }

        if self.net_installed != 0 {
            writeln!(fmt, "{}Net Upgrade Size{}: {}", *BOLD, *RESET, self.net_installed.to_byteunit(IEC))?;
        }

        Ok(())
    }
}

impl Display for Summary {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), FmtError> {
        match (&self.kind, self.mode) {
            (SummaryKind::Basic, TransactionMode::Local) | (SummaryKind::BasicForeign, _) => self.basic(fmt),
            (SummaryKind::Table, TransactionMode::Local) | (SummaryKind::TableForeign, _) => self.table(fmt),
            (SummaryKind::Sum, TransactionMode::Local) | (SummaryKind::SumForeign, _) => self.footer(fmt),
            (_, TransactionMode::Foreign) => Ok(()),
        }
    }
}

impl Default for SummaryKind {
    fn default() -> Self {
        Self::Basic
    }
}

impl TableColumns {
    fn header<'a>(&'a self, preface: &'a str) -> Vec<&str> {
        match self {
            Self::OldNewNetDownload => vec![preface, "Old Version", "New Version", "Net Change", "Download Size"],
            Self::NewNetDownload => vec![preface, "New Version", "Net Change", "Download Size"],
            Self::OldNewNet => vec![preface, "Old Version", "New Version", "Net Change"],
            Self::OldNet => vec![preface, "Old Version", "Net Change"],
            Self::NewNet => vec![preface, "New Version", "Net Change"],
            Self::Version => vec![preface, "Version"],
        }
    }
}

impl From<&Summary> for TableColumns {
    fn from(sum: &Summary) -> Self {
        match sum.columns() {
            //Grr, don't try to figure out _how_ this works, just know that it does..
            (false, false, false, false, false) => Self::OldNet,
            (true, true, _, true, true) => Self::OldNewNetDownload,
            (true, true, _, true, false) => Self::OldNewNet,
            (.., true, false, true) => Self::NewNetDownload,
            (.., true, false, false) => Self::NewNet,
            _ => Self::Version,
        }
    }
}
