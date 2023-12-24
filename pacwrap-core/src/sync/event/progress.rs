use alpm::Progress;
use dialoguer::console::Term;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{constants::{BOLD, RESET}, sync::transaction::{TransactionType, TransactionMode}};
use super::whitespace;

#[derive(Clone)]
pub struct ProgressEvent {
    progress: ProgressBar,
    offset: usize,
    style: ProgressStyle,
    current: String,
}

impl ProgressEvent {
    pub fn new(state: &TransactionType) -> Self {
        let size = Term::size(&Term::stdout());
        let width = (size.1 / 2).to_string();
        
        Self {
            offset: state.pr_offset(),
            style: ProgressStyle::with_template(&(" {spinner:.green} {msg:<".to_owned()+width.as_str()+"} [{wide_bar}] {percent:<3}%"))
            .unwrap().progress_chars("#-").tick_strings(&[" ", "✓"]),
            progress: ProgressBar::new(0),
            current: "".into(),
        }
    }
}

pub fn event(progress: Progress, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
    let ident = ident(progress,pkgname);  

    if ident != this.current {
        let pos = current + this.offset;
        let total = howmany + this.offset; 
        let progress_name: String = name(progress,pkgname);
        let whitespace = whitespace(total.to_string().len(), pos.to_string().len());
 
        this.progress = ProgressBar::new(100);
        this.progress.set_message(format!("({}{whitespace}{pos}{}/{}{total}{}) {progress_name}", *BOLD, *RESET, *BOLD, *RESET)); 
        this.progress.set_style(this.style.clone());
        this.current = ident;
    }
    
    this.progress.set_position(percent as u64);

    if percent == 100 {
        this.progress.finish();            
    }
}

pub fn condensed(progress: Progress, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressEvent) {
     if let Progress::AddStart | Progress::RemoveStart | Progress::UpgradeStart = progress { 
        let pos = current + this.offset;
        let total = howmany + this.offset; 
        let progress_name: String = name(progress,pkgname);
        let whitespace = whitespace(total.to_string().len(), pos.to_string().len());

        if this.current != "" {
            this.progress = ProgressBar::new(howmany as u64);
            this.progress.set_message(format!("({}{whitespace}{pos}{}/{}{total}{}) {progress_name}", *BOLD, *RESET, *BOLD, *RESET)); 
            this.progress.set_style(this.style.clone());
            this.progress.set_position(current as u64);
            this.current = "".into();
        } else {
            this.progress.set_position(current as u64);
            this.progress.set_message(format!("({}{whitespace}{pos}{}/{}{total}{}) {progress_name}", *BOLD, *RESET, *BOLD, *RESET));  
        }

        if current == howmany {
            this.progress.set_message(format!("({}{whitespace}{pos}{}/{}{total}{}) Foreign synchronization complete", *BOLD, *RESET, *BOLD, *RESET));   
            this.progress.finish();
        }
    } else {
        event(progress, pkgname, percent, howmany, current, this)
    }
}

fn name(progress: Progress, pkgname: &str) -> String {
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

fn ident(progress: Progress, pkgname: &str) -> String {
    match progress {
        Progress::KeyringStart => "keyring",
        Progress::IntegrityStart => "integrity",
        Progress::LoadStart => "loadstart",
        Progress::ConflictsStart => "conflicts",
        _ => pkgname

    }.into()
}

pub fn callback(state: &TransactionMode) -> for<'a, 'b> fn(Progress, &'a str, i32, usize, usize, &'b mut ProgressEvent) {
    match state { 
        TransactionMode::Local => event,
        TransactionMode::Foreign => condensed,
    }
}
