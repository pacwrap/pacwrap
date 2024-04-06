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

use indexmap::IndexSet;
use lazy_static::lazy_static;
use std::fmt::{Display, Formatter, Result as FmtResult};

use pacwrap_core::{
    err,
    impl_error,
    utils::{arguments::Operand, is_color_terminal, Arguments},
    Error,
    ErrorTrait,
    Result,
};

mod config;
mod manual;
pub mod version;

pub use version::print_version;

lazy_static! {
    static ref HELP_ALL: Vec<HelpTopic> = [
        HelpTopic::Execute,
        HelpTopic::Sync,
        HelpTopic::Remove,
        HelpTopic::Compose,
        HelpTopic::Query,
        HelpTopic::Process,
        HelpTopic::List,
        HelpTopic::Utils,
        HelpTopic::Version,
        HelpTopic::Help,
        HelpTopic::Env,
        HelpTopic::Copyright
    ]
    .into();
}

#[derive(Debug)]
enum ErrorKind {
    InvalidTopic(String),
}

impl_error!(ErrorKind);

impl Display for ErrorKind {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::InvalidTopic(err) => write!(fmter, "Topic '{}' is not available.", err),
        }?;

        write!(fmter, "\nTry 'pacwrap -h' for more information on valid operational parameters.")
    }
}

pub fn help(mut args: &mut Arguments) -> Result<()> {
    let help = ascertain_help(&mut args)?;
    let mut buffer = String::new();

    for topic in help.0 {
        topic.write(&mut buffer, help.1).unwrap();
    }

    if let HelpLayout::Console = help.1 {
        print!("\x1b[?7l{}\x1b[?7h", buffer)
    } else {
        print!("{}", buffer)
    }

    Ok(())
}

fn ascertain_help<'a>(args: &'a mut Arguments) -> Result<(IndexSet<&'a HelpTopic>, &'a HelpLayout)> {
    let mut layout = match is_color_terminal() {
        true => &HelpLayout::Console,
        false => &HelpLayout::Dumb,
    };
    let mut topic: Vec<&HelpTopic> = vec![&HelpTopic::Default];
    let mut more = false;

    while let Some(arg) = args.next() {
        match arg {
            Operand::Long("format") | Operand::Long("help") | Operand::Short('f') | Operand::Short('h') => continue,
            Operand::Short('m') | Operand::Long("more") => more = true,
            Operand::ShortPos('f', "man") | Operand::LongPos("format", "man") => layout = &HelpLayout::Man,
            Operand::ShortPos('f', "ansi") | Operand::LongPos("format", "ansi") => layout = &HelpLayout::Console,
            Operand::ShortPos('f', "dumb") | Operand::LongPos("format", "dumb") => layout = &HelpLayout::Dumb,
            Operand::ShortPos('f', "markdown") | Operand::LongPos("format", "markdown") => layout = &HelpLayout::Markdown,
            Operand::ShortPos('h', "all")
            | Operand::LongPos("help", "all")
            | Operand::Short('a')
            | Operand::Long("all")
            | Operand::Value("all") => topic.extend(HELP_ALL.iter()),
            Operand::ShortPos('h', value) | Operand::LongPos("help", value) | Operand::Value(value) =>
                topic.push(HelpTopic::from(value)?),
            _ => args.invalid_operand()?,
        }
    }

    let len = topic.len();
    let start = if more || len == 1 || len > 7 { 0 } else { 1 };

    args.set_index(1);
    Ok((topic.drain(start ..).collect(), layout))
}

#[derive(Eq, PartialEq, Hash)]
enum HelpTopic {
    Sync,
    Remove,
    Compose,
    Execute,
    Default,
    Query,
    Utils,
    Process,
    List,
    Help,
    Version,
    Env,
    Copyright,
    PacwrapYml,
}

impl HelpTopic {
    fn from(str: &str) -> Result<&Self> {
        Ok(match str {
            "E" | "exec" | "run" => &HelpTopic::Execute,
            "S" | "sync" | "init" => &HelpTopic::Sync,
            "P" | "process" | "ps" => &HelpTopic::Process,
            "L" | "list" | "ls" => &HelpTopic::List,
            "U" | "utils" => &HelpTopic::Utils,
            "R" | "remove" => &HelpTopic::Remove,
            "C" | "compose" => &HelpTopic::Compose,
            "Q" | "query" => &HelpTopic::Query,
            "V" | "version" => &HelpTopic::Version,
            "h" | "help" => &HelpTopic::Help,
            "env" | "environment" => &HelpTopic::Env,
            "copyright" => &HelpTopic::Copyright,
            "synopsis" => &HelpTopic::Default,
            "pacwrap.yml" => &HelpTopic::PacwrapYml,
            _ => err!(ErrorKind::InvalidTopic(str.into()))?,
        })
    }

    fn write(&self, buf: &mut String, layout: &HelpLayout) -> FmtResult {
        match self {
            Self::Default => manual::default(buf, layout),
            Self::Sync => manual::sync(buf, layout),
            Self::Remove => manual::remove(buf, layout),
            Self::Execute => manual::execute(buf, layout),
            Self::Process => manual::process(buf, layout),
            Self::Version => manual::version(buf, layout),
            Self::Env => manual::environment(buf, layout),
            Self::Compose => manual::compose(buf, layout),
            Self::Utils => manual::utils(buf, layout),
            Self::List => manual::list(buf, layout),
            Self::Help => manual::meta(buf, layout),
            Self::Query => manual::query(buf, layout),
            Self::Copyright => manual::copyright(buf, layout),
            Self::PacwrapYml => config::default(buf, layout),
        }
    }
}

enum HelpLayout {
    Man,
    Dumb,
    Markdown,
    Console,
}

impl HelpLayout {
    fn head(&self) -> &str {
        match self {
            Self::Console => "[1m",
            Self::Markdown => "## ",
            Self::Man => ".SH\n",
            Self::Dumb => "",
        }
    }

    fn sub_bold(&self) -> &str {
        match self {
            Self::Console => "    [37;1m",
            Self::Markdown => "#### **",
            Self::Man => ".TP\n\\fB",
            Self::Dumb => "    ",
        }
    }

    fn sub(&self) -> &str {
        match self {
            Self::Markdown => "#### ",
            Self::Man => ".TP\n",
            Self::Dumb | Self::Console => "    ",
        }
    }

    fn sub_section(&self) -> &str {
        match self {
            Self::Console => "  [1m",
            Self::Markdown => "### **",
            Self::Man => ".SS\n",
            Self::Dumb => "    ",
        }
    }

    fn sub_paragraph(&self) -> &str {
        match self {
            Self::Console | Self::Dumb => "    ",
            Self::Man => ".PP\n",
            Self::Markdown => "",
        }
    }

    fn tab(&self) -> &str {
        match self {
            Self::Console | Self::Dumb => "    ",
            Self::Markdown | Self::Man => "",
        }
    }

    #[allow(dead_code)]
    fn underline(&self) -> &str {
        match self {
            Self::Console => "[4m",
            Self::Man => "\n.I",
            Self::Markdown => "<ins>",
            Self::Dumb => "",
        }
    }

    #[allow(dead_code)]
    fn reset_underline(&self) -> &str {
        match self {
            Self::Console => "[0m",
            Self::Man => "\\fR",
            Self::Markdown => "</ins>",
            Self::Dumb => "",
        }
    }

    fn reset(&self) -> &str {
        match self {
            Self::Console => "[0m",
            Self::Man => "\\fR",
            Self::Markdown | Self::Dumb => "",
        }
    }

    fn reset_bold(&self) -> &str {
        match self {
            Self::Console => "[0m",
            Self::Man => "\\fR",
            Self::Markdown => "**",
            Self::Dumb => "",
        }
    }

    fn bold(&self) -> &str {
        match self {
            Self::Console => "[37;1m",
            Self::Man => "\\fB",
            Self::Markdown => "**",
            Self::Dumb => "",
        }
    }

    fn code(&self) -> &str {
        match self {
            Self::Console | Self::Dumb | Self::Man => "",
            Self::Markdown => "```",
        }
    }
}

fn version_string() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let release = env!("PACWRAP_BUILD");
    let head = env!("PACWRAP_BUILDHEAD");
    let date = env!("PACWRAP_BUILDSTAMP");

    if head.is_empty() {
        format!("{version} ({date})")
    } else {
        format!("{version}-{head}-{release} ({date})")
    }
}
