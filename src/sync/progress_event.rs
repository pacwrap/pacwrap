use std::collections::HashMap;

use alpm::Progress;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use console::{Term, style};

use crate::utils::whitespace;
use super::utils::i32_into_u64;

#[derive(Clone)]
pub struct ProgressCallback {
    progress: MultiProgress,
    prbar: HashMap<String, ProgressBar>,
    style: ProgressStyle,
    offset: usize,
}

impl ProgressCallback {
    pub fn new() -> Self {
        let size = Term::size(&Term::stdout());
        let width = (size.1 / 2).to_string();
        let width_str = " {spinner:.green} {msg:<".to_owned()+width.as_str();

        Self {
            offset: 1,
            progress: MultiProgress::new(),
            style:  ProgressStyle::with_template(&(width_str+"} [{wide_bar}] {percent:<3}%"))
            .unwrap().progress_chars("#-").tick_strings(&[" ", "✓"]),
            prbar: HashMap::new(),
        }
    }
}

pub fn progress_event(progress: Progress, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressCallback) {
    let progress_ident: String = progress_ident(progress,pkgname);
    match this.prbar.get_mut(&progress_ident) {
        Some(pb) => {
            pb.set_position(i32_into_u64(percent));
            if percent == 100 {
                pb.finish();
            }
        },
        None => {
            if current == 1 {
                pos_offset(this,progress);
            }
           
            let pos = current + this.offset;
            let total = howmany + this.offset; 
            let progress_name: String = progress_name(progress,pkgname);
            let pb = this.progress.add(ProgressBar::new(100));
            let whitespace = whitespace(total.to_string().len(), pos.to_string().len()); 
            
            pb.set_style(this.style.clone());
            pb.set_message(format!("({}{}/{}) {}", whitespace, style(pos).bold().white(), style(total).bold().white(), progress_name)); 
            pb.set_position(i32_into_u64(percent)); 

            if percent == 100 {
                pb.finish();
            }

            this.prbar.insert(progress_ident, pb);   
        }
    }
}

fn progress_name(progress: Progress, pkgname: &str) -> String {
    match progress {
        Progress::KeyringStart => "Loading keyring".into(), 
        Progress::IntegrityStart => "Checking integrity".into(),
        Progress::LoadStart => "Loading packages".into(),
        Progress::ConflictsStart => "Checking conflicts".into(),
        Progress::DiskspaceStart => "Checking available diskspace".into(),
        Progress::UpgradeStart => format!("Upgrading {}", pkgname), 
        Progress::AddStart => format!("Installing {}", pkgname),
        Progress::RemoveStart => format!("Removing {}", pkgname),
        Progress::DowngradeStart => format!("Downgrading {}", pkgname),
        Progress::ReinstallStart => format!("Reinstalling {}", pkgname)
    }
}

fn pos_offset(this: &mut ProgressCallback, progress: Progress) {
    match progress {
        Progress::RemoveStart => this.offset = 0, _ => ()
    }
}

fn progress_ident(progress: Progress, pkgname: &str) -> String {
    match progress {
        Progress::KeyringStart => "keyring".into(),
        Progress::IntegrityStart => "integrity".into(),
        Progress::LoadStart => "loadstart".into(),
        Progress::ConflictsStart => "conflicts".into(),
        _ => pkgname.into()

    }
}
