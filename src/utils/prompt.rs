#![allow(dead_code)]

use console::{style, Style};
use dialoguer::{theme::ColorfulTheme, Input};

pub fn prompt(prompt: &str) -> Result<(),()> {
    if let Ok(value) = create_prompt(prompt, "[Y/n]") {  
        if value.to_lowercase() == "y" || value.is_empty() {
            Ok(())
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

pub fn prompt_no(prompt: &str) -> Result<(),()> {
    if let Ok(value) = create_prompt(prompt, "[y/N]") {  
        if value.to_lowercase() == "y" {
            Ok(())
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

fn create_prompt(message: &str, prompt: &str) -> Result<String, std::io::Error> {
    let theme = ColorfulTheme {
        success_prefix: style("::".to_string()).green().bold(),
        prompt_prefix: style("::".to_string()).green().bold(),
        error_prefix: style("::".to_string()).red().bold(),
        prompt_suffix: style(prompt.to_string()).bold(),
        success_suffix: style(prompt.to_string()).bold(), 
        prompt_style: Style::new().bold(),
        values_style: Style::new(),
        ..ColorfulTheme::default()
    };

    return Input::with_theme(&theme)
            .with_prompt(message)
            .allow_empty(true)
            .interact_text();
}
