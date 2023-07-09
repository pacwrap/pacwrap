use std::collections::HashMap;

use alpm::{AnyDownloadEvent, DownloadEvent};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use console::Term;

#[derive(Clone)]
pub struct DownloadCallback {
    progress: MultiProgress,
    prbar: HashMap<String, ProgressBar>,
    style: ProgressStyle,
    finished: ProgressStyle,
    total: bool
}

impl DownloadCallback {
    pub fn new(t: bool) -> Self {
        let size = Term::size(&Term::stdout());
        let width = ((size.1 / 2) - 14).to_string();
        let width_str = " {spinner:.green} {msg:<".to_owned()+width.as_str();

        Self {
            total: t,
            progress: MultiProgress::new(),
            style:  ProgressStyle::with_template(&(width_str+"} {bytes:>11} {bytes_per_sec:>12} {elapsed_precise:>5} [{wide_bar}] {percent:<3}%"))
            .unwrap().progress_chars("#-").tick_strings(&[" ", "✓"]),
            finished:  ProgressStyle::with_template(" {spinner:.green} {msg}")
                .unwrap().progress_chars("#-").tick_strings(&[" ","✓"]),
            prbar: HashMap::new(),
        }
    }
}

//TODO: Implement total progress bar.

pub fn download_event(filename: &str, download: AnyDownloadEvent, this: &mut DownloadCallback) {
    match download.event() {
        DownloadEvent::Progress(progress) => { 
            match this.prbar.get_mut(&filename.to_string()) {
                Some(pb) => {
                    pb.set_position(progress.downloaded.unsigned_abs());
                },
                None => {
                    let name: Vec<&str> = filename.split(".").collect(); 
                    let pb = this.progress.add(ProgressBar::new(progress.total.unsigned_abs()));
                    pb.set_style(this.style.clone());
                    pb.set_message(name[0].to_string()); 
                    this.prbar.insert(filename.to_string(), pb);   
                }
            }    
        },
        DownloadEvent::Completed(_) => {
            match this.prbar.get_mut(&filename.to_string()) {
                Some(pb) => {
                    pb.finish(); 
                },
                None => {
                    let name: Vec<&str> = filename.split(".").collect(); 
                    let pb = this.progress.add(ProgressBar::new(0));
                    pb.set_style(this.finished.clone());
                    pb.set_message(format!("{} is up-to-date!", name[0].to_string())); 
                    pb.finish();
                    this.prbar.insert(filename.to_string(), pb); 
                }
            }
        },
        _ => (),
    }
}
