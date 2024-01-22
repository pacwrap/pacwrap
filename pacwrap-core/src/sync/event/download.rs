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

use std::collections::HashMap;

use alpm::{AnyDownloadEvent, DownloadEvent as Event};
use dialoguer::console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use lazy_static::lazy_static;
use simplebyteunit::simplebyteunit::*;

use crate::{
    config::global::ProgressKind,
    constants::{ARROW_CYAN, BOLD, RESET},
    sync::transaction::TransactionMode,
};

use super::whitespace;

lazy_static! {
    static ref INIT: ProgressStyle = ProgressStyle::with_template(" {spinner:.green} {msg}").unwrap();
    static ref UP_TO_DATE: ProgressStyle = ProgressStyle::with_template(" {spinner:.green} {msg} is up-to-date!")
        .unwrap()
        .progress_chars("#-")
        .tick_strings(&[" ", "✓"]);
}

#[derive(Clone)]
pub struct DownloadEvent {
    total: usize,
    position: usize,
    total_bar: Option<ProgressBar>,
    condensed: bool,
    progress: MultiProgress,
    bars: HashMap<String, ProgressBar>,
    style: Option<ProgressStyle>,
}

impl DownloadEvent {
    pub fn new() -> Self {
        Self {
            total: 0,
            position: 0,
            total_bar: None,
            condensed: false,
            progress: MultiProgress::new(),
            bars: HashMap::new(),
            style: None,
        }
    }

    pub fn style(mut self, kind: &ProgressKind) -> Self {
        self.style = match kind {
            ProgressKind::Simple => None,
            _ => Some({
                let size = Term::size(&Term::stdout());
                let width = ((size.1 / 2) - 14).to_string();

                ProgressStyle::with_template(
                    &(" {spinner:.green} {msg:<".to_owned()
                        + &width
                        + "} {bytes:>11} {bytes_per_sec:>12} {elapsed_precise:>5} [{wide_bar}] {percent:<3}%"),
                )
                .unwrap()
                .progress_chars("#-")
                .tick_strings(&[" ", "✓"])
            }),
        };
        self
    }

    pub fn total(mut self, bytes: u64, files: usize) -> Self {
        self.total = files;
        self.total_bar = match (bytes > 0 && files > 1, self.style.as_ref()) {
            (true, Some(style)) => Some({
                let bar = ProgressBar::new(bytes).with_style(style.clone()).with_message(format!(
                    "Total ({}0/{})",
                    whitespace(files, 1),
                    files
                ));
                let bar = self.progress.add(bar);

                bar.set_position(0);
                bar
            }),
            _ => None,
        };

        self
    }

    pub fn configure(mut self, mode: &TransactionMode, progress: &ProgressKind) -> Self {
        self.condensed = match progress {
            ProgressKind::Condensed => true,
            ProgressKind::CondensedForeign => match mode {
                TransactionMode::Foreign => true,
                TransactionMode::Local => false,
            },
            ProgressKind::CondensedLocal => match mode {
                TransactionMode::Foreign => false,
                TransactionMode::Local => true,
            },
            _ => false,
        };
        self
    }

    fn increment(&mut self, progress: u64) {
        let bar = match self.total_bar.as_mut() {
            Some(bar) => bar,
            None => return,
        };

        self.position += 1;

        let total = self.total;
        let pos = self.position;
        let whitespace = whitespace(total, pos);

        bar.inc(progress);
        bar.set_message(format!("Total ({}{}/{})", whitespace, pos, total));

        if total == pos {
            bar.finish();
        }
    }

    fn insert(&mut self, file: &str) {
        let pb = match self.total_bar.as_mut() {
            Some(total) => match self.condensed {
                true => self.progress.insert_after(&total, ProgressBar::new(0)),
                false => self.progress.insert_before(&total, ProgressBar::new(0)),
            },
            None => self.progress.add(ProgressBar::new(0)),
        };

        pb.set_style(INIT.clone());
        pb.set_message(message(file));
        self.bars.insert(file.to_owned(), pb);
    }
}

pub fn simple(file: &str, download: AnyDownloadEvent, this: &mut DownloadEvent) {
    if file.ends_with(".sig") {
        return;
    }

    if let Event::Completed(progress) = download.event() {
        this.position += 1;

        let size = progress.total.abs().to_byteunit(SI);
        let total = this.total;
        let pos = this.position;
        let whitespace = whitespace(total, pos);
        let message = message(file);

        eprintln!(
            "{} ({}{whitespace}{pos}{}/{}{total}{}) {message} downloaded ({size})",
            *ARROW_CYAN, *BOLD, *RESET, *BOLD, *RESET
        );
    }
}

pub fn event(file: &str, download: AnyDownloadEvent, this: &mut DownloadEvent) {
    if file.ends_with(".sig") {
        return;
    }

    match download.event() {
        Event::Progress(progress) =>
            if let Some(pb) = this.bars.get_mut(file) {
                if pb.length().unwrap() == 0 {
                    pb.set_length(progress.total.unsigned_abs());
                    pb.set_style(this.style.as_ref().unwrap().clone());
                }

                pb.set_position(progress.downloaded.unsigned_abs());
            },
        Event::Completed(progress) =>
            if let Some(pb) = this.bars.remove(file) {
                if pb.length().unwrap() == 0 {
                    pb.set_style(UP_TO_DATE.clone());
                }

                pb.finish();
                this.increment(progress.total.unsigned_abs());

                if this.condensed {
                    pb.set_draw_target(ProgressDrawTarget::hidden());
                }
            },
        Event::Init(progress) => {
            if progress.optional {
                return;
            }

            this.insert(file);
        }
        Event::Retry(_) =>
            if let Some(pb) = this.bars.get_mut(file) {
                pb.set_position(0);
                pb.set_style(INIT.clone());
            },
    }
}

pub fn callback(progress: &ProgressKind) -> for<'a, 'b, 'c> fn(file: &'a str, AnyDownloadEvent<'b>, this: &'c mut DownloadEvent) {
    match progress {
        ProgressKind::Simple => simple,
        _ => event,
    }
}

fn message(filename: &str) -> String {
    let name: Vec<&str> = filename.split(".pkg.tar.").collect();
    let mut msg_name: String = name[0].to_string();

    if msg_name.ends_with(".db") {
        msg_name.truncate(msg_name.len() - 3);
    }

    msg_name
}
