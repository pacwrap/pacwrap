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

pub struct Authors;
pub struct License;
pub struct Meta;
pub struct Version;

impl HelpObject for Meta {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let bold = layout.bold();
        let sub_bold = layout.sub_bold();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();
        let tab = layout.tab();

        writeln!(
             buf,
             "{head}HELP{reset}
{sub_bold}-h, --help{reset_bold} <{bold}TOPIC{reset_bold}>
{tab}{tab}Print the specified topic to {bold}STDOUT{reset_bold}.

{sub_bold}-m, --more{reset_bold}
{tab}{tab}When specifying a topic to display, show the default topic in addition to specified options.

{sub_bold}-f, --format{reset_bold} <{bold}FORMAT{reset_bold}>
{tab}{tab}Change output format of help in {bold}STDOUT{reset_bold}. Format options include: 'ansi', 'dumb', 'markdown', and 'man'. 
{tab}{tab}This option is for the express purposes of generating documentation at build time, and has little utility
{tab}{tab}outside the context of package maintenance. 'man' option produces troff-formatted documents for man pages.

{sub_bold}-a, --all, --help=all{reset_bold}
{tab}{tab}Display all help topics.\n"
    )
    }
}

impl HelpObject for Version {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let sub_bold = layout.sub_bold();
        let tab = layout.tab();
        let bold = layout.bold();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();

        writeln!(
            buf,
            "{head}VERSION{reset}
{sub_bold}-V, --version, --version=min{reset_bold}
{tab}{tab}Sends version information to {bold}STDOUT{reset_bold} with colourful ASCII art. 
{tab}{tab}The 'min' option provides a minimalistic output as is provided to non-colour terms.\n"
        )
    }
}

impl HelpObject for License {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let reset = layout.reset();

        writeln!(
            buf,
            "{head}LICENSE{reset}
{tab}This program may be freely redistributed under the terms of the GNU General Public License v3 only.\n"
        )
    }
}

impl HelpObject for Authors {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let reset = layout.reset();

        writeln!(
            buf,
            "{head}AUTHOR{reset}
{tab}Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
{tab}Copyright (C) 2024-2025 Pacwrap Contributors

{tab}To find a current list of contributors, visit the following link:
{tab}https://github.com/pacwrap/pacwrap/graphs/contributors\n"
        )
    }
}
