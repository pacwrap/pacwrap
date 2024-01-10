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

use std::process::Command;

use pacwrap_core::{config,
    exec::utils::handle_process,
    utils::arguments::{Arguments, Operand}, 
    ErrorKind, 
    error::*, 
    err}; 

fn save_configuration() -> Result<()> {
    err!(ErrorKind::Message("This function has been deprecated."))?
}

fn print_configuration(instance: &str) -> Result<()> {
    let ins = config::provide_new_handle(instance)?;
    let mut pkgs_string = String::new();
    let mut depends_string = String::new();

    println!("INSTANCE_CONFIG[{},0]={}", instance, ins.metadata().container_type());   

    for i in ins.metadata().dependencies() {
        depends_string.push_str(&format!("{} ", i));    
    }
    println!("INSTANCE_CONFIG[{},1]=\"{}\"", instance, depends_string);

    for i in ins.metadata().explicit_packages() {
        pkgs_string.push_str(&format!("{} ", i));
    }

    println!("INSTANCE_CONFIG[{},3]=\"{}\"", instance, pkgs_string);
    Ok(())
}

pub fn compat(args: &mut Arguments) -> Result<()> {
    match args.next().unwrap_or_default() {
        Operand::Short('s') | Operand::Long("save") => save_configuration(),
        Operand::Short('l') | Operand::Long("load") => print_configuration(args.target()?),
        _ => args.invalid_operand()
    }
}

pub fn execute_bash(executable: &'static str, args: &mut Arguments) -> Result<()> { 
    handle_process(&executable, Command::new(&executable)
        .args(args.values())
        .spawn())
}
