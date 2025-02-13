/*
 * pacwrap
 *
 * Copyright (C) 2023-2025 Xavier Moffett <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This program is free software: you can redistribute it and/or modify
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

use std::fmt::{Result, Write};

use crate::help::{HelpLayout, HelpObject};

pub struct List;

impl HelpObject for List {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let sub = layout.sub();
        let sub_sect = layout.sub_section();
        let sub_bold = layout.sub_bold();
        let sub_para = layout.sub_paragraph();
        let bold = layout.bold();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();

        writeln!(
            buf,
            "{head}LIST{reset}
{sub_para}List all initialized containers presently managed by pacwrap. 

{sub_para}This command module is a shortcut to {bold}-Ul{reset_bold}. Command verb {bold}`ls`{reset_bold} also is a
{tab}{tab}shortcut to this command module.

{sub_bold}-t, --total{reset_bold}
{tab}{tab}Display a total column.

{sub_bold}-o, --on-disk{reset_bold}
{tab}{tab}Display a size on disk column.

{sub_bold}-b, --bytes{reset_bold}
{tab}{tab}Toggle byte unit display.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap -Ld`
{tab}{tab}Print container tabulation out to {bold}STDOUT{reset_bold} with two total columns, one listing the
{tab}{tab}container name, and the other detailing the total size-on-disk consumption displayed with byteunits.

{sub}`$ pacwrap ls -btbts`
{tab}{tab}Print container tabulation to {bold}STDOUT{reset_bold} with three total columns, first listing the
{tab}{tab}container name, second the total amount of bytes, and the last showing the total with byteunits. 
{tab}{tab}Then print a summation of total, actual consumption below.\n"
        )
    }
}
