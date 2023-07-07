use std::collections::HashMap;

use alpm::Progress;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use console::{Term, style};

#[derive(Clone)]
pub struct ProgressCallback {
    progress: MultiProgress,
    prbar: HashMap<String, ProgressBar>,
    style: ProgressStyle,
    finished: ProgressStyle,
    total: bool
}

impl ProgressCallback {
    pub fn new(t: bool) -> Self {
        let size = Term::size(&Term::stdout());
        let width = (size.1 / 2).to_string();
        let width_str = " {spinner:.green} {msg:<".to_owned()+width.as_str();

        Self {
            total: t,
            progress: MultiProgress::new(),
            style:  ProgressStyle::with_template(&(width_str+"} [{wide_bar}] {percent:<3}%"))
            .unwrap().progress_chars("#-").tick_strings(&[" ", "✓"]),
            finished:  ProgressStyle::with_template(" {spinner:.green} {msg}")
                .unwrap().progress_chars("#-").tick_strings(&[" ","✓"]),
            prbar: HashMap::new(),
        }
    }
}

//TODO: Implement total progress

pub fn progress_event(progress: Progress, pkgname: &str, percent: i32, howmany: usize, current: usize, this: &mut ProgressCallback) {
    let progress_ident: String = progress_ident(progress,pkgname);

    match this.prbar.get_mut(&progress_ident) {
        Some(pb) => {
            if percent < 100 {
                pb.set_position(progress_u64(percent));
            } else {
                pb.finish();
            }
        },
        None => {
            let len = howmany.to_string().len()-current.to_string().len();  
            let mut whitespace = String::new();
            if len > 0 {
                for _ in 0..len {
                    whitespace.push_str(" ");
                } 
            }

            let pos = current + 1;
            let progress_name: String = progress_name(progress,pkgname);
            let pb = this.progress.add(ProgressBar::new(progress_u64(percent)));
            pb.set_style(this.style.clone());
            pb.set_message(format!("({}{}/{}) {}", whitespace, pos, howmany, style(progress_name).bold())); 
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
        _ => pkgname.into()
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

fn progress_u64(u: i32) -> u64 {
    match u.try_into() { Ok(i) => i, Err(_) => 0 }
}
