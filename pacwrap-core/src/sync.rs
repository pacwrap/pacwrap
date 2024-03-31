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
    fmt::{Display, Formatter, Result as FmtResult},
    fs::{create_dir, create_dir_all},
    path::Path,
    process::exit,
};

use alpm::{Alpm, SigLevel, Usage};
use lazy_static::lazy_static;
use pacmanconf::{self, Config};
use serde::{Deserialize, Serialize};

use crate::{
    config::{cache::ContainerCache, global::ProgressKind, ContainerHandle, ContainerType, ContainerVariables, Global, CONFIG},
    constants::{ARROW_RED, BAR_GREEN, BOLD, CACHE_DIR, CONFIG_DIR, DATA_DIR, RESET},
    err,
    error,
    exec::pacwrap_key,
    impl_error,
    lock::Lock,
    sync::{
        event::download::{self, DownloadEvent},
        filesystem::create_hard_link,
    },
    Error,
    ErrorGeneric,
    ErrorTrait,
    Result,
};

use self::filesystem::create_blank_state;

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
    SignalInterrupt,
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
    fn fmt(&self, fmter: &mut Formatter<'_>) -> FmtResult {
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
            Self::SignalInterrupt => write!(fmter, "Signal interrupt was triggered."),
            Self::UnableToLocateKeyrings => write!(fmter, "Unable to locate pacman keyrings."),
            Self::RepoConfError(path, err) => write!(fmter, "'{}': {}", path, err),
            Self::NothingToDo(_) => write!(fmter, "Nothing to do."),
            _ => Ok(()),
        }?;

        if let Self::SignalInterrupt = self {
            write!(fmter, "\n{} Transaction aborted.", *ARROW_RED)
        } else {
            write!(fmter, "\n{} Transaction failed.", *ARROW_RED)
        }
    }
}

impl_error!(SyncError);

impl From<&Error> for SyncError {
    fn from(error: &Error) -> SyncError {
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
    let hook_dirs = vec!["/mnt/fs/usr/share/libalpm/hooks/", "/mnt/fs/etc/pacman.d/hooks/"];

    handle.set_logfile("/mnt/share/pacwrap.log").unwrap();
    handle.set_hookdirs(hook_dirs.iter()).unwrap();
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
    let mut handle = Alpm::new(insvars.root(), &db_path).unwrap();

    handle.set_cachedirs(vec![format!("{}/pkg", *CACHE_DIR)].iter()).unwrap();
    handle.set_gpgdir(format!("{}/pacman/gnupg", *DATA_DIR)).unwrap();
    handle.set_logfile(format!("{}/pacwrap.log", *DATA_DIR)).unwrap();
    handle.set_parallel_downloads(CONFIG.alpm().parallel_downloads());
    handle.set_check_space(CONFIG.alpm().check_space());
    handle.set_disable_dl_timeout(CONFIG.alpm().download_timeout());
    handle = register_remote(handle, remotes);
    handle
}

pub fn instantiate_container<'a>(handle: &'a ContainerHandle<'a>) -> Result<()> {
    let instype = handle.metadata().container_type();
    let root = handle.vars().root();
    let home = handle.vars().home();

    create_dir(root).prepend_io(|| root.into())?;

    if let ContainerType::Aggregate | ContainerType::Base = instype {
        if !Path::new(home).exists() {
            create_dir(home).prepend_io(|| home.into())?;
        }
    }

    if let ContainerType::Base | ContainerType::Slice = instype {
        create_blank_state(handle.vars().instance())?;
    }

    if let ContainerType::Base = instype {
        schema::extract(handle, &None)?;
    }

    handle.save()
}

pub fn instantiate_trust() -> Result<()> {
    let path = &format!("{}/pacman/gnupg/", *DATA_DIR);

    if Path::new(path).exists() {
        return Ok(());
    }

    println!("{} {}Initializing package trust database...{}", *BAR_GREEN, *BOLD, *RESET);

    if !Path::new("/usr/share/pacman/keyrings").exists() {
        err!(SyncError::UnableToLocateKeyrings)?
    }

    create_dir_all(path).prepend_io(|| path.into())?;
    pacwrap_key(vec!["--init"])?;
    pacwrap_key(vec!["--populate"])
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

fn synchronize_database(cache: &ContainerCache, force: bool, lock: &Lock) -> Result<()> {
    match cache.obtain_base_handle() {
        Some(ins) => {
            let db_path = format!("{}/pacman/", *DATA_DIR);
            let mut handle = alpm_handle(&ins.vars(), db_path, &*DEFAULT_ALPM_CONF);

            lock.assert()?;
            println!("{} {}Synchronizing package databases...{}", *BAR_GREEN, *BOLD, *RESET);
            handle.set_dl_cb(DownloadEvent::new().style(&ProgressKind::Verbose), download::event);

            if let Err(err) = handle.syncdbs_mut().update(force) {
                err!(SyncError::InitializationFailure(err.to_string()))?
            }

            Alpm::release(handle).unwrap();
            lock.assert()?;

            for i in cache.registered().iter() {
                let ins: &ContainerHandle = cache.get_instance(i)?;

                for repo in PACMAN_CONF.repos.iter() {
                    let src = &format!("{}/pacman/sync/{}.db", *DATA_DIR, repo.name);
                    let dest = &format!("{}/var/lib/pacman/sync/{}.db", ins.vars().root(), repo.name);
        
                    if let Err(error) = create_hard_link(src, dest).prepend(|| format!("Failed to hardlink db '{}'", dest)) {
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
    if sigs.is_empty() {
        return default;
    }

    let mut sig = SigLevel::empty();

    for level in sigs {
        sig = sig
            | match level.as_ref() {
                "TrustAll" => SigLevel::DATABASE_UNKNOWN_OK | SigLevel::PACKAGE_UNKNOWN_OK,
                "DatabaseTrustAll" => SigLevel::DATABASE_UNKNOWN_OK | SigLevel::PACKAGE_MARGINAL_OK,
                "PackageTrustAll" => SigLevel::PACKAGE_UNKNOWN_OK | SigLevel::DATABASE_MARGINAL_OK,
                "DatabaseRequired" | "DatabaseTrustedOnly" => SigLevel::DATABASE,
                "PackageRequired" | "Required" => SigLevel::PACKAGE,
                "PackageOptional" => SigLevel::PACKAGE_OPTIONAL,
                "DatabaseOptional" => SigLevel::DATABASE_OPTIONAL,
                _ => SigLevel::empty(),
            }
    }

    sig
}

fn default_signature() -> SigLevel {
    signature(&CONFIG.alpm().sig_level(), SigLevel::PACKAGE | SigLevel::DATABASE_OPTIONAL)
}

fn load_repositories() -> Config {
    let path = format!("{}/repositories.conf", *CONFIG_DIR);

    match Config::from_file(&path) {
        Ok(config) => config,
        Err(error) => {
            //The following code is ugly, precisely because, the pacman_conf library does not
            //provide ergonomic error strings. At some point perhaps, we should fork pacman_conf?

            let error = error.to_string();
            let error = error.split("error: ").collect::<Vec<_>>()[1].split("\n").collect::<Vec<&str>>()[0];
            let error = error!(SyncError::RepoConfError(path, error.to_string()));

            exit(error.error());
        }
    }
}
