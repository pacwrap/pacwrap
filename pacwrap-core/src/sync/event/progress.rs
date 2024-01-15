/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
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

use alpm::Progress as Event;
use dialoguer::console::Term;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    config::global::ProgressKind,
    constants::{ARROW_CYAN, BOLD, RESET},
    sync::{
        event::whitespace,
        transaction::{TransactionMode, TransactionType},
    },
};

#[derive(Clone)]
pub struct ProgressEvent {
    current: Option<String>,
    progress: Option<ProgressBar>,
    offset: usize,
    style: Option<ProgressStyle>,
}

impl ProgressEvent {
    pub fn new() -> Self {
        Self {
            current: None,
            progress: None,
            style: None,
            offset: 0,
        }
    }

    pub fn configure(mut self, state: &TransactionType) -> Self {
        self.offset = state.pr_offset();
        self
    }

    pub fn style(mut self, kind: &ProgressKind) -> Self {
        self.style = match kind {
            ProgressKind::Simple => None,
            _ => Some({
                let size = Term::size(&Term::stdout());
                let width = (size.1 / 2).to_string();

                ProgressStyle::with_template(&(" {spinner:.green} {msg:<".to_owned() + &width + "} [{wide_bar}] {percent:<3}%"))
                    .unwrap()
                    .progress_chars("#-")
                    .tick_strings(&[" ", "✓"])
            }),
        };
        self
    }

    fn bar(&mut self, ident: Option<String>, name: &str, howmany: usize, current: usize, size: usize) {
        let pos = current + self.offset;
        let total = howmany + self.offset;
        let whitespace = whitespace(total, pos);
        let progress = ProgressBar::new(size as u64);

        progress.set_message(format!("({}{whitespace}{pos}{}/{}{total}{}) {name}", *BOLD, *RESET, *BOLD, *RESET));
        progress.set_style(self.style.as_ref().unwrap().clone());

        self.progress = Some(progress);
        self.current = ident
    }
}

pub fn event(event: Event, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
    let ident = ident(event, pkgname);

    match this.current.as_deref() {
        Some(current_ident) =>
            if ident != current_ident {
                this.bar(Some(ident), &name(event, pkgname), howmany, current, 100)
            },
        None => this.bar(Some(ident), &name(event, pkgname), howmany, current, 100),
    };

    let progress = match this.progress.as_mut() {
        Some(progress) => progress,
        None => return,
    };

    progress.set_position(percent as u64);

    if percent == 100 {
        progress.finish();
    }
}

pub fn simple(progress: Event, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
    if percent == 0 {
        let pos = current + this.offset;
        let total = howmany + this.offset;
        let progress_name: String = name(progress, pkgname);
        let whitespace = whitespace(total, pos);

        eprintln!("{} ({}{whitespace}{pos}{}/{}{total}{}) {progress_name}", *ARROW_CYAN, *BOLD, *RESET, *BOLD, *RESET);

        if current == howmany {
            eprintln!(
                "{} ({}{whitespace}{pos}{}/{}{total}{}) Synchronization complete",
                *ARROW_CYAN, *BOLD, *RESET, *BOLD, *RESET
            );
        }
    }
}

pub fn condensed(kind: Event, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
    if let Event::AddStart | Event::RemoveStart | Event::UpgradeStart = kind {
        let pos = current + this.offset;
        let total = howmany + this.offset;
        let progress_name: String = name(kind, pkgname);
        let whitespace = whitespace(total, pos);

        if let Some(_) = this.current {
            this.bar(None, &progress_name, howmany, current, howmany);
        }

        let progress = match this.progress.as_mut() {
            Some(progress) => progress,
            None => return,
        };

        progress.set_position(current as u64);

        if current != howmany {
            progress.set_message(format!("({}{whitespace}{pos}{}/{}{total}{}) {progress_name}", *BOLD, *RESET, *BOLD, *RESET));
        } else {
            progress.set_message(format!(
                "({}{whitespace}{pos}{}/{}{total}{}) Synchronization complete",
                *BOLD, *RESET, *BOLD, *RESET
            ));
            progress.finish();
        }
    } else {
        event(kind, pkgname, percent, howmany, current, this)
    }
}

fn name(progress: Event, pkgname: &str) -> String {
    match progress {
        Event::UpgradeStart => format!("Upgrading {}", pkgname),
        Event::AddStart => format!("Installing {}", pkgname),
        Event::RemoveStart => format!("Removing {}", pkgname),
        Event::DowngradeStart => format!("Downgrading {}", pkgname),
        Event::ReinstallStart => format!("Reinstalling {}", pkgname),
        Event::KeyringStart => format!("Loading keyring"),
        Event::IntegrityStart => format!("Checking integrity"),
        Event::LoadStart => format!("Loading packages"),
        Event::ConflictsStart => format!("Checking conflicts"),
        Event::DiskspaceStart => format!("Checking available diskspace"),
    }
}

fn ident(progress: Event, pkgname: &str) -> String {
    match progress {
        Event::KeyringStart => "keyring",
        Event::IntegrityStart => "integrity",
        Event::LoadStart => "loadstart",
        Event::ConflictsStart => "conflicts",
        _ => pkgname,
    }
    .to_owned()
}

pub fn callback(
    state: &TransactionMode,
    kind: &ProgressKind,
) -> for<'a, 'b> fn(Event, &'a str, i32, usize, usize, &'b mut ProgressEvent) {
    match kind {
        ProgressKind::Simple => simple,
        ProgressKind::CondensedForeign => match state {
            TransactionMode::Local => event,
            TransactionMode::Foreign => condensed,
        },
        ProgressKind::CondensedLocal => match state {
            TransactionMode::Local => condensed,
            TransactionMode::Foreign => event,
        },
        ProgressKind::Condensed => condensed,
        ProgressKind::Verbose => event,
    }
}
