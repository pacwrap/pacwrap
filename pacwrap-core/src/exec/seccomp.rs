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

use std::os::fd::AsRawFd;

use libseccomp::{
    ScmpAction as Action,
    ScmpArch,
    ScmpArgCompare as Compare,
    ScmpCompareOp as Op,
    ScmpFilterContext,
    ScmpSyscall as Syscall,
};
use nix::libc;
use os_pipe::{PipeReader, PipeWriter};

use crate::config::container::ContainerRuntime;

use self::FilterType::*;

#[derive(PartialEq)]
pub enum FilterType {
    Namespaces,
    TtyControl,
    Standard,
}

static EPERM: Action = Action::Errno(libc::EPERM);
static ENOSYS: Action = Action::Errno(libc::ENOSYS);

/*
 * Personality values obtained from personality.h in the Linux kernel
 *
 * https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/uapi/linux/personality.h
 */
static PERSONALITY: u64 = if cfg!(target_pointer_width = "64") {
    0x0000
} else {
    0x0000 | 0x0800000
};

/*
 * Syscall blocklists derived from flatpak-run.c in the flatpak project.
 *
 * https://github.com/flatpak/flatpak/blob/main/common/flatpak-run.c#L1835
 *
 * Please do not open issue reports, esplicitly regarding lessened security, regarding filters
 * that of which can be toggled. When the relevant options are activated, users are warned of
 * the potential ramifications of so doing.
 *
 * This encumbers a great responsibility upon the user when exercising this great power.
 */
static RULES: [(FilterType, &'static str, Action); 28] = [
    (Standard, "syslog", EPERM),
    (Standard, "uselib", EPERM),
    (Standard, "acct", EPERM),
    (Standard, "quotactl", EPERM),
    (Standard, "add_key", EPERM),
    (Standard, "keyctl", EPERM),
    (Standard, "request_key", EPERM),
    (Standard, "move_pages", EPERM),
    (Standard, "mbind", EPERM),
    (Standard, "get_mempolicy", EPERM),
    (Standard, "set_mempolicy", EPERM),
    (Standard, "migrate_pages", EPERM),
    (Standard, "clone3", ENOSYS),
    (Standard, "open_tree", ENOSYS),
    (Standard, "move_mount", ENOSYS),
    (Standard, "fsopen", ENOSYS),
    (Standard, "fsconfig", ENOSYS),
    (Standard, "fsmount", ENOSYS),
    (Standard, "fspick", ENOSYS),
    (Standard, "mount_setattr", ENOSYS),
    (Standard, "perf_event_open", ENOSYS),
    (Standard, "ptrace", ENOSYS),
    (Namespaces, "unshare", EPERM),
    (Namespaces, "setns", EPERM),
    (Namespaces, "mount", EPERM),
    (Namespaces, "umount2", EPERM),
    (Namespaces, "pivot_root", EPERM),
    (Namespaces, "chroot", EPERM),
];
static RULES_COND: [(FilterType, &'static str, Action, Compare); 4] = [
    (TtyControl, "ioctl", EPERM, Compare::new(1, Op::MaskedEqual(libc::TIOCLINUX), libc::TIOCLINUX)),
    (TtyControl, "ioctl", EPERM, Compare::new(1, Op::MaskedEqual(libc::TIOCSTI), libc::TIOCSTI)),
    (Namespaces, "clone", EPERM, Compare::new(0, Op::MaskedEqual(libc::CLONE_NEWUSER as u64), libc::CLONE_NEWUSER as u64)),
    (Standard, "personality", EPERM, Compare::new(0, Op::NotEqual, PERSONALITY)),
];

// Provide configuration parameters for berkley filtering program generation
pub fn configure_bpf_program(instance: &ContainerRuntime) -> Vec<FilterType> {
    let mut filters = vec![Standard];

    if !instance.enable_userns() {
        filters.push(Namespaces)
    }

    if !instance.retain_session() {
        filters.push(TtyControl)
    }

    filters
}

// Generate berkley packet filtering program to pass into the namespaces container
pub fn provide_bpf_program(
    types: Vec<FilterType>,
    reader: &PipeReader,
    mut writer: PipeWriter,
) -> Result<i32, Box<dyn std::error::Error>> {
    let mut filter = ScmpFilterContext::new_filter(Action::Allow)?;
    let rules = RULES
        .iter()
        .filter(|a| types.contains(&a.0))
        .map(|a| (a.1, a.2))
        .collect::<Vec<(&str, Action)>>();
    let rules_cond = RULES_COND
        .iter()
        .filter(|a| types.contains(&a.0))
        .map(|a| (a.1, a.2, a.3))
        .collect::<Vec<(&str, Action, Compare)>>();

    if cfg!(target_arch = "x86_64") {
        filter.add_arch(ScmpArch::X86)?;
        filter.add_arch(ScmpArch::X8664)?;
    } else {
        filter.add_arch(ScmpArch::Native)?;
    }

    for rule in rules {
        filter.add_rule(rule.1, Syscall::from_name(rule.0)?)?;
    }

    for rule in rules_cond {
        filter.add_rule_conditional(rule.1, Syscall::from_name(rule.0)?, &[rule.2])?;
    }

    filter.export_bpf(&mut writer).unwrap();
    Ok(reader.as_raw_fd())
}
