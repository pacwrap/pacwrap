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

pub struct Compose;

impl HelpObject for Compose {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let tab = layout.tab();
        let bold = layout.bold();
        let sub_bold = layout.sub_bold();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();
        let sub = layout.sub();
        let sub_sect = layout.sub_section();

        writeln!(
            buf,
            "{head}COMPOSE{reset}
{tab}Compose containers from container configuration files. This functionality provides a way
{tab}to deterministically compose containers from an established configuration.

{sub_bold}<FILE_PATH>{reset_bold}
{tab}{tab}Compose a container from the specified configuration file on disk. Unless a target is
{tab}{tab}otherwise specified, the container will be initialized with a name derived from the
{tab}{tab}filename provided.

{sub_bold}-r, --reinitialize{reset_bold}
{tab}{tab}Compose an available, existing container for composition. The pre-existing container root
{tab}{tab}will be deleted and the container will be composited from the configuration data enumerated.

{sub_bold}-t, --target{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Specify a target container for the specified operation.

{sub_bold}-f, --force{reset_bold}
{tab}{tab}Disable sanity checks and force removal of container filesystem(s).

{sub_bold}--reinitialize-all{reset_bold}
{tab}{tab}Queues all available, existing containers for composition. All pre-existing container roots
{tab}{tab}will be deleted and composited from the available configuration data enumerated.

{sub_bold}-l, --lazy-load{reset_bold}
{tab}{tab}Enable lazy-database initialization for this transaction. {bold}NOTE{reset_bold}: This feature is experimental.
{tab}{tab}Edge cases exist wherein the use of {bold}`--force-foreign`{reset_bold} may be required.

{sub_bold}--from-config{reset_bold}
{tab}{tab}Instruct pacwrap to populate configuration data from uninitialized containers. Under normal
{tab}{tab}circumstances, configuration data will only be populated from containers with configuration
{tab}{tab}data and an associative container root present. This option engages an alternate enuermation 
{tab}{tab}pathway to allow composition of dormant, uninitialized container configurations.

{sub_bold}--noconfirm{reset_bold}
{tab}{tab}Override confirmation prompts and confirm all operations.

{sub_bold}--disable-sandbox{reset_bold}
{tab}{tab}Instruct libalpm to disable its own sandbox, utilizing landlock and seccomp, in order to mitigate potential
{tab}{tab}issues with kernel compatibillity.

{sub_bold}--debug{reset_bold}
{tab}{tab}Use this option when reporting bugs.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap compose -rt element element.yml`
{tab}{tab}Reinitialize an existing container named element with its configuration derived 
{tab}{tab}from the file 'element.yml'.

{sub}`$ pacwrap compose --reinitialize-all --from-config`
{tab}{tab}Reinitialize all container configurations available in '{bold}$PACWRAP_CONFIG_DIR{reset_bold}/container/'.\n"
        )
    }
}
