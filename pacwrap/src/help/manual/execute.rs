/*
 * pacwrap
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
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

pub struct Execute;

impl HelpObject for Execute {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let reset = layout.reset();
        let bold = layout.bold();
        let sub = layout.sub();
        let sub_sect = layout.sub_section();
        let sub_bold = layout.sub_bold();
        let sub_para = layout.sub_paragraph();
        let reset_bold = layout.reset_bold();

        writeln!(
            buf,
            "{head}EXECUTE{reset}
{sub_para}Invoke a container to execute the provided command sequence. Command verb {bold}`run`{reset_bold} provides a 
{tab}shortcut to this module.

{sub_bold}<CONTAINER> <CMD>{reset_bold}
{tab}{tab}Container name to spawn an instance of, along with the proceeding command-line sequence to execute.
{tab}{tab}execute. All command-line parameters after the container name are passed through to execute inside
{tab}{tab}of the container environment.

{sub_bold}-s, --shell{reset_bold}
{tab}{tab}Invoke a bash shell in the target container. Command verb {bold}`shell`{reset_bold} provides a shortcut
{tab}{tab}to this module with this option.

{sub_bold}-r, --root{reset_bold}
{tab}{tab}Execute the provided command sequence with fakeroot and fakechroot.
	
{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap run firefox firefox`
{tab}{tab}Launch firefox inside an instance of the firefox container.

{sub}`$ pacwrap shell -r base`
{tab}{tab}Open a fakeroot bash shell inside an instance of the base container.\n"
        )
    }
}
