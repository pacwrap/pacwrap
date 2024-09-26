/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    io::ErrorKind::{Interrupted, NotConnected},
};

use dialoguer::{
    console::{style, Style},
    theme::ColorfulTheme,
    Input,
};

use crate::{
    constants::{BAR_RED, BOLD, RESET},
    err,
    impl_error,
    Error,
    ErrorGeneric,
    ErrorTrait,
    Result,
};

#[derive(Debug)]
pub enum PromptError {
    PromptInterrupted,
    PromptNotTerminal,
}

impl Display for PromptError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::PromptInterrupted => write!(fmter, "Prompt was interrupted."),
            Self::PromptNotTerminal => write!(fmter, "Input is not a terminal."),
        }
    }
}

impl_error!(PromptError);

pub fn prompt(prefix: &str, prompt: impl Into<String>, yn_prompt: bool) -> Result<bool> {
    let value = create_prompt(prompt.into(), prefix, yn_prompt)?;

    Ok(value.to_lowercase() == "y" || (yn_prompt && value.is_empty()))
}

fn create_prompt(message: String, prefix: &str, yn_prompt: bool) -> Result<String> {
    let prompt = match yn_prompt {
        true => ("[Y/n]", style(prefix.into()).blue().bold()),
        false => ("[y/N]", style(prefix.into()).red().bold()),
    };

    let theme = ColorfulTheme {
        success_prefix: style(prefix.into()).green().bold(),
        prompt_prefix: prompt.1,
        error_prefix: style(prefix.into()).red().bold(),
        prompt_suffix: style(prompt.0.to_string()).bold(),
        success_suffix: style(prompt.0.to_string()).bold(),
        prompt_style: Style::new(),
        values_style: Style::new(),
        ..ColorfulTheme::default()
    };
    let input: String = match Input::with_theme(&theme).with_prompt(message).allow_empty(true).interact_text() {
        Ok(prompt) => prompt,
        Err(error) => match error.kind() {
            Interrupted => err!(PromptError::PromptInterrupted)?,
            NotConnected => err!(PromptError::PromptNotTerminal)?,
            _ => Err(error).generic()?,
        },
    };

    Ok(input)
}

pub fn prompt_targets(targets: &[&str], ins_prompt: &str, yn_prompt: bool) -> Result<bool> {
    eprintln!("{} {}Container{}{}\n", *BAR_RED, *BOLD, if targets.len() > 1 { "s" } else { "" }, *RESET);

    for target in targets.iter() {
        eprint!("{} ", target);
    }

    eprintln!("\n");
    prompt("::", format!("{}{}", *BOLD, ins_prompt), yn_prompt)
}
