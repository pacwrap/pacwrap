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

pub struct Remove;

impl HelpObject for Remove {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let sub = layout.sub();
        let bold = layout.bold();
        let sub_sect = layout.sub_section();
        let sub_bold = layout.sub_bold();
        let sub_para = layout.sub_paragraph();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();

        writeln!(
            buf,
            "{head}REMOVE{reset}
{sub_para}Remove packages from specified containers.

{sub_bold}-s, --recursive{reset_bold}
{tab}{tab}Recursively remove all target packages with the associated target container. This does
{tab}{tab}not apply to packages upstream of a downstream container.

{sub_bold}-c, --cascade{reset_bold}
{tab}{tab}Remove all target packages with the associated target container, including all their 
{tab}{tab}associated dependencies, provided they are not required by other packages, and are not
{tab}{tab}marked as being upstream of the target container.

{sub_bold}-t, --target{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Specify a target container for the specified operation. At least one container target is 
{tab}{tab}is required for package removal operations.

{sub_bold}--force-foreign{reset_bold}
{tab}{tab}Force the removal of foreign packages on target container. Useful for cleaning up
{tab}{tab}the package database of foreign, upstream dependencies synchronized to the target
{tab}{tab}container's package database.

{sub_bold}-m, --delete{reset_bold}
{tab}{tab}Delete root filesystem(s) of specified targets. Shortcout to {bold}-Ur{reset_bold}.

{sub_bold}-p, --preview{reset_bold}
{tab}{tab}Preview operation and perform no transaction.

{sub_bold}--dbonly{reset_bold}
{tab}{tab}Transact on resident containers with a database-only transaction.

{sub_bold}--noconfirm{reset_bold}
{tab}{tab}Override confirmation prompts and confirm all operations.

{sub_bold}--disable-sandbox{reset_bold}
{tab}{tab}Instruct libalpm to disable its own sandbox, utilizing landlock and seccomp, in order to mitigate potential
{tab}{tab}issues with kernel compatibillity.

{sub_bold}--debug{reset_bold}
{tab}{tab}Use this option when reporting bugs.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap -Rt firefox firefox`
{tab}{tab}Remove the target package firefox from target container firefox.

{sub}`$ pacwrap rm firefox`
{tab}{tab}Delete the root filesystem for the firefox container.\n"
        )
    }
}
