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

use std::path::Path;

#[derive(Clone)]
/// A file checker
pub struct FileCheck {
    /// The file's path
    pub path: String,

    /// The command to execute at the end
    pub cmd: String,

    /// The previous size of the file
    pub size: u64,

    /// The next check in milliseconds
    pub next_check: i64,

    /// The check interval in milliseconds
    pub check_interval: i64,
}

impl FileCheck {
    /// Remove time from next check
    ///
    /// # Parameters
    ///
    /// * `time`: The time passed in milliseconds
    pub fn tick(&mut self, time: i64) {
        self.next_check -= time;
    }

    /// Check if file has changed
    ///
    /// If `true`, the command will not be executed
    pub fn has_changed(&mut self) -> bool {
        // If it's not time to check, we retrun true to not trigger the command
        if self.next_check > 0 {
            return true;
        }

        // If file does not exist, we set the size to zero
        let file = Path::new(&self.path);
        if !file.exists() {
            print!("Warning: file does not exist: {}", self.path);
        }

        // Same with metadata reading
        let metadata = std::fs::metadata(&self.path);

        // Size extraction
        let new_size = match metadata {
            Ok(v) => v.len(),
            Err(e) => {
                print!("Warning: error while reading file metadata: {}", e);
                0
            }
        };

        println!(
            "File {} checked, was {} bytes long, now {}",
            &self.path, self.size, new_size
        );

        // If size hadn't changed, we trigger the command
        if new_size == self.size {
            return false;
        }

        // If not, we reset for a new check

        self.size = new_size;
        self.next_check = self.check_interval;
        true
    }

    /// Creates a new file checker
    ///
    /// # Parameters
    ///
    /// * `path`: The file to check
    /// * `check_interval`: The check time interval in milliseconds
    pub fn new(path: &str, check_interval: i64, cmd: &str) -> Self {
        Self {
            path: path.to_string(),
            size: 0,
            next_check: check_interval,
            check_interval,
            cmd: cmd.to_string(),
        }
    }
}
