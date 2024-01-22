use dialoguer::{
    console::{style, Style},
    theme::ColorfulTheme,
    Input,
};

pub fn prompt(prefix: &str, prompt: impl Into<String>, yn_prompt: bool) -> Result<(), ()> {
    if let Ok(value) = create_prompt(prompt.into(), prefix, yn_prompt) {
        if value.to_lowercase() == "y" || (yn_prompt && value.is_empty()) {
            Ok(())
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

fn create_prompt(message: String, prefix: &str, yn_prompt: bool) -> Result<String, std::io::Error> {
    let prompt = match yn_prompt {
        true => ("[Y/n]", style(prefix.into()).blue().bold()),
        false => ("[y/N]", style(prefix.into()).red().bold()),
    };

    let theme = ColorfulTheme {
        success_prefix: style(prefix.into()).green().bold(),
        prompt_prefix: prompt.1,
        error_prefix: style(prefix.into()).red().bold(),
        prompt_suffix: style(prompt.0.to_string()).bold(),
        success_suffix: style(prompt.0.to_string()).bold(),
        prompt_style: Style::new(),
        values_style: Style::new(),
        ..ColorfulTheme::default()
    };

    return Input::with_theme(&theme).with_prompt(message).allow_empty(true).interact_text();
}
