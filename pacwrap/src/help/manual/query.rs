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

pub struct Query;

impl HelpObject for Query {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let sub_bold = layout.sub_bold();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();
        let sub = layout.sub();
        let bold = layout.bold();
        let sub_sect = layout.sub_section();
        let sub_para = layout.sub_paragraph();

        writeln!(
            buf,
            "{head}QUERY{reset}
{sub_para}Query package list on target container.

{sub_bold}-q, --quiet{reset_bold}
{tab}{tab}Quiet the output by truncating the package string.

{sub_bold}-t, --target{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Specify a target container for the specified operation.

{sub_bold}-e, --explicit{reset_bold}
{tab}{tab}Filter output to explicitly-marked packages.

{sub_sect}EXAMPLE{reset_bold}
{sub}`$ pacwrap -Qqe base`
{tab}{tab}Print a list of explicit packages from the {bold}base{reset_bold} container to {bold}STDOUT{reset_bold}.\n"
        )
    }
}
