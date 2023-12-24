use indexmap::IndexSet;
use lazy_static::lazy_static;
use std::fmt::Write;

use pacwrap_core::{utils::{Arguments, 
    arguments::Operand,
    is_color_terminal, 
    is_truecolor_terminal}, ErrorKind};

lazy_static! {
    static ref HELP_ALL: Vec<HelpTopic> = 
        [HelpTopic::Execute, 
        HelpTopic::Sync,
        HelpTopic::Process,
        HelpTopic::Utils,
        HelpTopic::Help,
        HelpTopic::Version, 
        HelpTopic::Copyright].into();
}

pub fn help(mut args: &mut Arguments) -> Result<(), ErrorKind> {
    let help = ascertain_help(&mut args)?;
    let mut buffer = String::new();

    for topic in help.0 {
        topic.write(&mut buffer, help.1).unwrap(); 
    }

    match help.1 {
        HelpLayout::Console => print!("\x1b[?7l{buffer}\x1b[?7h"), _ => print!("{buffer}"),
    }

    Ok(())
}

fn ascertain_help<'a>(args: &mut Arguments) -> Result<(IndexSet<&'a HelpTopic>, &'a HelpLayout), ErrorKind> {
    let mut layout = match is_color_terminal() {
        true => &HelpLayout::Console, false => &HelpLayout::Dumb,
    };
    let mut topic: Vec<&HelpTopic> = vec!(&HelpTopic::Default);
    let mut more = false;

    while let Some(arg) = args.next() { 
        match arg {
            Operand::Long("format")
                | Operand::Long("help")
                | Operand::Short('f')
                | Operand::Short('h') 
                => continue,
            Operand::Short('m') 
                | Operand::Long("more") 
                => more = true,
            Operand::LongPos("format", "dumb") 
                | Operand::ShortPos('f', "dumb") 
                => layout = &HelpLayout::Dumb, 
            Operand::LongPos("format", "markdown") 
                | Operand::ShortPos('f', "markdown") 
                => layout = &HelpLayout::Markdown,
            Operand::LongPos("format", "man") 
                | Operand::ShortPos('f', "man") 
                => layout = &HelpLayout::Man,
            Operand::LongPos("format", "ansi") 
                | Operand::ShortPos('f', "ansi") 
                => layout = &HelpLayout::Console,
            Operand::ShortPos('h', "sync")
                | Operand::ShortPos('h', "S")
                | Operand::LongPos("help", "sync")
                | Operand::LongPos("help", "S") 
                => topic.push(&HelpTopic::Sync),
            Operand::ShortPos('h', "E")
                | Operand::ShortPos('h', "exec")
                | Operand::LongPos("help", "E")
                | Operand::LongPos("help", "exec") 
                => topic.push(&HelpTopic::Execute),
            Operand::ShortPos('h', "process")
                | Operand::ShortPos('h', "P")
                | Operand::LongPos("help", "process")
                | Operand::LongPos("help", "P")
                => topic.push(&HelpTopic::Process),
            Operand::ShortPos('h', "utils")
                | Operand::ShortPos('h', "U")
                | Operand::LongPos("help", "utils")
                | Operand::LongPos("help", "U") 
                => topic.push(&HelpTopic::Utils),
            Operand::ShortPos('h', "help")
                | Operand::ShortPos('h', "h")
                | Operand::LongPos("help", "help")
                | Operand::LongPos("help", "h") 
                => topic.push(&HelpTopic::Help),
            Operand::ShortPos('h', "synopsis") 
                | Operand::LongPos("help", "synopsis") 
                => topic.push(&HelpTopic::Default),
            Operand::ShortPos('h', "V")
                | Operand::ShortPos('h', "version")
                | Operand::LongPos("help", "V")
                | Operand::LongPos("help", "version")
                => topic.push(&HelpTopic::Version),
            Operand::ShortPos('h', "copyright") 
                | Operand::LongPos("help", "copyright") 
                => topic.push(&HelpTopic::Copyright),
            Operand::ShortPos('h', "all")
                | Operand::LongPos("help", "all")
                | Operand::Short('a')
                | Operand::Long("all") 
                => topic.extend(HELP_ALL.iter()),
            Operand::ShortPos('h', topic) 
                | Operand::LongPos("help", topic) 
                => Err(ErrorKind::Message(format!("Topic '{topic}' is not available.").leak()))?,
           _ => Err(args.invalid_operand())?,
        }
    }

    let len = topic.len();
    let start = if more || len == 1 || len > 7 { 0 } else { 1 }; 

    args.set_index(1);
    Ok((topic.drain(start..).collect(), layout))
}

fn minimal(args: &mut Arguments) -> bool {
    match args.next().unwrap_or_default() { 
        Operand::LongPos("version", "min") | Operand::ShortPos('V', "min") => true, _ => false
    }
}

#[derive(Eq, PartialEq, Hash)]
enum HelpTopic {
    Sync,
    Execute,
    Default,
    Utils,
    Process,
    Help,
    Copyright,
    Version
}

enum HelpLayout { 
    Man,
    Dumb,
    Markdown,
    Console
}

impl HelpLayout {
    fn head(&self) -> &str {
        match self {
            Self::Console => "[1m",
            Self::Markdown => "## ",
            Self::Man => ".SH\n",
            Self::Dumb => "",
        }
    }

    fn sub(&self) -> &str {
        match self {
            Self::Console => "    [37;1m",
            Self::Markdown => "* **",
            Self::Man => ".TP\n\\fB", 
            Self::Dumb => "    ",
        }
    }

    fn sub_text(&self) -> &str {
        match self {
            Self::Console | Self::Dumb => "    ",
            Self::Man => ".PP\n", 
            Self::Markdown => "", 
        }
    }

    fn reset(&self) -> &str {
        match self {
            Self::Console => "[0m",
            Self::Markdown 
                | Self::Man 
                | Self::Dumb => ""
        }
    }

    fn reset_bold(&self) -> &str {
        match self {
            Self::Console => "[0m",
            Self::Man => "\\fP",
            Self::Markdown => "**",
            Self::Dumb => ""
        }
    }

    fn bold(&self) -> &str {
        match self {
            Self::Console => "[37;1m",
            Self::Man => "\\fP",
            Self::Markdown => "**",
            Self::Dumb => "",
        }
    }

    fn tab(&self) -> &str {
        match self {
            Self::Console | Self::Dumb => "    ",
            Self::Markdown | Self::Man => "",
        }
    }
}

impl HelpTopic {
    fn write(&self, buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
        match self {
            Self::Default => default(buf,layout),
            Self::Sync => sync(buf,layout),
            Self::Execute => execute(buf,layout),
            Self::Process => process(buf,layout),
            Self::Utils => utils(buf,layout),
            Self::Help => meta(buf,layout),
            Self::Copyright => copyright(buf,layout),
            Self::Version => version(buf,layout),
        }
    }
}

fn default(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let tab = layout.tab(); 
    let sub = layout.sub();
    let bold = layout.bold();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();
    let name = env!("CARGO_PKG_NAME");

    match layout {
        HelpLayout::Man => writeln!(buf, ".nh\n.TH {name} 1 \"{}-{} ({})\" {name} \"User Manual\"\n",
            env!("CARGO_PKG_VERSION"), 
            env!("PACWRAP_BUILDSTAMP"), 
            env!("PACWRAP_BUILDTIME"))?,
        HelpLayout::Markdown => writeln!(buf, "# Pacwrap User Manual

This document was generated by the {name} binary on {} with version {}-{} of the program.\n", 
            env!("PACWRAP_BUILDTIME"), 
            env!("CARGO_PKG_VERSION"), 
            env!("PACWRAP_BUILDSTAMP"))?,
        _ => ()
    }

    writeln!(buf, "{head}NAME{reset}
{tab}pacwrap - Command-line application which facilitates the creation, management, and execution of unprivileged, 
{tab}sandboxed containers with bubblewrap and libalpm.

{head}SYNOPSIS{reset}
{tab}pacwrap [{bold}OPERATIONS{reset_bold}] [{bold}ARGUMENTS{reset_bold}] [{bold}TARGET(S){reset_bold}]	

{head}OPERATIONS{reset}

{sub}-S, --sync{reset_bold}
{tab}{tab}Synchronize package databases and update packages in target containers. 

{sub}-U, --utils{reset_bold}
{tab}{tab}Invoke miscellaneous utilities to manage containers.

{sub}-P, --process{reset_bold}
{tab}{tab}Manage and show status of running container processes.

{sub}-E, --execute{reset_bold}
{tab}{tab}Executes application in target container using bubblewrap.

{sub}-h, --help=OPTION{reset_bold}
{tab}{tab}Invoke a printout of this manual to {bold}STDOUT{reset_bold}.

{sub}-V, --version{reset_bold}
{tab}{tab}Display version and copyright information in {bold}STDOUT{reset_bold}.\n")
}

fn execute(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let sub = layout.sub();
    let tab = layout.tab();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();

    writeln!(buf, "{head}EXECUTE{reset}

{sub}-r, --root{reset_bold}
{tab}{tab}Execute operation with fakeroot and fakechroot. Facilitates a command with faked privileges.
	
{sub}-s, --shell{reset_bold}
{tab}{tab}Invoke a bash shell\n")
}

fn meta(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let bold = layout.bold();
    let sub = layout.sub();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();
    let tab = layout.tab();

    writeln!(buf, "{head}HELP{reset}

{sub}-m, --more{reset_bold}
{tab}{tab}When specifying a topic to display, show the default topic in addition to specified options.

{sub}-f, --format=FORMAT{reset_bold}
{tab}{tab}Change output format of help in {bold}STDOUT{reset_bold}. Format options include: 'ansi', 'dumb', 'markdown', and 'man'. 
{tab}{tab}This option is for the express purposes of generating documentation at build time, and has little utility
{tab}{tab}outside the context of package maintenance. 'man' option produces troff-formatted documents for man pages.

{sub}-a, --all, --help=all{reset_bold}
{tab}{tab}Display all help topics.\n")
}

fn sync(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let bold = layout.bold();
    let tab = layout.tab();
    let sub = layout.sub();
    let reset = layout.reset(); 
    let reset_bold = layout.reset_bold();

    writeln!(buf, "{head}SYNCHRONIZATION{reset}

{sub}-y, --refresh{reset_bold}
{tab}{tab}Synchronize remote package databases. Specify up to 2 times to force a refresh.

{sub}-u, --upgrade{reset_bold}
{tab}{tab}Execute aggregate upgrade routine on all or specified containers. Use {bold}-t, --target=TARGET{reset_bold} to limit
{tab}{tab}package synchronization operations to the specified target containers. Packages applicable to 
{tab}{tab}a target {bold}must{reset_bold} be specified only after the target operand. 
{tab}{tab}e.g. '-t electron element-desktop -t mozilla firefox thunderbird'

{sub}-f, --filesystem{reset_bold}
{tab}{tab}Force execution of filesystem synchronization target on all or specified containers. In combination
{tab}{tab}with {bold}-o/--target-only{reset_bold}, in addition to no other specified targets, filesystem slices will be
{tab}{tab}synchronized without package synchronization on all applicable containers.

{sub}-c, --create{reset_bold}
{tab}{tab}Create a container with the first specified target. A container type argument is also required.

{sub}-b, --base{reset_bold}
{tab}{tab}Base container type. Specify alongside {bold}-c, --create{reset_bold} to assign this container type during creation.
{tab}{tab}This container type is used as the base layer for all downstream containers. Only one base container 
{tab}{tab}dependency per slice or per root is supported. Filesystem and package deduplication via slices 
{tab}{tab}and root containers is recommended, but optional.

{sub}-s, --slice{reset_bold}
{tab}{tab}Slice container type. Specify alongside {bold}-c, --create{reset_bold} to assign this container type during creation.
{tab}{tab}Requires a base dependency be specified, and one or more sliced dependencies, in order to ascertain
{tab}{tab}foreign packages and influence ordering of downstream synchronization target(s). Container slicing 
{tab}{tab}provides the ability to install packages in a lightweight, sliced filesytem, which aid in the 
{tab}{tab}deduplication of common downstream package and filesystem dependencies e.g. graphics drivers, 
{tab}{tab}graphical toolkits, fonts, etc..

{sub}-r, --root{reset_bold}
{tab}{tab}Root container type. Specify alongside {bold}-c, --create{reset_bold} to this assign container type during creation.
{tab}{tab}Requires a base dependency be specified, and optionally one or more sliced dependencies, in order 
{tab}{tab}to ascertain foreign packages and influence ordering of this target. These containers are ideal 
{tab}{tab}for installing software in with the least amount of filesystem and package synchronization overhead.

{sub}-t, --target=TARGET{reset_bold}
{tab}{tab}Specify a target container for the specified operation.

{sub}-d, --dep=DEPEND{reset_bold}
{tab}{tab}Specify a dependency container for the specified operation.

{sub}-o, --target-only{reset_bold}
{tab}{tab}Apply specified operation on the specified target only.

{sub}--force-foreign{reset_bold}
{tab}{tab}Force synchronization of foreign packages on resident container. Useful for when installing 
{tab}{tab}a new package in a root container without all the prerequisite foreign dependencies synchronized 
{tab}{tab}to this container's package database.

{sub}--dbonly{reset_bold}
{tab}{tab}Transact on resident containers with a database-only transaction.

{sub}--noconfirm{reset_bold}
{tab}{tab}Override confirmation prompts and confirm all operations.\n")

}

fn process(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let tab = layout.tab();
    let reset = layout.reset();
 
    writeln!(buf, "{head}PROCESS{reset}
{tab}{tab}-TODO-\n")
}

fn utils(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let tab = layout.tab();
    let reset = layout.reset();

    writeln!(buf, "{head}UTILITIES{reset}
{tab}{tab}-TODO-\n")
}

fn version(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let sub = layout.sub();
    let sub_text = layout.sub_text();
    let tab = layout.tab();
    let bold = layout.bold();
    let reset = layout.reset();
    let reset_bold = layout.reset_bold();
    let name = env!("CARGO_PKG_NAME");
    let suffix = env!("PACWRAP_BUILDSTAMP");
    let timestamp = env!("PACWRAP_BUILDTIME");
    let release = env!("PACWRAP_BUILD");
    let version_num = env!("CARGO_PKG_VERSION");

    writeln!(buf, "{head}VERSION{reset}

{sub}-V, --version, --version=min{reset_bold}
{tab}{tab}Sends version information to {bold}STDOUT{reset_bold} with colourful ASCII art. 
{tab}{tab}The 'min' option provides a minimalistic output as is provided to non-colour terms.

{sub_text}This documentation was generated by {name} v{version_num}-{suffix}-{release} ({timestamp}).
{tab}Please seek relevant documentation if '{name} -V' mismatches with the aforementioned.\n")
}

fn copyright(buf: &mut String, layout: &HelpLayout) -> Result<(), std::fmt::Error> {
    let head = layout.head();
    let tab = layout.tab();
    let reset = layout.reset();

    writeln!(buf, "{head}COPYRIGHT{reset}

{tab}{tab}Copyright (C) 2023 Xavier R.M.

{tab}{tab}This program may be freely redistributed under
{tab}{tab}the terms of the GNU General Public License v3.\n")
}

pub fn print_version(mut args: &mut Arguments) -> Result<(), ErrorKind> {
    let name = env!("CARGO_PKG_NAME"); 
    let version = env!("CARGO_PKG_VERSION"); 
    let suffix = env!("PACWRAP_BUILDSTAMP");
    let timestamp = env!("PACWRAP_BUILDTIME");
    let release = env!("PACWRAP_BUILD");

    if ! minimal(&mut args) && is_truecolor_terminal() {
        println!("\x1b[?7l\n               [0m[38;2;8;7;6m [0m[38;2;35;31;23mR[0m[38;2;62;56;41mP[0m[38;2;90;81;58mA[0m[38;2;117;105;76mA[0m[38;2;146;131;94mC[0m[38;2;174;156;111mW[0m[38;2;204;182;130mW[0m[38;2;225;200;142mR[0m[38;2;196;173;120mR[0m[38;2;149;130;91mA[0m[38;2;101;88;62mA[0m[38;2;53;46;33mP[0m[38;2;10;8;6m                 [0m
        [0m[38;2;14;12;10m [0m[38;2;40;36;26mR[0m[38;2;67;60;43mA[0m[38;2;93;83;60mP[0m[38;2;120;107;77mP[0m[38;2;147;132;95mP[0m[38;2;175;157;112mA[0m[38;2;201;180;129mC[0m[38;2;225;202;144mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;221;195;135mC[0m[38;2;180;158;110mA[0m[38;2;134;118;82mP[0m[38;2;86;76;53mA[0m[38;2;38;34;24mR[0m[38;2;3;3;2m            [0m
[0m[38;2;9;8;6m [0m[38;2;38;34;25mR[0m[38;2;66;59;43mA[0m[38;2;94;84;60mP[0m[38;2;123;109;79mP[0m[38;2;151;135;97mP[0m[38;2;180;161;114mA[0m[38;2;209;190;115mC[0m[38;2;234;216;110m#[0m[38;2;238;221;100m#[0m[38;2;238;222;99m#[0m[38;2;237;219;106m#[0m[38;2;234;214;123m#[0m[38;2;230;207;143mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;211;186;129mC[0m[38;2;165;145;101mA[0m[38;2;117;103;72mP[0m[38;2;69;61;43mA[0m[38;2;22;19;14mR       [0m
[0m[38;2;94;84;60mP[0m[38;2;227;202;140mC[0m[38;2;229;204;143mC[0m[38;2;230;206;146mC[0m[38;2;234;214;122m#[0m[38;2;244;234;62m#[0m[38;2;252;248;20m#[0m[38;2;255;255;1m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;253;251;13m#[0m[38;2;246;237;53m#[0m[38;2;236;218;109m#[0m[38;2;230;206;145mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;228;201;138mC[0m[38;2;194;170;118mA[0m[38;2;147;129;90mP[0m[38;2;97;85;60mP[0m[38;2;46;41;30mR[0m[38;2;6;5;4m   [0m
[0m[38;2;90;80;57mP[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;202;137mC[0m[38;2;237;222;85m#[0m[38;2;249;243;29m#[0m[38;2;255;255;1m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;1m#[0m[38;2;246;238;51m#[0m[38;2;231;209;138m#[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;216;190;129mC[0m[38;2;169;148;99mA[0m[38;2;66;59;39mA[0m	{name} v{version}-{suffix}-{release} ({timestamp})
[0m[38;2;75;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;136mC[0m[38;2;231;211;112mC[0m[38;2;242;230;63m#[0m[38;2;252;249;17m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;254;253;6m#[0m[38;2;250;246;27m#[0m[38;2;247;239;48m#[0m[38;2;243;231;70m#[0m[38;2;234;215;120m#[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;236;219;177m#[0m[38;2;241;230;205m#[0m[38;2;241;231;207m#[0m[38;2;237;222;185m#[0m[38;2;230;206;147mC[0m[38;2;230;202;139mC[0m[38;2;231;205;147mC[0m[38;2;239;224;190m#[0m[38;2;242;232;209m#[0m[38;2;241;230;205m#[0m[38;2;236;217;174m#[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;231;206;148mC[0m[38;2;239;224;190m#[0m[38;2;242;231;208m#[0m[38;2;241;229;202m#[0m[38;2;234;213;164m#[0m[38;2;228;201;135mC[0m[38;2;227;200;132mC[0m[38;2;226;199;130mC[0m[38;2;225;198;127mC[0m[38;2;223;197;124mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m	Copyright (C) 2023 Xavier R.M.
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;227;202;134mC[0m[38;2;234;216;99m#[0m[38;2;241;229;68m#[0m[38;2;243;232;67m#[0m[38;2;238;222;99m#[0m[38;2;231;209;137m#[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;234;216;169m#[0m[38;2;249;249;248m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;237;222;184m#[0m[38;2;230;202;139mC[0m[38;2;240;227;197m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;249;249;248m#[0m[38;2;233;210;158m#[0m[38;2;229;201;137mC[0m[38;2;235;218;175m#[0m[38;2;236;223;187m#[0m[38;2;232;214;165m#[0m[38;2;227;205;143mC[0m[38;2;223;198;126mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;227;204;130mC[0m[38;2;236;220;89m#[0m[38;2;246;238;45m#[0m[38;2;250;244;31m#[0m[38;2;246;238;51m#[0m[38;2;242;230;73m#[0m[38;2;234;214;123m#[0m[38;2;230;206;146mC[0m[38;2;230;206;147mC[0m[38;2;236;220;181m#[0m[38;2;241;231;207m#[0m[38;2;241;231;208m#[0m[38;2;236;221;184m#[0m[38;2;228;204;143mC[0m[38;2;228;202;138mC[0m[38;2;230;204;144mC[0m[38;2;237;221;182m#[0m[38;2;238;227;196m#[0m[38;2;234;219;178m#[0m[38;2;229;209;152m#[0m[38;2;223;197;124mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;230;209;117mC[0m[38;2;240;228;70m#[0m[38;2;244;234;58m#[0m[38;2;230;207;134mC[0m[38;2;228;203;141mC[0m[38;2;227;202;139mC[0m[38;2;226;202;138mC[0m[38;2;226;201;136mC[0m[38;2;225;200;133mC[0m[38;2;225;199;131mC[0m[38;2;224;198;128mC[0m[38;2;224;198;126mC[0m[38;2;223;197;124mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m	Website: https://pacwrap.sapphirus.org/
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;225;199;132mC[0m[38;2;224;198;128mC[0m[38;2;223;197;125mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;63mP[0m	Github: https://github.com/sapphirusberyl/pacwrap
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;63mP[0m
[0m[38;2;56;50;35mA[0m[38;2;218;194;133mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;63mP[0m
 [0m[38;2;31;27;20mR[0m[38;2;141;125;87mP[0m[38;2;221;197;135mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;116;103;66mP[0m	This program may be freely redistributed under
   [0m[38;2;41;36;26mR[0m[38;2;160;143;99mP[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;119;105;69mP[0m	the terms of the GNU General Public License v3.
    [0m[38;2;1;1;1m [0m[38;2;62;55;39mA[0m[38;2;172;153;106mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;200;176;111mC[0m[38;2;144;127;80mP[0m[38;2;88;77;50mA[0m[38;2;33;30;20mR[0m[38;2;1;1;1m [0m
      [0m[38;2;1;1;1m [0m[38;2;68;60;43mA[0m[38;2;181;161;112mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;211;187;118mC[0m[38;2;159;141;89mP[0m[38;2;101;90;57mP[0m[38;2;45;40;26mR[0m[38;2;3;3;2m     [0m
        [0m[38;2;3;3;2m [0m[38;2;83;74;52mA[0m[38;2;192;171;118mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;218;193;121mC[0m[38;2;173;153;97mA[0m[38;2;116;102;65mP[0m[38;2;59;52;34mA[0m[38;2;10;8;6m         [0m
          [0m[38;2;8;7;5m [0m[38;2;95;85;59mP[0m[38;2;202;180;124mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;198;128mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;193;170;108mA[0m[38;2;136;121;77mP[0m[38;2;77;68;43mA[0m[38;2;18;16;11m             [0m
            [0m[38;2;12;11;8m [0m[38;2;115;102;72mP[0m[38;2;214;190;131mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;224;199;129mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;205;181;115mC[0m[38;2;150;133;84mP[0m[38;2;93;82;53mP[0m[38;2;35;31;21mR[0m[38;2;1;1;1m                [0m
              [0m[38;2;25;22;16mR[0m[38;2;127;113;79mP[0m[38;2;216;192;132mC[0m[38;2;226;201;137mC[0m[38;2;224;199;129mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;211;186;117mC[0m[38;2;157;139;88mP[0m[38;2;103;91;58mP[0m[38;2;48;42;28mR[0m[38;2;5;4;3m                    [0m
                [0m[38;2;28;25;18mR[0m[38;2;137;123;85mP[0m[38;2;215;190;125mC[0m[38;2;174;154;98mA[0m[38;2;118;104;66mP[0m[38;2;61;54;35mA[0m[38;2;9;8;5m                        [0m\n\x1b[?7h");
    } else {
        println!("{name} v{version}-{suffix}-{release} ({timestamp})
Copyright (C) 2023 Xavier R.M.

Website: https://pacwrap.sapphirus.org/
Github: https://github.com/sapphirusberyl/pacwrap

This program may be freely redistributed under
the terms of the GNU General Public License v3 only.\n");
    }
    Ok(())
}