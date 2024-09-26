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

use std::path::Path;

use alpm::{AnyQuestion, Question::*};

use crate::{utils::prompt::prompt, ErrorGeneric};

pub fn callback(question: AnyQuestion, _: &mut ()) {
    match question.question() {
        Conflict(mut x) => {
            let pkg_a = x.conflict().package1().name();
            let pkg_b = x.conflict().package2().name();
            let prompt_string = format!("Conflict between {pkg_a} and {pkg_b}; Remove {pkg_b}?");

            match prompt("->", prompt_string, false).generic() {
                Ok(bool) => x.set_remove(bool),
                Err(err) => err.error(),
            }
        }
        Replace(x) => {
            let old = x.oldpkg().name();
            let new = x.newpkg().name();
            let prompt_string = format!("Replace package {old} with {new}?");

            match prompt("->", prompt_string, false).generic() {
                Ok(bool) => x.set_replace(bool),
                Err(err) => err.error(),
            }
        }
        Corrupted(mut x) => {
            let filepath = x.filepath();
            let filename = Path::new(filepath).file_name().unwrap().to_str().unwrap();
            let reason = x.reason();
            let prompt_string = format!("'{filename}': {reason}. Remove package?");

            match prompt("->", prompt_string, false).generic() {
                Ok(bool) => x.set_remove(bool),
                Err(err) => err.error(),
            }
        }
        ImportKey(mut x) => {
            let fingerprint = x.fingerprint();
            let name = x.uid();
            let prompt_string = format!("Import key {fingerprint}, \"{name}\" to keyring?");

            match prompt("->", prompt_string, false).generic() {
                Ok(bool) => x.set_import(bool),
                Err(err) => err.error(),
            }
        }
        _ => (),
    }
}
