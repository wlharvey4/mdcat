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

//! Tools for syntax highlighting.

use super::types::{AnsiColour, AnsiStyle};
use super::write::Terminal;
use failure::Error;
use std::io::Write;
use syntect::highlighting::{FontStyle, Style};

/// Write regions as ANSI 8-bit coloured text.
///
/// We use this function to simplify syntax highlighting to 8-bit ANSI values
/// which every theme provides.  Contrary to 24 bit colours this gives us a good
/// guarantee that highlighting works with any terminal colour theme, whether
/// light or dark, and saves us all the hassle of mismatching colours.
///
/// We assume Solarized colours here: Solarized cleanly maps to 8-bit ANSI
/// colours so we can safely map its RGB colour values back to ANSI colours.  We
/// do so for all accent colours, but leave "base*" colours alone: Base colours
/// change depending on light or dark Solarized; to address both light and dark
/// backgrounds we must map all base colours to the default terminal colours.
///
/// Furthermore we completely ignore any background colour settings, to avoid
/// conflicts with the terminal colour themes.
pub fn write_as_ansi<W: Write>(
    terminal: &mut dyn Terminal<TerminalWrite = W>,
    regions: &[(Style, &str)],
) -> Result<(), Error> {
    for &(style, text) in regions {
        let rgb = {
            let fg = style.foreground;
            (fg.r, fg.g, fg.b)
        };
        match rgb {
            // base03, base02, base01, base00, base0, base1, base2, and base3
            (0x00, 0x2b, 0x36)
            | (0x07, 0x36, 0x42)
            | (0x58, 0x6e, 0x75)
            | (0x65, 0x7b, 0x83)
            | (0x83, 0x94, 0x96)
            | (0x93, 0xa1, 0xa1)
            | (0xee, 0xe8, 0xd5)
            | (0xfd, 0xf6, 0xe3) => terminal.set_style(AnsiStyle::DefaultForeground)?,
            (0xb5, 0x89, 0x00) => terminal.set_style(AnsiStyle::Foreground(AnsiColour::Yellow))?, // yellow
            (0xcb, 0x4b, 0x16) => terminal.set_style(AnsiStyle::Foreground(AnsiColour::LightRed))?, // orange
            (0xdc, 0x32, 0x2f) => terminal.set_style(AnsiStyle::Foreground(AnsiColour::Red))?, // red
            (0xd3, 0x36, 0x82) => terminal.set_style(AnsiStyle::Foreground(AnsiColour::Magenta))?, // magenta
            (0x6c, 0x71, 0xc4) => {
                terminal.set_style(AnsiStyle::Foreground(AnsiColour::LightMagenta))?
            } // violet
            (0x26, 0x8b, 0xd2) => terminal.set_style(AnsiStyle::Foreground(AnsiColour::Blue))?, // blue
            (0x2a, 0xa1, 0x98) => terminal.set_style(AnsiStyle::Foreground(AnsiColour::Cyan))?, // cyan
            (0x85, 0x99, 0x00) => terminal.set_style(AnsiStyle::Foreground(AnsiColour::Green))?, // green
            (r, g, b) => panic!("Unexpected RGB colour: #{:2>0x}{:2>0x}{:2>0x}", r, g, b),
        };
        let font = style.font_style;
        if font.contains(FontStyle::BOLD) {
            terminal.set_style(AnsiStyle::Bold)?;
        };
        if font.contains(FontStyle::ITALIC) {
            terminal.set_style(AnsiStyle::Italic)?;
        };
        if font.contains(FontStyle::UNDERLINE) {
            terminal.set_style(AnsiStyle::Underline)?;
        };
        terminal.write().write_all(text.as_bytes())?;
        terminal.set_style(AnsiStyle::Reset)?;
    }

    Ok(())
}
