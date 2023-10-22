use indexmap::IndexSet;
use lazy_static::lazy_static;

use crate::utils::{Arguments, arguments::Operand, print_help_error, is_color_terminal, is_truecolor_terminal};

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

pub fn help(mut args: Arguments) {
    let help = ascertain_help(&mut args);

    for topic in help.0 {
        topic.display(help.1);
    }
}

fn ascertain_help<'a>(args: &mut Arguments) -> (IndexSet<&'a HelpTopic>, &'a HelpLayout) {
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
                | Operand::Short('h') => continue,
            Operand::Short('m') | Operand::Long("more") => more = true,
            Operand::LongPos("format", "dumb") | Operand::ShortPos('f', "dumb") => layout = &HelpLayout::Dumb, 
            Operand::LongPos("format", "markdown") | Operand::ShortPos('f', "markdown") => layout = &HelpLayout::Markdown,
            Operand::LongPos("format", "man") | Operand::ShortPos('f', "man") => layout = &HelpLayout::Man,
            Operand::LongPos("format", "console") | Operand::ShortPos('f', "console") => layout = &HelpLayout::Console,
            Operand::ShortPos('h', "sync")
                | Operand::ShortPos('h', "S")
                | Operand::LongPos("help", "sync")
                | Operand::LongPos("help", "S") => topic.push(&HelpTopic::Sync),
            Operand::ShortPos('h', "E")
                | Operand::ShortPos('h', "exec")
                | Operand::LongPos("help", "E")
                | Operand::LongPos("help", "exec") => topic.push(&HelpTopic::Execute),
            Operand::ShortPos('h', "process")
                | Operand::ShortPos('h', "P")
                | Operand::LongPos("help", "process")
                | Operand::LongPos("help", "P")=> topic.push(&HelpTopic::Process),
            Operand::ShortPos('h', "utils")
                | Operand::ShortPos('h', "U")
                | Operand::LongPos("help", "utils")
                | Operand::LongPos("help", "U")=> topic.push(&HelpTopic::Utils),
            Operand::ShortPos('h', "help")
                | Operand::ShortPos('h', "h")
                | Operand::LongPos("help", "help")
                | Operand::LongPos("help", "h")=> topic.push(&HelpTopic::Help),
            Operand::ShortPos('h', "synopsis") | Operand::LongPos("help", "synopsis") => topic.push(&HelpTopic::Default),
            Operand::ShortPos('h', "V")
                | Operand::ShortPos('h', "version")
                | Operand::LongPos("help", "V")
                | Operand::LongPos("help", "version")
                => topic.push(&HelpTopic::Version),
            Operand::ShortPos('h', "copyright") | Operand::LongPos("help", "copyright") => topic.push(&HelpTopic::Copyright),
            Operand::ShortPos('h', "all")
                | Operand::LongPos("help", "all")
                | Operand::Short('a')
                | Operand::Long("all") => topic.extend(HELP_ALL.iter()),
            Operand::ShortPos('h', topic) | Operand::LongPos("help", topic) => print_help_error(format!("Topic '{topic}' is not available.")),
           _ => args.invalid_operand(),
        }
    }

    let len = topic.len();
    let start = if more || len == 1 || len > 7 { 0 } else { 1 }; 

    args.set_index(1);
    (topic.drain(start..).collect(), layout)
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
            Self::Markdown | Self::Man => "# ",
            Self::Dumb => "",
        }
    }

    fn sub(&self) -> &str {
        match self {
            Self::Console => "   [37;1m",
            Self::Markdown | Self::Man => "**",
            Self::Dumb => "   ",
        }
    }

    #[allow(dead_code)]
    fn reset(&self) -> &str {
        match self {
            Self::Console => "[0m",
            Self::Markdown => "",
            Self::Man | Self::Dumb => ""
        }
    }

    fn reset_bold(&self) -> &str {
        match self {
            Self::Console => "[0m",
            Self::Markdown => "**",
            Self::Man | Self::Dumb => ""
        }
    }

    fn bold(&self) -> &str {
        match self {
            Self::Console => "[37;1m",
            Self::Markdown | Self::Man => "**",
            Self::Dumb => "",
        }
    }

    fn sub_text(&self) -> &str {
        match self {
            Self::Console | Self::Dumb => "      ",
            Self::Markdown => " ",
            Self::Man => ": ",
        }
    }
}

impl HelpTopic {
    fn display(&self, layout: &HelpLayout) {
        match self {
            Self::Default => default(layout),
            Self::Sync => sync(layout),
            Self::Execute => execute(layout),
            Self::Process => process(layout),
            Self::Utils => utils(layout),
            Self::Help => meta(layout),
            Self::Copyright => copyright(layout),
            Self::Version => version(layout),
        }
    }
}

fn default(layout: &HelpLayout) {
    let head = layout.head();
    let sub = layout.sub();
    let sub_text = layout.sub_text();
    let bold = layout.bold();
    let reset_bold = layout.reset_bold();

    println!("{head}NAME{reset_bold}
{sub_text}pacwrap - Command-line application which facilitates the creation, management, and execution of unprivileged, 
{sub_text}Sandboxed containers with bubblewrap and libalpm.

{head}SYNOPSIS{reset_bold}
{sub_text}pacwrap [{bold}OPERATIONS{reset_bold}] [{bold}ARGuMENTS{reset_bold}] [{bold}TARGET(S){reset_bold}]	

{head}OPERATIONS{reset_bold}

{sub}-S, --sync{reset_bold}
{sub_text}Synchronize package databases and update packages in target containers. 

{sub}-U, --utils{reset_bold}
{sub_text}Invoke miscellaneous utilities to manage containers.

{sub}-P, --process{reset_bold}
{sub_text}Manage and show status of running container processes.

{sub}-E, --execute{reset_bold}
{sub_text}Executes application in target container using bubblewrap.

{sub}-h, --help{reset_bold}
{sub_text}Invoke a printout of this manual to {bold}stdout{reset_bold}. Specify an option verbatim for further information.

{sub}-V, --version{reset_bold}
{sub_text}Display version and copyright information in {bold}STDOUT{reset_bold}.\n");
}

fn execute(layout: &HelpLayout) {
    let head = layout.head();
    let sub = layout.sub();
    let sub_text = layout.sub_text();
    let reset_bold = layout.reset_bold();

    println!("{head}EXECUTE{reset_bold}

{sub}-r, --root{reset_bold}
{sub_text}Execute operation with fakeroot and fakechroot. Facilitates a command with faked privileges.
	
{sub}-s, --shell{reset_bold}
{sub_text}Invoke a bash shell\n");
}

fn meta(layout: &HelpLayout) {
    let head = layout.head();
    let sub = layout.sub();
    let sub_text = layout.sub_text();
    let reset_bold = layout.reset_bold();

    println!("{head}HELP{reset_bold}

{sub}-m, --more{reset_bold}
{sub_text}When specifying a topic to display, show the default topic in addition to specified options.

{sub}-f, --format=FORMAT{reset_bold}
{sub_text}Change output format of help in STDOUT. Format options include: 'console', 'markdown', and 'man'. 
{sub_text}This option is for the express purposes of generating documentation at build time, and has little utility
{sub_text}outside the context of package maintenance. --format=man presently requires go-md2man to parse output.

{sub}-a, --all, --help=all{reset_bold}
{sub_text}Display all help topics.\n");
}

fn sync(layout: &HelpLayout) {
    let head = layout.head();
    let sub = layout.sub();
    let sub_text = layout.sub_text();
    let reset_bold = layout.reset_bold();

    println!("{head}SYNCHRONIZATION{reset_bold}
{sub}-y, --refresh{reset_bold}
{sub_text}Synchronize remote database. Specify up to 2 times to force a refresh.

{sub}-u, --upgrade{reset_bold}
{sub_text}Execute aggregate upgrade routine on all or specified containers.

{sub}-f, --filesystem{reset_bold}
{sub_text}Force execution of filesystem synchronization coroutines on all or specified containers.

{sub}--dbonly{reset_bold}
{sub_text}Transact on resident containers with a database-only transaction.

{sub}--force-foreign{reset_bold}
{sub_text}Force synchronization of foreign packages on resident container.

{sub}--dbonly{reset_bold}
{sub_text}Override confirmation prompts and confirm all operations.\n");
}

fn process(layout: &HelpLayout) {
    let head = layout.head();
    let sub_text = layout.sub_text();
    let reset_bold = layout.reset_bold();

    println!("{head}PROCESS{reset_bold}
{sub_text}-TODO-\n");
}

fn utils(layout: &HelpLayout) {
    let head = layout.head();
    let sub_text = layout.sub_text();
    let reset_bold = layout.reset_bold();

    println!("{head}UTILITIES{reset_bold}
{sub_text}-TODO-\n");
}

fn version(layout: &HelpLayout) {
    let head = layout.head();
    let sub = layout.sub();
    let sub_text = layout.sub_text();
    let reset_bold = layout.reset_bold();
    let name = env!("CARGO_PKG_NAME");
    let suffix = match option_env!("GIT_HEAD") {
        Some(suf) => format!("-{suf}"), None => String::new(),
    }; 
    let version_num = env!("CARGO_PKG_VERSION");

    println!("{head}VERSION{reset_bold}

{sub}NOTICE{reset_bold}
{sub_text}This documentation pertains to version string '{name} {version_num}{suffix}'
{sub_text}Please seek relevant documentation if '{name} -v' mismatches with the above.

{sub}-v, --version, --version=min{reset_bold}
{sub_text}Sends version information to STDOUT with colourful ASCII art. 
{sub_text}The 'min' option provides a minimalistic output as is provided to non-colour terms.\n");
}

fn copyright(layout: &HelpLayout) {
    let head = layout.head();
    let sub_text = layout.sub_text();
    let reset_bold = layout.reset_bold();

    println!("{head}COPYRIGHT{reset_bold}
{sub_text}Copyright (C) 2023 - Xavier R.M.

{sub_text}This program may be freely redistributed under
{sub_text}the terms of the GNU General Public License v3.\n");
}

pub fn print_version(mut args: Arguments) {
    let name = env!("CARGO_PKG_NAME"); 
    let version = env!("CARGO_PKG_VERSION"); 
    let suffix = match option_env!("GIT_HEAD") {
        Some(suf) => format!("-{suf}"), None => String::new(),
    }; 

    if ! minimal(&mut args) && is_truecolor_terminal() {
        println!("\n               [0m[38;2;8;7;6m [0m[38;2;35;31;23mR[0m[38;2;62;56;41mP[0m[38;2;90;81;58mA[0m[38;2;117;105;76mA[0m[38;2;146;131;94mC[0m[38;2;174;156;111mW[0m[38;2;204;182;130mW[0m[38;2;225;200;142mR[0m[38;2;196;173;120mR[0m[38;2;149;130;91mA[0m[38;2;101;88;62mA[0m[38;2;53;46;33mP[0m[38;2;10;8;6m                 [0m
        [0m[38;2;14;12;10m [0m[38;2;40;36;26mR[0m[38;2;67;60;43mA[0m[38;2;93;83;60mP[0m[38;2;120;107;77mP[0m[38;2;147;132;95mP[0m[38;2;175;157;112mA[0m[38;2;201;180;129mC[0m[38;2;225;202;144mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;221;195;135mC[0m[38;2;180;158;110mA[0m[38;2;134;118;82mP[0m[38;2;86;76;53mA[0m[38;2;38;34;24mR[0m[38;2;3;3;2m            [0m
[0m[38;2;9;8;6m [0m[38;2;38;34;25mR[0m[38;2;66;59;43mA[0m[38;2;94;84;60mP[0m[38;2;123;109;79mP[0m[38;2;151;135;97mP[0m[38;2;180;161;114mA[0m[38;2;209;190;115mC[0m[38;2;234;216;110m#[0m[38;2;238;221;100m#[0m[38;2;238;222;99m#[0m[38;2;237;219;106m#[0m[38;2;234;214;123m#[0m[38;2;230;207;143mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;211;186;129mC[0m[38;2;165;145;101mA[0m[38;2;117;103;72mP[0m[38;2;69;61;43mA[0m[38;2;22;19;14mR       [0m
[0m[38;2;94;84;60mP[0m[38;2;227;202;140mC[0m[38;2;229;204;143mC[0m[38;2;230;206;146mC[0m[38;2;234;214;122m#[0m[38;2;244;234;62m#[0m[38;2;252;248;20m#[0m[38;2;255;255;1m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;253;251;13m#[0m[38;2;246;237;53m#[0m[38;2;236;218;109m#[0m[38;2;230;206;145mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;228;201;138mC[0m[38;2;194;170;118mA[0m[38;2;147;129;90mP[0m[38;2;97;85;60mP[0m[38;2;46;41;30mR[0m[38;2;6;5;4m   [0m
[0m[38;2;90;80;57mP[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;202;137mC[0m[38;2;237;222;85m#[0m[38;2;249;243;29m#[0m[38;2;255;255;1m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;1m#[0m[38;2;246;238;51m#[0m[38;2;231;209;138m#[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;205;144mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;216;190;129mC[0m[38;2;169;148;99mA[0m[38;2;66;59;39mA[0m	{name} v{version}{suffix}
[0m[38;2;75;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;136mC[0m[38;2;231;211;112mC[0m[38;2;242;230;63m#[0m[38;2;252;249;17m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;255;255;0m#[0m[38;2;254;253;6m#[0m[38;2;250;246;27m#[0m[38;2;247;239;48m#[0m[38;2;243;231;70m#[0m[38;2;234;215;120m#[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;236;219;177m#[0m[38;2;241;230;205m#[0m[38;2;241;231;207m#[0m[38;2;237;222;185m#[0m[38;2;230;206;147mC[0m[38;2;230;202;139mC[0m[38;2;231;205;147mC[0m[38;2;239;224;190m#[0m[38;2;242;232;209m#[0m[38;2;241;230;205m#[0m[38;2;236;217;174m#[0m[38;2;230;202;139mC[0m[38;2;230;202;139mC[0m[38;2;231;206;148mC[0m[38;2;239;224;190m#[0m[38;2;242;231;208m#[0m[38;2;241;229;202m#[0m[38;2;234;213;164m#[0m[38;2;228;201;135mC[0m[38;2;227;200;132mC[0m[38;2;226;199;130mC[0m[38;2;225;198;127mC[0m[38;2;223;197;124mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m	Copyright (C) 2023 Xavier R.M.
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;227;202;134mC[0m[38;2;234;216;99m#[0m[38;2;241;229;68m#[0m[38;2;243;232;67m#[0m[38;2;238;222;99m#[0m[38;2;231;209;137m#[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;230;206;146mC[0m[38;2;234;216;169m#[0m[38;2;249;249;248m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;237;222;184m#[0m[38;2;230;202;139mC[0m[38;2;240;227;197m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;249;249;249m#[0m[38;2;249;249;248m#[0m[38;2;233;210;158m#[0m[38;2;229;201;137mC[0m[38;2;235;218;175m#[0m[38;2;236;223;187m#[0m[38;2;232;214;165m#[0m[38;2;227;205;143mC[0m[38;2;223;198;126mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;227;204;130mC[0m[38;2;236;220;89m#[0m[38;2;246;238;45m#[0m[38;2;250;244;31m#[0m[38;2;246;238;51m#[0m[38;2;242;230;73m#[0m[38;2;234;214;123m#[0m[38;2;230;206;146mC[0m[38;2;230;206;147mC[0m[38;2;236;220;181m#[0m[38;2;241;231;207m#[0m[38;2;241;231;208m#[0m[38;2;236;221;184m#[0m[38;2;228;204;143mC[0m[38;2;228;202;138mC[0m[38;2;230;204;144mC[0m[38;2;237;221;182m#[0m[38;2;238;227;196m#[0m[38;2;234;219;178m#[0m[38;2;229;209;152m#[0m[38;2;223;197;124mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;230;209;117mC[0m[38;2;240;228;70m#[0m[38;2;244;234;58m#[0m[38;2;230;207;134mC[0m[38;2;228;203;141mC[0m[38;2;227;202;139mC[0m[38;2;226;202;138mC[0m[38;2;226;201;136mC[0m[38;2;225;200;133mC[0m[38;2;225;199;131mC[0m[38;2;224;198;128mC[0m[38;2;224;198;126mC[0m[38;2;223;197;124mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;64mP[0m	Website: https://pacwrap.sapphirus.org/
[0m[38;2;76;67;47mA[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;226;201;137mC[0m[38;2;225;199;132mC[0m[38;2;224;198;128mC[0m[38;2;223;197;125mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;223;197;123mC[0m[38;2;112;99;63mP[0m	Github: https;//git.sapphirus.org/
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
                [0m[38;2;28;25;18mR[0m[38;2;137;123;85mP[0m[38;2;215;190;125mC[0m[38;2;174;154;98mA[0m[38;2;118;104;66mP[0m[38;2;61;54;35mA[0m[38;2;9;8;5m                        [0m\n");
    } else {
        println!("{name} v{version}{suffix}
Copyright (C) 2023 Xavier R.M.

Website: https://git.sapphirus.org/pacwrap
Github: https://github.com/sapphirusberyl/pacwrap

This program may be freely redistributed under
the terms of the GNU General Public License v3 only.\n");
    }
}
