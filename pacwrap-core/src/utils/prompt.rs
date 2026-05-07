/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
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

use dialoguer::{
    Input,
    console::{Style, style},
    theme::ColorfulTheme,
};

use crate::{
    Result,
    constants::{BAR_RED, BOLD, RESET},
};

pub fn prompt(prefix: impl AsRef<str>, prompt: impl AsRef<str>, yn_prompt: bool) -> Result<bool> {
    let value = create_prompt(prompt.as_ref(), prefix.as_ref(), yn_prompt)?;

    Ok(value.to_lowercase() == "y" || (yn_prompt && value.is_empty()))
}

fn create_prompt<T: AsRef<str>>(message: T, prefix: T, yn_prompt: bool) -> Result<String> {
    let prefix = prefix.as_ref();
    let prompt = match yn_prompt {
        true => ("[Y/n]", style(prefix.into()).blue().bold()),
        false => ("[y/N]", style(prefix.into()).red().bold()),
    };

    let theme = ColorfulTheme {
        prompt_prefix: prompt.1,
        success_prefix: style(prefix.into()).green().bold(),
        error_prefix: style(prefix.into()).red().bold(),
        prompt_suffix: style(prompt.0.into()).bold(),
        success_suffix: style(prompt.0.into()).bold(),
        prompt_style: Style::new(),
        values_style: Style::new(),
        ..ColorfulTheme::default()
    };
    Ok(Input::with_theme(&theme)
        .with_prompt(message.as_ref())
        .allow_empty(true)
        .interact_text()?)
}

pub fn prompt_targets(targets: &[&str], ins_prompt: &str, yn_prompt: bool) -> Result<bool> {
    eprintln!("{} {}Container{}{}\n", *BAR_RED, *BOLD, if targets.len() > 1 { "s" } else { "" }, *RESET);

    for target in targets.iter() {
        eprint!("{} ", target);
    }

    eprintln!("\n");
    prompt("::", format!("{}{}", *BOLD, ins_prompt), yn_prompt)
}
