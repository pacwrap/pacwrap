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

pub struct Desktop;

impl HelpObject for Desktop {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let sub = layout.sub();
        let sub_para = layout.sub_paragraph();
        let sub_bold = layout.sub_bold();
        let sub_sect = layout.sub_section();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();
        let bold = layout.bold();

        writeln!(
            buf,
            "{head}DESKTOP{reset}
{sub_para}Create and manage desktop files to launch applications in pacwrap from your favourite applications menu.

{sub_bold}-c, --create{reset_bold} <{bold}CONTAINER{reset_bold}> <{bold}APPLICATION{reset_bold}>
{tab}{tab}Create desktop file at `$HOME/.local/share/applications/` launching an associated container with pacwrap.

{sub_bold}-l, --list{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Enumerate available desktop files in the container root located at `/usr/share/applications/`
{tab}{tab}or `HOME/.local/share/applications/`.

{sub_bold}-r, --remove{reset_bold} <{bold}APPLICATION{reset_bold}>
{tab}{tab}Remove desktop file associated with application from `$HOME/.local/share/applications/`.

{sub_bold}-f, --find{reset_bold} <{bold}PREDICATE{reset_bold}>
{tab}{tab}Filter desktop list enumeration based on a {bold}predicate{reset_bold}.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap desktop list firefox`
{tab}{tab}Print tabulation of desktop files in the `firefox` container root.

{sub}`$ pacwrap desktop list --find libreoffice`
{tab}{tab}Tabulate all entries with the predicate `libreoffice` from `$HOME/.local/share/applications/`.

{sub}`$ pacwrap desktop create firefox firefox`
{tab}{tab}Install desktop file from the contaienr root into `$HOME/.local/share/applications/`.\n"
        )
    }
}
