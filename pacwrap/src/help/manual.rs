/*
 * pacwrap
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
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

use std::fmt::{Result as FmtResult, Write};

use crate::help::{version_string, HelpLayout};

fn header(buf: &mut String, layout: &HelpLayout) -> FmtResult {
    let name = env!("CARGO_PKG_NAME");
    let date = env!("PACWRAP_BUILDSTAMP");

    match layout {
        HelpLayout::Man => writeln!(buf, ".nh\n.TH {name} 1 \"{date}\" \"{name} version_string_placeholder\" \"User Manual\"\n"),
        HelpLayout::Markdown => writeln!(
            buf,
            "# Pacwrap User Manual

This document was generated by the {name} binary with version {} of the program.\n",
            version_string()
        ),
        _ => Ok(()),
    }
}

pub fn default(buf: &mut String, layout: &HelpLayout) -> FmtResult {
    let head = layout.head();
    let tab = layout.tab();
    let sub_para = layout.sub_paragraph();
    let sub_bold = layout.sub_bold();
    let bold = layout.bold();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();

    header(buf, layout)?;
    writeln!(
        buf,
        "{head}NAME{reset}
{tab}pacwrap

{head}SYNOPSIS{reset}
{tab}pacwrap [{bold}OPERATION{reset_bold} | {bold}COMMAND MODULE{reset_bold}] [{bold}ARGUMENTS{reset_bold}] [{bold}TARGETS{reset_bold}]	

{head}DESCRIPTION{reset}
{sub_para}A package management front-end which utilises libalpm to facilitate the creation of unprivileged, 
{tab}namespace containers with parallelised, filesystem-agnostic deduplication. These containers
{tab}are constructed with bubblewrap to execute package transactions and launch applications.

{sub_para}This application is designed to allow for the creation and execution of secure, replicable 
{tab}containerised environments for general-purpose use. CLI and GUI applications are all supported. 
{tab}Once a container environment is configured, it can be re-established or replicated on any system. 

{head}OPERATIONS{reset}
{sub_bold}-S, --sync{reset_bold}
{tab}{tab}Synchronize package databases and update packages in target containers. 

{sub_bold}-R, --remove{reset_bold}
{tab}{tab}Remove packages from target containers.

{sub_bold}-Q, --query{reset_bold}
{tab}{tab}Query package information from target container.

{sub_bold}-C, --compose{reset_bold}
{tab}{tab}Compose a container from configuration.

{sub_bold}-P, --process{reset_bold}
{tab}{tab}Manage and show status of running container processes.

{sub_bold}-E, --execute{reset_bold}
{tab}{tab}Executes application in target container using bubblewrap.

{sub_bold}-L, --list{reset_bold}
{tab}{tab}List of available containers managed by pacwrap.

{sub_bold}-U, --utils{reset_bold}
{tab}{tab}Invoke miscellaneous utilities to manage containers.

{sub_bold}-h, --help=MODULE{reset_bold}
{tab}{tab}Invoke a printout of this manual to {bold}STDOUT{reset_bold}.

{sub_bold}-V, --version{reset_bold}
{tab}{tab}Display version and copyright information in {bold}STDOUT{reset_bold}.\n"
    )
}

pub fn execute(buf: &mut String, layout: &HelpLayout) -> FmtResult {
    let head = layout.head();
    let sub_bold = layout.sub_bold();
    let tab = layout.tab();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();

    writeln!(
        buf,
        "{head}EXECUTE{reset}
{sub_bold}-r, --root{reset_bold}
{tab}{tab}Execute operation with fakeroot and fakechroot. Facilitates a command with faked privileges.
	
{sub_bold}-s, --shell{reset_bold}
{tab}{tab}Invoke a bash shell\n"
    )
}

pub fn meta(buf: &mut String, layout: &HelpLayout) -> FmtResult {
    let head = layout.head();
    let bold = layout.bold();
    let sub_bold = layout.sub_bold();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();
    let tab = layout.tab();

    writeln!(
             buf,
             "{head}HELP{reset}
{sub_bold}-m, --more{reset_bold}
{tab}{tab}When specifying a topic to display, show the default topic in addition to specified options.

{sub_bold}-f, --format=FORMAT{reset_bold}
{tab}{tab}Change output format of help in {bold}STDOUT{reset_bold}. Format options include: 'ansi', 'dumb', 'markdown', and 'man'. 
{tab}{tab}This option is for the express purposes of generating documentation at build time, and has little utility
{tab}{tab}outside the context of package maintenance. 'man' option produces troff-formatted documents for man pages.

{sub_bold}-a, --all, --help=all{reset_bold}
{tab}{tab}Display all help topics.\n"
    )
}

pub fn sync(buf: &mut String, layout: &HelpLayout) -> FmtResult {
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
{tab}{tab}Execute aggregate upgrade routine on all or specified containers. Use {bold}-t, --target=TARGET{reset_bold} followed by
{tab}{tab}a list of packages to specify package targets. Packages applicable to a target {bold}must{reset_bold} only be specified 
{tab}{tab}after the target operand.

{sub_bold}-f, --filesystem{reset_bold}
{tab}{tab}Force execution of filesystem synchronization target on all or specified containers. In combination 
{tab}{tab}with {bold}-o/--target-only{reset_bold}, in addition to no other specified targets, filesystem slices will be synchronized 
{tab}{tab}without package synhcronization on on all applicable containers. This operation is useful for propagation 
{tab}{tab}of manual filesystem changes to all aggregate containers.

{sub_bold}-c, --create{reset_bold}
{tab}{tab}Create a container with the first specified target. A container type argument is also required.

{sub_bold}-b, --base{reset_bold}
{tab}{tab}Base container type. Specify alongside {bold}-c, --create{reset_bold} to assign this container type during creation.

{tab}{tab}This container type is used as the base layer for all downstream containers. Only one base container 
{tab}{tab}dependency per slice or aggregate is supported. Filesystem and package deduplication via slices and 
{tab}{tab}aggregate containers are recommended, but optional. This container type is not dependant.

{sub_bold}-s, --slice{reset_bold}
{tab}{tab}Slice container type. Specify alongside {bold}-c, --create{reset_bold} to assign this container type during creation.
{tab}{tab}Requires a base dependency, and optionally one or more sliced dependencies, to ascertain foreign
{tab}{tab}packages and influence ordering of downstream synchronization target(s). Container slicing provides
{tab}{tab}the ability to install packages in a lightweight, sliced filesytem, which aid in the deduplication 
{tab}{tab}of common downstream package and filesystem dependencies.

{tab}{tab}Useful for graphics drivers, graphical toolkits, fonts, etc.; these are not meant for applications.

{sub_bold}-a, --aggegrate{reset_bold}
{tab}{tab}Aggregate container type. Specify alongside {bold}-c, --create{reset_bold} to this assign container type during creation.
{tab}{tab}
{tab}{tab}Requires a base dependency, and optionally one or more sliced dependencies, in order to acertain foreign
{tab}{tab}packages and amalgamate the target. These containers are ideal for installing software with the aid of
{tab}{tab}filesystem and package deduplication. 
{tab}{tab}
{tab}{tab}Useful for all general purpose applications, browsers, e-mail clients, or even terminal user interface 
{tab}{tab}applications such as IRC clients. It is recommended to base your containers on aggregate type containers.

{sub_bold}-t, --target=TARGET{reset_bold}
{tab}{tab}Specify a target container for the specified operation.

{sub_bold}-d, --dep=DEPEND{reset_bold}
{tab}{tab}Specify a dependent container for the specified operation.

{sub_bold}-p, --preview{reset_bold}
{tab}{tab}Perform a dryrun operation on existing containers to preview changes applicable or otherwise specified.
{tab}{tab}Only applicable to 

{sub_bold}-o, --target-only{reset_bold}
{tab}{tab}Apply specified operation on the specified target(s) only.

{sub_bold}--force-foreign{reset_bold}
{tab}{tab}Force synchronization of foreign packages on resident container. Useful for when installing 
{tab}{tab}a new package in an aggregate container without all the prerequisite foreign dependencies
{tab}{tab}synchronized to this container's package database.

{sub_bold}--dbonly{reset_bold}
{tab}{tab}Transact on resident containers with a database-only transaction.

{sub_bold}--noconfirm{reset_bold}
{tab}{tab}Override confirmation prompts and confirm all operations.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap -Syucbt base`
{tab}{tab}Create a base container named base with no additional packages

{sub}`$ pacwrap -Syucbt firefox firefox --dep=base,common,nvidia`
{tab}{tab}Create aggregate container named firefox with firefox installed.

{sub}`$ pacwrap -Syut electron element-desktop -t mozilla firefox thunderbird`
{tab}{tab}Synchronize package databases and upgrade all containers, as well as install element-desktop 
{tab}{tab}in the target electron, and install firefox and thunderbird in the target mozilla.

{sub}`$ pacwrap -Syucst common gtk3 qt6-base --dep=base -cst nvidia nvidia-utils --dep=base,common`
{tab}{tab}Create two sliced containers, one named common with the packages gtk3, qt6-base, and another 
{tab}{tab}named nvidia with the package nvidia-utils.

{sub}`$ pacwrap -Sof`
{tab}{tab}Synchronize filesystem state of all associated containers present in the data directory.\n")
}

pub fn remove(buf: &mut String, layout: &HelpLayout) -> FmtResult {
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

{sub_bold}-t, --target=TARGET{reset_bold}
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

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap -Rt firefox firefox`
{tab}{tab}Remove the target package firefox from target container firefox.

{sub}`$ pacwrap rm firefox`
{tab}{tab}Delete the root filesystem for the firefox container.
\n"
    )
}

pub fn compose(buf: &mut String, layout: &HelpLayout) -> FmtResult {
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

{tab}{tab}Reinitialize container 

{sub_bold}-t, --target=TARGET{reset_bold}
{tab}{tab}Specify a target container for the specified operation.

{sub_bold}-f, --force{reset_bold}
{tab}{tab}Disable sanity checks and force removal of container filesystem(s).

{sub_bold}--reinitialize-all{reset_bold}
{tab}{tab}Queues all available, existing containers for composition. All pre-existing container roots
{tab}{tab}will be deleted and composited from the available configuration data enumerated.

{sub_bold}--from-config{reset_bold}
{tab}{tab}Instruct pacwrap to populate configuration data from uninitialized containers. Under normal
{tab}{tab}circumstances, configuration data will only be populated from containers with configuration
{tab}{tab}data and an associative container root present. This option engages an alternate enuermation 
{tab}{tab}pathway to allow composition of dormant, uninitialized container configurations.

{sub_bold}--noconfirm{reset_bold}
{tab}{tab}Override confirmation prompts and confirm all operations.

{sub_sect}EXAMPLES{reset_bold}
{sub}`$ pacwrap compose -rt element element.yml`
{tab}{tab}Reinitialize an existing container named element with its configuration derived 
{tab}{tab}from the file 'element.yml'.

{sub}`$ pacwrap compose --reinitialize-all --from-config`
{tab}{tab}Reinitialize all available containers as configured in '{bold}$PACWRAP_CONFIG_DIR{reset_bold}/container/'.\n"
    )
}

pub fn query(buf: &mut String, layout: &HelpLayout) -> FmtResult {
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
{sub_para}Query package list on target container. This module presently is not complete.

{sub_bold}-q, --quiet{reset_bold}
{tab}{tab}Quiet the output by truncating the package string.

{sub_bold}-t, --target=TARGET{reset_bold}
{tab}{tab}Specify a target container for the specified operation.

{sub_bold}-e, --explicit{reset_bold}
{tab}{tab}Filter output to explicitly-marked packages.

{sub_sect}EXAMPLE{reset_bold}
{sub}`$ pacwrap -Qqe base`
{tab}{tab}Print a list of explicit packages from the {bold}base{reset_bold} container to {bold}STDOUT{reset_bold}.\n"
    )
}

pub fn process(buf: &mut String, layout: &HelpLayout) -> FmtResult {
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
{sub_para}Table a process list of running containers. Containers may be filtered on target and process depth.

{sub_bold}-s, --summary{reset_bold}
{tab}{tab}Enumerate a process summary of containers being executed by pacwrap.

{sub_bold}-k, --kill{reset_bold}
{tab}{tab}Kill target containers and their associated processes.

{sub_bold}-a, --all{reset_bold}
{tab}{tab}Enumerate all processes associated with running containers.

{sub_bold}-d, --depth{reset_bold}
{tab}{tab}Enumerate all processes at the specified depth associated with running containers.

{sub_bold}-t, --target=TARGET{reset_bold}
{tab}{tab}Specify a target container for the specified operation.

{sub_bold}--noconfirm{reset_bold}
{tab}{tab}Override confirmation prompts and confirm all operations.

{sub_sect}EXAMPLE{reset_bold}
{sub}`$ pacwrap -Psaxc`
{tab}{tab}Print table enumerating all container processes to {bold}STDOUT{reset_bold} with process
{tab}{tab}arguments and execution path split into separate columns.\n"
    )
}

pub fn utils(buf: &mut String, layout: &HelpLayout) -> FmtResult {
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
{tab}maintenance of operate containers.

{sub_bold}-v, --view{reset_bold}
{tab}{tab}Invoke {bold}$EDITOR{reset_bold} to view file associated with pacwrap.

{sub_bold}-e, --edit{reset_bold}
{tab}{tab}Invoke {bold}$EDITOR{reset_bold} to edit file associated with pacwrap.

{sub_bold}-o, --open{reset_bold}
{tab}{tab}Invoke default file viewer on specified target's home or root directory.

{sub_bold}-l, --list{reset_bold}
{tab}{tab}Print a list of containers and basic metrics.

{sub_bold}-s, --symlink{reset_bold}
{tab}{tab}Create a symbolic container.

{sub_bold}-r, --remove{reset_bold}
{tab}{tab}Delete a container(s) root filesystem.

{sub_sect}EDITOR OPTIONS{reset_bold}
{sub_para}These options are associated with the {bold}--edit{reset_bold} and {bold}--view{reset_bold} utility command modules.

{sub_bold}-c, --config=CONTAINER{reset_bold}
{tab}{tab}Edit specified container configuration located in the pacwrap data directory. Defaults to
{tab}{tab}the primary configuration file: '{bold}$PACWRAP_CONFIG_DIR{reset_bold}/pacwrap.yml' if no option is otherwise
{tab}{tab}specified.

{sub_bold}-d, --desktop=APPLICATION{reset_bold}
{tab}{tab}Edit specified desktop file associated with a pacwrap container.

{sub_bold}-r, --repo{reset_bold}
{tab}{tab}Edit repositories configuration file: '{bold}$PACWRAP_CONFIG_DIR{reset_bold}/repositories.conf'.

{sub_bold}-l, --log{reset_bold}
{tab}{tab}View 'pacwrap.log'. This file contains transaction log information.

{sub_sect}OPEN OPTIONS{reset_bold}
{sub_para}These options are associated with the {bold}--open{reset_bold} utility command module.

{sub_bold}-h, --home=CONTAINER{reset_bold}
{tab}{tab}Specified container's home filesystem.

{sub_bold}-r, --root=CONTAINER{reset_bold}
{tab}{tab}Specified container's root filesystem.

{sub_bold}-t, --target=CONTAINER{reset_bold}
{tab}{tab}Target container to perform the operation.

{sub_sect}LIST{reset_bold}
{sub_para}These options are associated with the {bold}--list{reset_bold} utility command module.

{sub_bold}-t, --total{reset_bold}
{tab}{tab}Display a total column.

{sub_bold}-d, --on-disk{reset_bold}
{tab}{tab}Display a size on disk column.

{sub_bold}-s, --summary{reset_bold}
{tab}{tab}Print out a summary table to {bold}STDOUT{reset_bold}.

{sub_bold}-b, --bytes{reset_bold}
{tab}{tab}Toggle byte unit display for the proceeding item.

{sub_sect}REMOVE OPTIONS{reset_bold}
{sub_para}These options are associated with the {bold}--remove{reset_bold} utility command module.

{sub_bold}-t, --target{reset_bold}
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
{sub}`$ pacwrap -Ulbtbts`
{tab}{tab}Print table listing containers out to {bold}STDOUT{reset_bold} with two total columns, one showing
{tab}{tab}the total amount of bytes. Then print a summary calculation of total consumption below.

{sub}`$ pacwrap -Uvl`
{tab}{tab}View '{bold}$PACWRAP_DATA_DIR{reset_bold}/pacwrap.log' with {bold}$EDITOR{reset_bold}.\n"
    )
}

pub fn list(buf: &mut String, layout: &HelpLayout) -> FmtResult {
    let head = layout.head();
    let tab = layout.tab();
    let sub = layout.sub();
    let sub_sect = layout.sub_section();
    let sub_bold = layout.sub_bold();
    let sub_para = layout.sub_paragraph();
    let bold = layout.bold();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();

    writeln!(
        buf,
        "{head}LIST{reset}
{sub_para}List all initialized containers presently managed by pacwrap. 

{sub_para}This command module is a shortcut to {bold}-Ul{reset_bold}.

{sub_bold}-t, --total{reset_bold}
{tab}{tab}Display a total column.

{sub_bold}-o, --on-disk{reset_bold}
{tab}{tab}Display a size on disk column.

{sub_bold}-b, --bytes{reset_bold}
{tab}{tab}Toggle byte unit display.

{sub_sect}EXAMPLE{reset_bold}
{sub}`$ pacwrap ls -btbts`
{tab}{tab}Print table listing containers out to {bold}STDOUT{reset_bold} with two total columns, one showing
{tab}{tab}the total amount of bytes. Then print a summary calculation of total consumption below.\n"
    )
}

pub fn environment(buf: &mut String, layout: &HelpLayout) -> FmtResult {
    let head = layout.head();
    let sub_bold = layout.sub_bold();
    let tab = layout.tab();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();
    let sub_para = layout.sub_paragraph();

    writeln!(
        buf,
        "{head}ENVIRONMENT VARIABLES{reset}
{sub_para}Provided herein are environment variables of which can be used to configure pacwrap's runtime parameters.
{tab}{tab}Use with care: These variables if used improperly could result in undesired behaviour.

{sub_bold}PACWRAP_CONFIG_DIR{reset_bold}
{tab}{tab}Set the configuration directory. This environment variable overrides the default, 
{tab}{tab}XDG Directory Specification compliant path. 

{sub_bold}PACWRAP_DATA_DIR{reset_bold}
{tab}{tab}Set the data directory. This environment variable overrides the default, 
{tab}{tab}XDG Directory Specification compliant path. 

{sub_bold}PACWRAP_CACHE_DIR{reset_bold}
{tab}{tab}Set the cache directory. This environment variable overrides the default, 
{tab}{tab}XDG Directory Specification compliant path. 

{sub_bold}PACWRAP_VERBOSE=[0|1]{reset_bold}
{tab}{tab}Toggle verbose output during a transaction. This option may be removed or otherwise
{tab}{tab}differ in functionality in future.

{sub_bold}PACWRAP_HOME{reset_bold}
{tab}{tab}Upon execution, mount the set path provided when engaging the 'home' module.

{sub_bold}PACWRAP_ROOT{reset_bold}
{tab}{tab}Upon execution, Mount the set path provided when engaging the 'root' module.\n"
    )
}

pub fn version(buf: &mut String, layout: &HelpLayout) -> FmtResult {
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

pub fn copyright(buf: &mut String, layout: &HelpLayout) -> FmtResult {
    let head = layout.head();
    let tab = layout.tab();
    let reset = layout.reset();

    writeln!(
        buf,
        "{head}COPYRIGHT{reset}
{tab}Copyright (C) 2023-2024 Xavier R.M.

{tab}This program may be freely redistributed under the
{tab}terms of the GNU General Public License v3 only.\n"
    )
}
