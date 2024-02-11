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
use std::{
    fmt::{Display, Formatter},
    path::Path,
    process::exit,
};

use alpm::{Alpm, SigLevel, Usage};
use lazy_static::lazy_static;
use pacmanconf;
use serde::{Deserialize, Serialize};

use crate::{
    config::{cache::ContainerCache, global::ProgressKind, ContainerHandle, ContainerVariables, Global, CONFIG},
    constants::{ARROW_RED, BAR_GREEN, BOLD, CACHE_DIR, CONFIG_DIR, DATA_DIR, RESET},
    err,
    error,
    error::*,
    exec::pacman_key,
    sync::event::download::{self, DownloadEvent},
    ErrorKind,
};

pub mod event;
pub mod filesystem;
mod resolver;
mod resolver_local;
pub mod schema;
pub mod transaction;
pub mod utils;

lazy_static! {
    pub static ref DEFAULT_ALPM_CONF: AlpmConfigData = AlpmConfigData::new();
    static ref PACMAN_CONF: pacmanconf::Config = load_repositories();
    static ref DEFAULT_SIGLEVEL: SigLevel = default_signature();
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SyncError {
    TransactionFailureAgent,
    ParameterAcquisitionFailure,
    DeserializationFailure,
    InvalidMagicNumber,
    AgentVersionMismatch,
    NothingToDo(bool),
    DependentContainerMissing(String),
    RecursionDepthExceeded(isize),
    TargetUpstream(String),
    TargetNotInstalled(String),
    TargetNotAvailable(String),
    PreparationFailure(String),
    TransactionFailure(String),
    InitializationFailure(String),
    InternalError(String),
    NoCompatibleRemotes,
    UnableToLocateKeyrings,
    RepoConfError(String, String),
}

impl Display for SyncError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::DependentContainerMissing(u) =>
                write!(fmter, "Dependent container '{}{u}{}' is misconfigured or otherwise is missing.", *BOLD, *RESET),
            Self::TargetNotAvailable(pkg) =>
                write!(fmter, "Target package {}{pkg}{}: Not available in sync databases.", *BOLD, *RESET),
            Self::TargetUpstream(pkg) =>
                write!(fmter, "Target package {}{pkg}{}: Installed in upstream container.", *BOLD, *RESET),
            Self::RecursionDepthExceeded(u) => write!(fmter, "Recursion depth exceeded maximum of {}{u}{}.", *BOLD, *RESET),
            Self::NoCompatibleRemotes => write!(fmter, "No compatible containers available to synchronize remote database."),
            Self::InvalidMagicNumber => write!(fmter, "Deserialization of input parameters failed: Invalid magic number."),
            Self::TargetNotInstalled(pkg) => write!(fmter, "Target package {}{pkg}{}: Not installed.", *BOLD, *RESET),
            Self::InitializationFailure(msg) => write!(fmter, "Failure to initialize transaction: {msg}"),
            Self::PreparationFailure(msg) => write!(fmter, "Failure to prepare transaction: {msg}"),
            Self::TransactionFailure(msg) => write!(fmter, "Failure to commit transaction: {msg}"),
            Self::DeserializationFailure => write!(fmter, "Deserialization of input parameters failed."),
            Self::ParameterAcquisitionFailure => write!(fmter, "Failure to acquire agent runtime parameters."),
            Self::AgentVersionMismatch => write!(fmter, "Agent binary mismatch."),
            Self::InternalError(msg) => write!(fmter, "Internal failure: {msg}"),
            Self::UnableToLocateKeyrings => write!(fmter, "Unable to locate pacman keyrings."),
            Self::RepoConfError(path, err) => write!(fmter, "'{}': {}", path, err),
            Self::NothingToDo(_) => write!(fmter, "Nothing to do."),
            _ => Ok(()),
        }?;

        write!(fmter, "\n{} Transaction failed.", *ARROW_RED)
    }
}

impl ErrorTrait for SyncError {
    fn code(&self) -> i32 {
        1
    }
}

impl From<Error> for SyncError {
    fn from(error: Error) -> SyncError {
        Self::InternalError(error.kind().to_string())
    }
}

impl From<SyncError> for String {
    fn from(error: SyncError) -> Self {
        error.into()
    }
}

#[derive(Serialize, Deserialize)]
pub struct AlpmConfigData {
    repos: Vec<(String, u32, Vec<String>)>,
}

impl AlpmConfigData {
    fn new() -> Self {
        let mut remotes = Vec::new();

        for repo in PACMAN_CONF.repos.iter() {
            remotes.push((repo.name.clone(), signature(&repo.sig_level, *DEFAULT_SIGLEVEL).bits(), repo.servers.clone()));
        }

        Self { repos: remotes }
    }
}

pub fn instantiate_alpm_agent(config: &Global, remotes: &AlpmConfigData) -> Alpm {
    let mut handle = Alpm::new("/mnt/fs", "/mnt/fs/var/lib/pacman/").unwrap();

    handle.set_logfile("/mnt/share/pacwrap.log").unwrap();
    handle
        .set_hookdirs(vec!["/mnt/fs/usr/share/libalpm/hooks/", "/mnt/fs/etc/pacman.d/hooks/"].iter())
        .unwrap();
    handle.set_cachedirs(vec!["/mnt/share/cache"].iter()).unwrap();
    handle.set_gpgdir("/mnt/share/gnupg").unwrap();
    handle.set_logfile("/mnt/share/pacwrap.log").unwrap();
    handle.set_check_space(false);
    handle.set_disable_dl_timeout(config.alpm().download_timeout());
    handle.set_parallel_downloads(config.alpm().parallel_downloads());
    handle = register_remote(handle, remotes);
    handle
}

pub fn instantiate_alpm(inshandle: &ContainerHandle) -> Alpm {
    alpm_handle(inshandle.vars(), format!("{}/var/lib/pacman/", inshandle.vars().root()), &*DEFAULT_ALPM_CONF)
}

fn alpm_handle(insvars: &ContainerVariables, db_path: String, remotes: &AlpmConfigData) -> Alpm {
    let root = insvars.root();
    let mut handle = Alpm::new(root, &db_path).unwrap();

    handle.set_cachedirs(vec![format!("{}/pkg", *CACHE_DIR)].iter()).unwrap();
    handle.set_gpgdir(format!("{}/pacman/gnupg", *DATA_DIR)).unwrap();
    handle.set_logfile(format!("{}/pacwrap.log", *DATA_DIR)).unwrap();
    handle.set_parallel_downloads(CONFIG.alpm().parallel_downloads());
    handle.set_check_space(CONFIG.alpm().check_space());
    handle.set_disable_dl_timeout(CONFIG.alpm().download_timeout());
    handle = register_remote(handle, remotes);
    handle
}

//TODO: Port pacman-key to Rust

pub fn instantiate_trust() -> Result<()> {
    let path = &format!("{}/pacman/gnupg/", *DATA_DIR);

    if Path::new(path).exists() {
        return Ok(());
    }

    println!("{} {}Initializing package trust database...{}", *BAR_GREEN, *BOLD, *RESET);

    if !Path::new("/usr/share/pacman/keyrings").exists() {
        err!(SyncError::UnableToLocateKeyrings)?
    }

    if let Err(error) = std::fs::create_dir_all(path) {
        err!(ErrorKind::IOError(path.into(), error.kind()))?
    }

    pacman_key(path, vec!["--init"])?;
    pacman_key(path, vec!["--populate"])
}

fn register_remote(mut handle: Alpm, config: &AlpmConfigData) -> Alpm {
    for repo in &config.repos {
        let core = handle.register_syncdb_mut(repo.0.clone(), SigLevel::from_bits(repo.1).unwrap()).unwrap();

        for server in &repo.2 {
            core.add_server(server.as_str()).unwrap();
        }

        core.set_usage(Usage::ALL).unwrap();
    }

    handle
}

fn synchronize_database(cache: &ContainerCache, force: bool) -> Result<()> {
    match cache.obtain_base_handle() {
        Some(ins) => {
            let db_path = format!("{}/pacman/", *DATA_DIR);
            let mut handle = alpm_handle(&ins.vars(), db_path, &*DEFAULT_ALPM_CONF);

            println!("{} {}Synchronizing package databases...{}", *BAR_GREEN, *BOLD, *RESET);
            handle.set_dl_cb(DownloadEvent::new().style(&ProgressKind::Verbose), download::event);

            if let Err(err) = handle.syncdbs_mut().update(force) {
                err!(SyncError::InitializationFailure(err.to_string()))?
            }

            Alpm::release(handle).unwrap();

            for i in cache.registered().iter() {
                let ins: &ContainerHandle = cache.get_instance(i).unwrap();

                for repo in PACMAN_CONF.repos.iter() {
                    let src = &format!("{}/pacman/sync/{}.db", *DATA_DIR, repo.name);
                    let dest = &format!("{}/var/lib/pacman/sync/{}.db", ins.vars().root(), repo.name);
                    if let Err(error) = filesystem::create_hard_link(src, dest) {
                        error.warn();
                    }
                }
            }

            Ok(())
        }
        None => err!(SyncError::NoCompatibleRemotes),
    }
}

fn signature(sigs: &Vec<String>, default: SigLevel) -> SigLevel {
    if sigs.len() > 0 {
        let mut sig = SigLevel::empty();

        for level in sigs {
            sig = sig
                | if level == "Required" || level == "PackageRequired" {
                    SigLevel::PACKAGE
                } else if level == "DatabaseRequired" || level == "DatabaseTrustedOnly" {
                    SigLevel::DATABASE
                } else if level == "PackageOptional" {
                    SigLevel::PACKAGE_OPTIONAL
                } else if level == "PackageTrustAll" {
                    SigLevel::PACKAGE_UNKNOWN_OK | SigLevel::DATABASE_MARGINAL_OK
                } else if level == "DatabaseOptional" {
                    SigLevel::DATABASE_OPTIONAL
                } else if level == "DatabaseTrustAll" {
                    SigLevel::DATABASE_UNKNOWN_OK | SigLevel::PACKAGE_MARGINAL_OK
                } else {
                    SigLevel::empty()
                }
        }

        sig
    } else {
        default
    }
}

fn load_repositories() -> pacmanconf::Config {
    let path = format!("{}/repositories.conf", *CONFIG_DIR);

    match pacmanconf::Config::from_file(&path) {
        Ok(config) => config,
        Err(error) => {
            //The following code is ugly, precisely because, the pacman_conf library does not
            //provide ergonomic error strings. At some point perhaps, pacman_conf should be
            //eliminated as an upstream dependency or otherwise forked.

            let error = error.to_string();
            let error = error.split("error: ").collect::<Vec<_>>()[1].split("\n").collect::<Vec<&str>>()[0];
            let error = error!(SyncError::RepoConfError(path, error.to_string()));

            exit(error.error());
        }
    }
}

fn default_signature() -> SigLevel {
    signature(&CONFIG.alpm().sig_level(), SigLevel::PACKAGE | SigLevel::DATABASE_OPTIONAL)
}
