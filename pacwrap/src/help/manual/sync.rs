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

pub struct Synchronization;

impl HelpObject for Synchronization {
    fn topic(buf: &mut String, layout: &HelpLayout) -> Result {
        let head = layout.head();
        let bold = layout.bold();
        let tab = layout.tab();
        let reset = layout.reset();
        let reset_bold = layout.reset_bold();
        let sub = layout.sub();
        let sub_bold = layout.sub_bold();
        let sub_sect = layout.sub_section();
        let sub_para = layout.sub_paragraph();

        writeln!(
             buf,
             "{head}SYNCHRONIZATION{reset}
{sub_para}Provides the facilities required to be able to synchronize and create containers in aggregate. 

{sub_bold}-y, --refresh{reset_bold}
{tab}{tab}Synchronize remote package databases. Specify up to 2 times to force a refresh.

{sub_bold}-u, --upgrade{reset_bold}
{tab}{tab}Execute aggregate upgrade routine on all or specified containers. Use {bold}`-t, --target[=CONTAINER]`{reset_bold} followed
{tab}{tab}by a list of packages to specify package targets. Packages applicable to a target {bold}must{reset_bold} only be specified 
{tab}{tab}after the target operand.

{sub_bold}-c, --create{reset_bold}
{tab}{tab}Create a container with the first specified target. A container type argument is also required. Command verb 
{tab}{tab}{bold}`init`{reset_bold} provides a shortcut to the synchronization module, equivalent to specifying the options {bold}`-Syuc`{reset_bold}.

{sub_bold}-b, --base{reset_bold}
{tab}{tab}Base container type. Specify alongside {bold}`-c, --create`{reset_bold} to assign this container type during creation.

{tab}{tab}This container type is used as the base layer for all downstream containers. Only one base container 
{tab}{tab}dependency per slice or aggregate is supported. Filesystem and package deduplication via slices and 
{tab}{tab}aggregate containers are recommended, but optional. This container type is not dependant.

{sub_bold}-s, --slice{reset_bold}
{tab}{tab}Slice container type. Specify alongside {bold}`-c, --create`{reset_bold} to assign this container type during creation.

{tab}{tab}Requires a base dependency, and optionally one or more sliced dependencies, to ascertain foreign
{tab}{tab}packages and influence ordering of downstream synchronization target(s). Container slicing provides
{tab}{tab}the ability to install packages in a lightweight, sliced filesytem, which aid in the deduplication 
{tab}{tab}of common downstream package and filesystem dependencies.

{tab}{tab}Useful for graphics drivers, graphical toolkits, fonts, etc.; these are not meant for applications.

{sub_bold}-a, --aggegrate{reset_bold}
{tab}{tab}Aggregate container type. Specify alongside {bold}`-c, --create`{reset_bold} to this assign container type during creation.

{tab}{tab}Requires a base dependency, and optionally one or more sliced dependencies, in order to acertain foreign
{tab}{tab}packages and amalgamate the target. These containers are ideal for installing software with the aid of
{tab}{tab}filesystem and package deduplication. 

{tab}{tab}Useful for all general purpose applications, browsers, e-mail clients, or even terminal user interface 
{tab}{tab}applications such as IRC clients. It is recommended to base your containers on aggregate type containers.

{sub_bold}-t, --target{reset_bold} <{bold}CONTAINER{reset_bold}> <..{bold}PACKAGE{reset_bold}>
{tab}{tab}Declare a target container for the specified operation, followed by a list of package target(s).

{sub_bold}-f, --filesystem{reset_bold}
{tab}{tab}Force execution of filesystem synchronization target on all or specified containers. In combination 
{tab}{tab}with {bold}-o/--target-only{reset_bold}, in addition to no other specified targets, filesystems will be synchronized 
{tab}{tab}without package synhcronization on on all applicable containers. This operation is useful for propagation 
{tab}{tab}of manual filesystem changes to all aggregate containers.

{sub_bold}-o, --target-only{reset_bold}
{tab}{tab}Apply specified operation on the specified target(s) only.

{sub_bold}-d, --dep{reset_bold} <{bold}CONTAINER{reset_bold}>
{tab}{tab}Specify dependencies for a container create operation.

{sub_bold}-p, --preview{reset_bold}
{tab}{tab}Perform a dryrun operation on existing containers to preview changes applicable or otherwise specified.
{tab}{tab}Only applicable to pre-existing targets and not create operations.

{sub_bold}-l, --lazy-load{reset_bold}
{tab}{tab}Enable lazy-database initialization for this transaction. {bold}NOTE{reset_bold}: This feature is experimental.
{tab}{tab}Edge cases exist wherein the use of {bold}`--force-foreign`{reset_bold} may be required.

{sub_bold}--force-foreign{reset_bold}
{tab}{tab}Force synchronization of foreign packages on resident container. Useful for when installing 
{tab}{tab}a new package in an aggregate container without all the prerequisite foreign dependencies
{tab}{tab}synchronized to the resident container's package database.

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
{sub}`$ pacwrap init --base --target base`
{tab}{tab}Synchronize remotes and create a base-type container named `base` with no additional packages.

{sub}`$ pacwrap -Syucst common gtk3 qt6-base --dep=base -st nvidia nvidia-utils --dep=base,common`
{tab}{tab}Synchronize remote databases, create two sliced containers, one named `common` with the packages 
{tab}{tab}`gtk3`, `qt6-base`, and another named `nvidia` with the package `nvidia-utils`.

{sub}`$ pacwrap -Syucat mozilla firefox --dep=base,common,nvidia`
{tab}{tab}Synchronize remote databases and upgrade container dependencies, then create aggregate container 
{tab}{tab}named `mozilla` with the package `firefox`.

{sub}`$ pacwrap -Sot mozilla thunderbird`
{tab}{tab}Install `thunderbird` in the target container `mozilla`.

{sub}`$ pacwrap -Sof`
{tab}{tab}Synchronize filesystem state of all associated containers present in the data directory.\n"
    )
    }
}
