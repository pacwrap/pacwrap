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

pub struct Environment;

impl HelpObject for Environment {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let bold = layout.bold();
        let tab = layout.tab();
        let sub_sect = layout.sub_section();
        let sub_bold = layout.sub_bold();
        let sub_para = layout.sub_paragraph();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();

        writeln!(
            buf,
            "{head}ENVIRONMENT{reset}
{sub_para}Provided herein are environment variables of which can be used to configure pacwrap's runtime parameters.
{tab}All environment variables listed are case sensitive.

{sub_para}Use with care: These variables if used improperly could result in undesired behaviour.

{sub_bold}PACWRAP_CONFIG_DIR{reset_bold} <{bold}DIR{reset_bold}>
{tab}{tab}Set path of the configuration directory, overriding the default location.

{sub_bold}PACWRAP_DATA_DIR{reset_bold} <{bold}DIR{reset_bold}>
{tab}{tab}Set path of the data directory, overriding the default location.

{sub_bold}PACWRAP_CACHE_DIR{reset_bold} <{bold}DIR{reset_bold}> 
{tab}{tab}Set path of the cache directory, overriding the default location.

{sub_bold}PACWRAP_HOME{reset_bold} <{bold}DIR{reset_bold}>
{tab}{tab}Upon container invocation, mount the set path provided when engaging the {bold}`home`{reset_bold} filesystem module.

{sub_bold}PACWRAP_ROOT{reset_bold} <{bold}DIR{reset_bold}>
{tab}{tab}Upon container invocation, mount the set path provided when engaging the {bold}`root`{reset_bold} filesystem module.

{sub_bold}PACWRAP_VERBOSE{reset_bold} <{bold}0{reset_bold} | {bold}1{reset_bold}>
{tab}{tab}Toggle verbose output during a transaction. Valid options are `1` for enablement and `0` for 
{tab}{tab}disablement of verbosity.

{sub_sect}DEFAULT{reset_bold}
{sub_para}For the following environment variables, contained herein are default runtime values. Any variables not
{tab}included here in this subsection are to be assumed to have inert values by default.

{sub_bold}PACWRAP_CACHE_DIR{reset_bold}
{tab}{tab}`$HOME/.cache/pacwrap`: Default cache directory.

{sub_bold}PACWRAP_CONFIG_DIR{reset_bold}
{tab}{tab}`$HOME/.config/pacwrap`: Default configuration directory.

{sub_bold}PACWRAP_DATA_DIR{reset_bold}
{tab}{tab}`$HOME/.local/share/pacwrap`: Default data directory.\n"
        )
    }
}
