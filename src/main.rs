// Copyright 2018 Sebastian Wiesner <sebastian@swsnr.de>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

// 	http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![deny(warnings)]
// Warn about deprecated trait object syntax
#![deny(bare_trait_objects)]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]

//! Show CommonMark documents on TTYs.

#[macro_use]
extern crate clap;
extern crate mdcat;
extern crate pulldown_cmark;
extern crate syntect;

use pulldown_cmark::Parser;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{stdin, stdout, Stdout};
use std::path::PathBuf;
use std::str::FromStr;
use syntect::parsing::SyntaxSet;

use mdcat::{detect_terminal, AnsiTerminal, DumbTerminal, ResourceAccess, Terminal, TerminalSize};

/// Colour options, for the --colour option.
#[derive(Debug, Clone, PartialEq)]
enum Colour {
    Yes,
    No,
    Auto,
}

#[derive(Debug)]
struct InvalidColour {}

impl ToString for InvalidColour {
    fn to_string(&self) -> String {
        String::from("invalid colour setting")
    }
}

impl FromStr for Colour {
    type Err = InvalidColour;

    fn from_str(value: &str) -> Result<Self, InvalidColour> {
        match value.to_lowercase().as_str() {
            "yes" => Ok(Colour::Yes),
            "no" => Ok(Colour::No),
            "auto" => Ok(Colour::Auto),
            _ => Err(InvalidColour {}),
        }
    }
}

/// Read input for `filename`.
///
/// If `filename` is `-` read from standard input, otherwise try to open and
/// read the given file.
fn read_input<T: AsRef<str>>(filename: T) -> std::io::Result<(PathBuf, String)> {
    let cd = std::env::current_dir()?;
    let mut buffer = String::new();

    if filename.as_ref() == "-" {
        stdin().read_to_string(&mut buffer)?;
        Ok((cd, buffer))
    } else {
        let mut source = File::open(filename.as_ref())?;
        source.read_to_string(&mut buffer)?;
        let base_dir = cd
            .join(filename.as_ref())
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(cd);
        Ok((base_dir, buffer))
    }
}

fn process_arguments(size: TerminalSize, args: Arguments) -> Result<(), Box<dyn Error>> {
    if args.detect_only {
        println!("Terminal: {}", args.terminal.name());
        Ok(())
    } else {
        let (base_dir, input) = read_input(args.filename)?;
        let parser = Parser::new(&input);

        if args.dump_events {
            mdcat::dump_events(&mut std::io::stdout(), parser)?;
            Ok(())
        } else {
            let syntax_set = SyntaxSet::load_defaults_newlines();
            mdcat::push_tty(
                args.terminal,
                TerminalSize {
                    width: args.columns,
                    ..size
                },
                parser,
                &base_dir,
                args.resource_access,
                syntax_set,
            )?;
            Ok(())
        }
    }
}

/// Represent command line arguments.
struct Arguments {
    filename: String,
    terminal: Box<dyn Terminal<TerminalWrite = Stdout>>,
    resource_access: ResourceAccess,
    columns: usize,
    dump_events: bool,
    detect_only: bool,
}

impl Arguments {
    /// Create command line arguments from matches.
    fn from_matches(matches: &clap::ArgMatches) -> clap::Result<Self> {
        let colour = value_t!(matches, "colour", Colour)?;
        let terminal = if colour == Colour::No {
            Box::new(DumbTerminal::new(stdout()))
        } else {
            let auto = detect_terminal();
            if !auto.supports_styles() && colour == Colour::Yes {
                Box::new(AnsiTerminal::new(stdout()))
            } else {
                auto
            }
        };
        let filename = value_t!(matches, "filename", String)?;
        let dump_events = matches.is_present("dump_events");
        let detect_only = matches.is_present("detect_only");
        let columns = value_t!(matches, "columns", usize)?;
        let resource_access = if matches.is_present("local_only") {
            ResourceAccess::LocalOnly
        } else {
            ResourceAccess::RemoteAllowed
        };

        Ok(Arguments {
            filename,
            columns,
            resource_access,
            dump_events,
            detect_only,
            terminal,
        })
    }
}

fn main() {
    use clap::*;
    let size = TerminalSize::detect().unwrap_or_default();
    let columns = size.width.to_string();
    let app = app_from_crate!()
        // Merge flags and options w/ arguments together, include args in usage
        // string and show options in the order of declaration.  And also:
        // COLOURS <3
        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::DontCollapseArgsInUsage)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::ColoredHelp)
        .after_help(
            "mdcat uses the standardized CommonMark dialect.  It formats
markdown documents for viewing in text terminals:

• Colours for headings, block quotes, etc
• Syntax highlighting for code blocks
• In some terminals: Inline images and inline links
• In iTerm2: Jump marks for headings

Copyright (C) 2018 Sebastian Wiesner
Licensed under the Apache License, Version 2.0
Report issues to <https://github.com/lunaryorn/mdcat>.",
        )
        .arg(
            Arg::with_name("filename")
                .help("The file to read.  If - read from standard input instead")
                .default_value("-")
        )
        .arg(
            Arg::with_name("colour")
                .short("c")
                .long("colour")
                .help("Whether to enable colours.")
                .possible_values(&["yes", "no", "auto"])
                .default_value("auto")
        )
        .arg(
            Arg::with_name("columns")
                .long("columns")
                .help("Maximum number of columns to use for output")
                .default_value(&columns)
        )
        .arg(
            Arg::with_name("local_only")
            .short("l")
            .long("local")
            .help("Do not load remote resources like images")
        )
        .arg(
            Arg::with_name("dump_events")
                .long("dump-events")
                .help("Dump Markdown parser events and exit")
                .hidden(true)
        )
        .arg(
            Arg::with_name("detect_only")
            .long("detect-only")
            .help("Only detect the terminal type and exit")
            .hidden(true)
        );

    let matches = app.get_matches();
    let arguments = Arguments::from_matches(&matches).unwrap_or_else(|e| e.exit());
    match process_arguments(size, arguments) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
