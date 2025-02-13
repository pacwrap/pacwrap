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

use indexmap::IndexSet;
use std::fmt::{Display, Formatter, Result as FmtResult};

use pacwrap_core::{
    err,
    impl_error,
    utils::{ansi::is_color_terminal, arguments::Operand, Arguments},
    Error,
    ErrorTrait,
    Result,
};

use crate::help::{
    config::PacwrapYml,
    manual::{
        compose::Compose,
        default::Default,
        desktop::Desktop,
        env::Environment,
        execute::Execute,
        list::List,
        meta::*,
        process::Process,
        query::Query,
        remove::Remove,
        sync::Synchronization,
        utils::Utils,
    },
};

mod config;
mod manual;
mod version;

pub use version::print_version;

static HELP_ALL: [HelpTopic; 14] = [
    HelpTopic::Execute,
    HelpTopic::Sync,
    HelpTopic::Remove,
    HelpTopic::Compose,
    HelpTopic::Query,
    HelpTopic::Process,
    HelpTopic::List,
    HelpTopic::Desktop,
    HelpTopic::Utils,
    HelpTopic::Version,
    HelpTopic::Help,
    HelpTopic::Env,
    HelpTopic::Authors,
    HelpTopic::License,
];

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

pub fn help(args: &mut Arguments, topic: &HelpTopic) -> Result<()> {
    let help = ascertain_help(args, topic)?;
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

fn ascertain_help<'a>(args: &'a mut Arguments, default_ht: &'a HelpTopic) -> Result<(IndexSet<&'a HelpTopic>, &'a HelpLayout)> {
    let mut layout = match is_color_terminal() {
        true => &HelpLayout::Console,
        false => &HelpLayout::Dumb,
    };
    let mut topic: Vec<&HelpTopic> = vec![default_ht];
    let mut more = false;

    if let HelpTopic::Default = default_ht {
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
    } else if args.next().is_some() {
        args.invalid_operand()?;
    }

    let len = topic.len();
    let start = if more || len == 1 || len > 7 { 0 } else { 1 };

    args.set_index(1);
    Ok((topic.drain(start ..).collect(), layout))
}

trait HelpObject {
    fn topic(buf: &mut String, layout: &HelpLayout) -> FmtResult;
}

#[derive(Eq, PartialEq, Hash)]
pub enum HelpTopic {
    Sync,
    Remove,
    Compose,
    Execute,
    Default,
    Query,
    Utils,
    Process,
    Desktop,
    List,
    Help,
    Version,
    Env,
    Authors,
    License,
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
            "d" | "desktop" => &HelpTopic::Desktop,
            "h" | "help" => &HelpTopic::Help,
            "env" | "environment" => &HelpTopic::Env,
            "author" | "authors" => &HelpTopic::Authors,
            "license" => &HelpTopic::License,
            "synopsis" => &HelpTopic::Default,
            "pacwrap.yml" => &HelpTopic::PacwrapYml,
            _ => err!(ErrorKind::InvalidTopic(str.into()))?,
        })
    }

    fn write(&self, buf: &mut String, layout: &HelpLayout) -> FmtResult {
        match self {
            Self::Default => Default::topic(buf, layout),
            Self::Sync => Synchronization::topic(buf, layout),
            Self::Remove => Remove::topic(buf, layout),
            Self::Execute => Execute::topic(buf, layout),
            Self::Process => Process::topic(buf, layout),
            Self::Desktop => Desktop::topic(buf, layout),
            Self::Version => Version::topic(buf, layout),
            Self::Env => Environment::topic(buf, layout),
            Self::Compose => Compose::topic(buf, layout),
            Self::Utils => Utils::topic(buf, layout),
            Self::List => List::topic(buf, layout),
            Self::Help => Meta::topic(buf, layout),
            Self::Query => Query::topic(buf, layout),
            Self::Authors => Authors::topic(buf, layout),
            Self::License => License::topic(buf, layout),
            Self::PacwrapYml => PacwrapYml::topic(buf, layout),
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
    let tag = env!("PACWRAP_BUILDTAG");

    if head.is_empty() | !tag.is_empty() {
        format!("{version} ({date})")
    } else {
        format!("{version}-{head}-{release} ({date})")
    }
}
