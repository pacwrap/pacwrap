#![allow(dead_code)]

use console::{style, Style, StyledObject};
use dialoguer::{theme::ColorfulTheme, Input};

pub fn prompt(prefix: &str, prompt: StyledObject<&str>, yn_prompt: bool)  -> Result<(),()> {
    if let Ok(value) = create_prompt(prompt, prefix, 
        if yn_prompt { "[Y/n]" } else { "[N/y]" }) {  
        if value.to_lowercase() == "y" || (yn_prompt && value.is_empty()) {
            Ok(())
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

fn create_prompt(message: StyledObject<&str>, prefix: &str, prompt: &str) -> Result<String, std::io::Error> {
    let theme = ColorfulTheme {
        success_prefix: style(prefix.into()).green().bold(),
        prompt_prefix: style(prefix.into()).green().bold(),
        error_prefix: style(prefix.into()).red().bold(),
        prompt_suffix: style(prompt.to_string()).bold(),
        success_suffix: style(prompt.to_string()).bold(), 
        prompt_style: Style::new(),
        values_style: Style::new(),
        ..ColorfulTheme::default()
    };

    return Input::with_theme(&theme)
            .with_prompt(message.to_string())
            .allow_empty(true)
            .interact_text();
}
