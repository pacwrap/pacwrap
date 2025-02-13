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

pub struct Utils;

impl HelpObject for Utils {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let sub = layout.sub();
        let sub_para = layout.sub_paragraph();
        let sub_sect = layout.sub_section();
        let sub_bold = layout.sub_bold();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();
        let bold = layout.bold();

        writeln!(
            buf,
            "{head}UTILITIES{reset}
{sub_para}Miscellaneous utilities which provide helpful auxiliary functionality to aid in configuration and
{tab}maintenance of containers. Each utility is considered a command module and therefore can be shortcuted
{tab}with a command verb.

{sub_bold}-v, --view{reset_bold}
{tab}{tab}Invoke {bold}$EDITOR{reset_bold} to view file associated with pacwrap.

{sub_bold}-e, --edit{reset_bold}
{tab}{tab}Invoke {bold}$EDITOR{reset_bold} to edit file associated with pacwrap.

{sub_bold}-o, --open{reset_bold}
{tab}{tab}Invoke default file viewer on specified target's home or root directory.

{sub_bold}-s, --symlink{reset_bold}
{tab}{tab}Create a symbolic container.

{sub_bold}-r, --remove{reset_bold}
{tab}{tab}Delete a container(s) root filesystem.

{sub_sect}EDITOR OPTIONS{reset_bold}
{sub_para}These options are associated with the {bold}--edit{reset_bold} and {bold}--view{reset_bold} utility command modules.

{sub_bold}-c, --config{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Edit specified container configuration located in the pacwrap data directory. Defaults to
{tab}{tab}the primary configuration file: '{bold}$PACWRAP_CONFIG_DIR{reset_bold}/pacwrap.yml' if no option is otherwise
{tab}{tab}specified.

{sub_bold}-d, --desktop{reset_bold} <{bold}APPLICATION{reset_bold}>
{tab}{tab}Edit specified desktop file associated with a pacwrap container.

{sub_bold}-r, --repo{reset_bold}
{tab}{tab}Edit repositories configuration file: `$PACWRAP_CONFIG_DIR/repositories.conf`.

{sub_bold}-l, --log{reset_bold}
{tab}{tab}View 'pacwrap.log'. This file contains transaction log iformation.

{sub_sect}OPEN OPTIONS{reset_bold}
{sub_para}These options are associated with the {bold}--open{reset_bold} utility command module.

{sub_bold}-h, --home{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Specified container's home filesystem.

{sub_bold}-r, --root{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Specified container's root filesystem.

{sub_bold}-t, --target{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Target container to perform the operation.

{sub_sect}REMOVE OPTIONS{reset_bold}
{sub_para}These options are associated with the {bold}--remove{reset_bold} utility command module.

{sub_bold}-t, --target{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Target container to perform the operation.

{sub_bold}--noconfirm{reset_bold}
{tab}{tab}Peform the operation without confirmation.

{sub_bold}--force{reset_bold}
{tab}{tab}Disable sanity checks and force removal of conatiner filesystem.

{sub_sect}SYMBOLIC{reset_bold}
{sub_para}These options are associated with the {bold}--symlink{reset_bold} utility command module.

{sub_bold}<TARGET> <DEST>{reset_bold}
{tab}{tab}Create a symbolic container of target at destination.

{sub_bold}-n, --new{reset_bold}
{tab}{tab}Create a fresh configuration rather than derive it from the target.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap -Uoh firefox`
{tab}{tab}Open firefox's home directory in the default file manager.

{sub}`$ pacwrap -Uvl`
{tab}{tab}View `{bold}$PACWRAP_DATA_DIR{reset_bold}/pacwrap.log` with {bold}$EDITOR{reset_bold}.

{sub}`$ pacwrap -Uec firefox`
{tab}{tab}Edit `$PACWRAP_CONFIG_DIR{reset_bold}/container/firefox.yml` with {bold}$EDITOR{reset_bold}.

{sub}`$ pacwrap utils -dc firefox firefox`
{tab}{tab}Create desktop file `$HOME/.local/share/applications/pacwrap.firefox.desktop` derived from
{tab}{tab}`/usr/share/applications/firefox.desktop` in the root of the firefox container.

{sub}`$ pacwrap utils symlink java runelite`
{tab}{tab}Create a symbolic container called `runelite` of `java`.

{sub}`$ pacwrap -Uld`
{tab}{tab}Print container tabulation out to {bold}STDOUT{reset_bold} with two total columns, one listing the
{tab}{tab}container name, and the other detailing the total size-on-disk consumption displayed with byteunits.

{sub}`$ pacwrap utils -lbtbts`
{tab}{tab}Print container tabulation to {bold}STDOUT{reset_bold} with three total columns, first listing the
{tab}{tab}container name, second the total amount of bytes, and the last showing the total with byteunits. 
{tab}{tab}Then print a summation of total, actual consumption below.\n"
        )
    }
}
