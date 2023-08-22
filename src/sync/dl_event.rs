use std::collections::HashMap;

use alpm::{AnyDownloadEvent, DownloadEvent};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use console::Term;
use lazy_static::lazy_static;

use super::utils::whitespace;

lazy_static!{
   static ref INIT: ProgressStyle = ProgressStyle::with_template(" {spinner:.green} {msg}")
        .unwrap();
   static ref UP_TO_DATE: ProgressStyle = ProgressStyle::with_template(" {spinner:.green} {msg} is up-to-date!")        
        .unwrap()
        .progress_chars("#-")
        .tick_strings(&[" ","✓"]); 
}

#[derive(Clone)]
pub struct DownloadCallback {
    progress: MultiProgress,
    prbar: HashMap<String, ProgressBar>,
    style: ProgressStyle,
    total_files: usize,
    total_files_len: usize,
    files_done: usize
}

impl DownloadCallback {
    pub fn new(total_bytes: u64, files: usize) -> Self {
        let mut bars = HashMap::new();
        let files_str_len = files.to_string().len();
        let size = Term::size(&Term::stdout());
        let width = ((size.1 / 2) - 14).to_string();
        let width_str = " {spinner:.green} {msg:<".to_owned()+width.as_str();
        let multiprogress = MultiProgress::new();
        let pb_style_tmpl = "} {bytes:>11} {bytes_per_sec:>12} {elapsed_precise:>5} [{wide_bar}] {percent:<3}%";
        let pb_style = ProgressStyle::with_template(&(width_str+pb_style_tmpl))
            .unwrap()
            .progress_chars("#-")
            .tick_strings(&[" ", "✓"]);
        
        if total_bytes > 0 {
            let pb_total = multiprogress.add(
                ProgressBar::new(total_bytes)
                    .with_style(pb_style.clone())
                    .with_message(format!("Total ({}0/{})", whitespace(files_str_len, 1), files)));

            bars.insert("total".into(), pb_total);
        }

        Self {
            style: pb_style,
            total_files: files,
            total_files_len: files_str_len,
            files_done: 0,
            progress: multiprogress,
            prbar: bars,
        }
    }
}

pub fn download_event(file: &str, download: AnyDownloadEvent, this: &mut DownloadCallback) {
    if file.ends_with(".sig") { 
        return; 
    }

    match download.event() {
        DownloadEvent::Progress(progress) => { 
            if let Some(pb) = this.prbar.get_mut(&file.to_string()) {
               if pb.length().unwrap() == 0 {
                    pb.set_length(progress.total.unsigned_abs());
                    pb.set_style(this.style.clone());
                }

                pb.set_position(progress.downloaded.unsigned_abs());
                    
                if let Some(total) = this.prbar.get("total") { 
                    total.inc(progress.downloaded.unsigned_abs());
                }
            }    
        },
        DownloadEvent::Completed(_) => {
            if let Some(pb) = this.prbar.get_mut(&file.to_string()) { 
                if pb.length().unwrap() == 0 {  
                    pb.set_style(UP_TO_DATE.clone());
                }
                   
                pb.finish();

                if let Some(total) = this.prbar.get("total") { 
                    this.files_done += 1;
         
                    total.set_message(format!("Total ({}{}/{})", 
                        whitespace(this.total_files_len, this.files_done.to_string().len()), 
                        this.files_done, 
                        this.total_files)); 
                    
                    if this.files_done == this.total_files { 
                        total.finish(); 
                    } 
                }
            }
        },
        DownloadEvent::Init(_) => {
            let pb = if let Some(total) = this.prbar.get("total") { 
                this.progress.insert_before(&total, ProgressBar::new(0))
            } else {
                this.progress.add(ProgressBar::new(0)) 
            };

            pb.set_style(INIT.clone());
            pb.set_message(message(file)); 
            this.prbar.insert(file.into(), pb);
        },
        _ => (),
    }
}

fn message(filename: &str) -> String {
    let name: Vec<&str> = filename.split(".pkg.tar.").collect(); 
    let mut msg_name: String = name[0].to_string();

    if msg_name.ends_with(".db") {
        msg_name.truncate(msg_name.len()-3);
    }
    
    msg_name
}
