use nix::unistd::isatty;

use crate::constants::{COLORTERM, IS_COLOR_TERMINAL, TERM};

pub fn is_truecolor_terminal() -> bool {
    let value = COLORTERM.to_lowercase();

    is_color_terminal() && value == "truecolor" || value == "24bit"
}

pub fn is_color_terminal() -> bool {
    !TERM.is_empty() && TERM.to_lowercase() != "dumb" && isatty(0).is_ok() && isatty(1).is_ok()
}

pub fn arrow_red() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;31m->[0m",
        false => "->",
    }
}

pub fn arrow_cyan() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;36m->[0m",
        false => "->",
    }
}

pub fn arrow_green() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;32m->[0m",
        false => "->",
    }
}

pub fn bar_red() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;31m::[0m",
        false => "::",
    }
}

pub fn bar_green() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;32m::[0m",
        false => "::",
    }
}

pub fn bar_cyan() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;36m::[0m",
        false => "::",
    }
}

pub fn bold_white() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;37m",
        false => "",
    }
}

pub fn bold_red() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;31m",
        false => "",
    }
}

pub fn bold_yellow() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;33m",
        false => "",
    }
}

pub fn bold_green() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1;32m",
        false => "",
    }
}

pub fn yellow() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[33m",
        false => "",
    }
}

pub fn bold() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[1m",
        false => "",
    }
}

pub fn checkmark() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => " [1;32m✓[0m",
        false => " ✓",
    }
}

pub fn reset() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[0m",
        false => "",
    }
}

pub fn underline() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[4m",
        false => "",
    }
}

pub fn dim() -> &'static str {
    match *IS_COLOR_TERMINAL {
        true => "[2m",
        false => "",
    }
}
