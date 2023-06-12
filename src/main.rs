// This file is part of rincron-mini <https://github.com/nevermille/rincron-mini>
// Copyright (C) 2022-2023 Camille Nevermind
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![doc = include_str!("../README.md")]

use rincron::Rincron;

/// The file checker
mod file_check;
/// The main program
mod rincron;
/// An event to watch
mod watch_element;
/// The manager of all events
mod watch_manager;

fn main() {
    println!("Rincron-Mini Copyright (C) 2022-2023 Camille Nevermind");
    println!("THIS SOFTWARE IS DISTRIBUTED UNDER GPL-3.0 LICENSE");
    println!("THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND");
    println!("EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES");
    println!("OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.");

    let mut rincron = Rincron::init().unwrap_or_else(|_| std::process::exit(1));
    rincron.execute();
}
