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

pub struct Process;

impl HelpObject for Process {
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
            "{head}PROCESS{reset}
{sub_para}Table a process list of running containers. Enumeration may be filtered by a specified predicate.
{tab}With invocation of the `ps` command-verb, the {bold}`-s, --summary`{reset_bold} option is assumed for convenience sake.
{tab}Flow is otherwise be specified with invocation of the {bold}`-P, --process`{reset_bold} short and long opts.

{sub_bold}-s, --summary{reset_bold}
{tab}{tab}Enumerate a process summary of containers instantiated by pacwrap.

{sub_bold}-i, --id-list{reset_bold}
{tab}{tab}Enumerate a process id list of containers instantiated by pacwrap. 

{sub_bold}-k, --kill{reset_bold}
{tab}{tab}Kill target containers and their associated processes.

{sub_bold}--noconfirm{reset_bold}
{tab}{tab}Override confirmation prompts and confirm all operations.

{sub_sect}Display Options{reset_bold}
{tab}{tab}These command-line arguments toggle visible columns. By default, the ID and Container columns are
{tab}{tab}present and cannot be toggled.

{sub_bold}-c, --command{reset_bold}
{tab}{tab}Display the Command column. When used without {bold}`-x, --exec`{reset_bold}, the executable name will be prepended
{tab}{tab}to the Command arguments row, with it otherwise being ommitted.

{sub_bold}-x, --exec{reset_bold}
{tab}{tab}Display the executable column; only the executable name is shown herein. This command-line flag
{tab}{tab}toggles column separation between the executable and command columns.

{sub_sect}Enumeration Options{reset_bold}
{sub_para}These options apply to both {bold}`-i, --id-list`{reset_bold} and {bold}`-s, --summary`{reset_bold} operations. Use these options
{tab}to modify enumeration predicates, or filter output by target or depth. Enumeration depth has
{tab}a default value of `1`.

{sub_bold}-t, --target{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Specify a target container and enumerate their associated processes.

{sub_bold}-a, --all{reset_bold}
{tab}{tab}Target all containers and enumerate their associated processes.

{sub_bold}-d, --depth{reset_bold}
{tab}{tab}Enumerate all processes at the specified depth associated with running containers.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap -Psaxc`
{tab}{tab}Print table enumerating all container processes to {bold}STDOUT{reset_bold} with process arguments
{tab}{tab}and execution path split into separate columns.

{sub}`$ ps up \"$(pacwrap -Pia)\"`
{tab}{tab}Enumerate container processes with `ps` via encapsulating an enumeration of pids from all instances
{tab}{tab}into a space-delimited bash string.\n"
        )
    }
}
