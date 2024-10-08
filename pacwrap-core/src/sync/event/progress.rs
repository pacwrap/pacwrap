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

use std::collections::HashMap;

use alpm::Progress as Event;
use dialoguer::console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::{
    config::global::ProgressKind,
    constants::{ARROW_CYAN, BOLD, RESET},
    sync::{
        event::whitespace,
        transaction::{TransactionMode, TransactionType},
    },
};

pub struct ProgressEvent {
    current: Option<String>,
    condensed: Option<ProgressBar>,
    style: Option<ProgressStyle>,
    offset: usize,
    progress: MultiProgress,
    bars: HashMap<String, ProgressBar>,
}

impl Default for ProgressEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressEvent {
    pub fn new() -> Self {
        Self {
            current: None,
            condensed: None,
            style: None,
            offset: 0,
            progress: MultiProgress::new(),
            bars: HashMap::new(),
        }
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

    pub fn configure(mut self, state: &TransactionType) -> Self {
        self.offset = state.pr_offset();
        self
    }

    fn insert(&mut self, ident: &str, name: &str, howmany: usize, current: usize, size: usize) -> &ProgressBar {
        let pos = current + self.offset;
        let total = howmany + self.offset;
        let whitespace = whitespace(total, pos);
        let pb = self.progress.add(ProgressBar::new(size as u64));

        pb.set_message(format!("({}{whitespace}{pos}{}/{}{total}{}) {name}", *BOLD, *RESET, *BOLD, *RESET));
        pb.set_style(self.style.as_ref().unwrap().clone());
        self.bars.insert(ident.to_owned(), pb);
        self.bars.get(ident).unwrap()
    }
}

pub fn event(event: Event, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
    let ident = ident(event, pkgname);
    let progress = match this.bars.get(ident) {
        Some(progress) => progress,
        None => match percent < 100 {
            false => return,
            true => this.insert(ident, &name(event, pkgname), howmany, current, 100),
        },
    };

    progress.set_position(percent as u64);

    if percent == 100 {
        if let Some(bar) = this.bars.remove(ident) {
            bar.finish();
        }
    }
}

pub fn simple(kind: Event, pkgname: &str, _: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
    if let Some(pkg) = this.current.as_deref() {
        if ident(kind, pkgname) != pkg {
            this.current = None;
        }
    }

    if this.current.as_deref().is_none() {
        let pos = current + this.offset;
        let total = howmany + this.offset;
        let progress_name: String = name(kind, pkgname);
        let whitespace = whitespace(total, pos);

        eprintln!("{} ({}{whitespace}{pos}{}/{}{total}{}) {progress_name}", *ARROW_CYAN, *BOLD, *RESET, *BOLD, *RESET);
        this.current = Some(ident(kind, pkgname).into());
    }
}

pub fn condensed(kind: Event, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
    if let Event::AddStart | Event::RemoveStart | Event::UpgradeStart = kind {
        let pos = current + this.offset;
        let total = howmany + this.offset;
        let progress_name: String = name(kind, pkgname);
        let whitespace = whitespace(total, pos);
        let progress = match this.condensed.as_mut() {
            Some(progress) => progress,
            None => {
                let progress = ProgressBar::new(howmany as u64);

                progress.set_style(this.style.as_ref().unwrap().clone());

                this.condensed = Some(progress);
                this.condensed.as_mut().unwrap()
            }
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
        Event::KeyringStart => "Loading keyring".into(),
        Event::IntegrityStart => "Checking integrity".into(),
        Event::LoadStart => "Loading packages".into(),
        Event::ConflictsStart => "Checking conflicts".into(),
        Event::DiskspaceStart => "Checking available diskspace".into(),
    }
}

fn ident(progress: Event, pkgname: &str) -> &str {
    match progress {
        Event::KeyringStart => "keyring",
        Event::IntegrityStart => "integrity",
        Event::LoadStart => "loadstart",
        Event::ConflictsStart => "conflicts",
        _ => pkgname,
    }
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
