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

use inotify::{Inotify, WatchDescriptor, WatchMask};
use serde_json::{Number, Value};
use simple_error::bail;
use std::path::Path;

/// Inotify watch element
#[derive(Clone, Eq, PartialEq)]
pub struct WatchElement {
    /// The inotify WatchDescriptor
    pub watch_descriptor: WatchDescriptor,

    /// The path string given by the user
    pub path: String,

    /// The command string
    pub command: String,

    /// The masks
    pub mask: WatchMask,

    /// The file_match option
    pub file_match: String,

    /// The time interval in seconds betweek size checks
    pub check_interval: i64,
}

impl WatchElement {
    /// Converts an event string to a WatchMask
    ///
    /// Both `EVENT` and `IN_EVENT` can be used
    fn event_name_to_value(name: &str) -> Option<WatchMask> {
        match name {
            "ATTRIB" | "IN_ATTRIB" => Some(WatchMask::ATTRIB),
            "CLOSE_WRITE" | "IN_CLOSE_WRITE" => Some(WatchMask::CLOSE_WRITE),
            "CLOSE_NOWRITE" | "IN_CLOSE_NOWRITE" => Some(WatchMask::CLOSE_NOWRITE),
            "CREATE" | "IN_CREATE" => Some(WatchMask::CREATE),
            "DELETE" | "IN_DELETE" => Some(WatchMask::DELETE),
            "DELETE_SELF" | "IN_DELETE_SELF" => Some(WatchMask::DELETE_SELF),
            "MODIFY" | "IN_MODIFY" => Some(WatchMask::MODIFY),
            "MOVE_SELF" | "IN_MOVE_SELF" => Some(WatchMask::MOVE_SELF),
            "MOVED_FROM" | "IN_MOVED_FROM" => Some(WatchMask::MOVED_FROM),
            "MOVED_TO" | "IN_MOVED_TO" => Some(WatchMask::MOVED_TO),
            "OPEN" | "IN_OPEN" => Some(WatchMask::OPEN),
            "ALL_EVENTS" | "IN_ALL_EVENTS" => Some(WatchMask::ALL_EVENTS),
            "MOVE" | "IN_MOVE" => Some(WatchMask::MOVE),
            "CLOSE" | "IN_CLOSE" => Some(WatchMask::MOVE),
            "DONT_FOLLOW" | "IN_DONT_FOLLOW" => Some(WatchMask::DONT_FOLLOW),
            "EXCL_UNLINK" | "IN_EXCL_UNLINK" => Some(WatchMask::EXCL_UNLINK),
            "MASK_ADD" | "IN_MASK_ADD" => Some(WatchMask::MASK_ADD),
            "ONESHOT" | "IN_ONESHOT" => Some(WatchMask::ONESHOT),
            "ONLYDIR" | "IN_ONLYDIR" => Some(WatchMask::ONLYDIR),
            _ => None,
        }
    }

    /// Creates an new element from json value and adds it to inotify
    ///
    /// # Parameters
    ///
    /// * `value`: The json value
    /// * `inotify`: The inotify object
    pub fn from_json_value(
        value: &Value,
        inotify: &mut Inotify,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // We need an object
        if !value.is_object() {
            bail!("One item is not an object: {}", value);
        }

        // Data extraction
        let mut path = value.get("path");

        // Will be deprecated in 0.3.0-beta
        if path.is_none() {
            path = value.get("dir");

            if path.is_some() {
                println!("Warning: 'dir' key used instead of 'path', this is deprecated and will be removed in a future version");
            }
        }

        let events = value.get("events");
        let command = value.get("command");

        // Extact parameters with default values
        let file_match = value
            .get("file_match")
            .unwrap_or(&Value::String(String::new()))
            .as_str()
            .unwrap_or_default()
            .to_string();

        let check_interval = value
            .get("check_interval")
            .unwrap_or(&Value::Number(Number::from(0)))
            .as_i64()
            .unwrap_or_default();

        // Integrity checks
        if path.is_none() || events.is_none() || command.is_none() {
            bail!("One parameter is missing between \"dir\", \"events\" and \"command\"");
        }

        let path = path.unwrap();
        let events = events.unwrap();
        let command = command.unwrap();

        if !path.is_string() {
            bail!("\"dir\" must be a string");
        }

        if !events.is_array() {
            bail!("\"events\" must be an array");
        }

        if !command.is_string() {
            bail!("\"command\" must be a string");
        }

        let path = path.as_str().unwrap();
        let events = events.as_array().unwrap();
        let command = command.as_str().unwrap();

        // Path check
        let dir_path = Path::new(path);

        if !dir_path.exists() {
            bail!("\"{}\" does not exist", path);
        }

        let in_dir = path;
        let mut in_events: Option<WatchMask> = None;

        // Events extraction
        for event in events {
            if !event.is_string() {
                println!("One event is not a string: {}", event);
                continue;
            }

            let event_name = event.as_str().unwrap();
            let detected = Self::event_name_to_value(event_name);

            if let Some(e) = detected {
                if in_events.is_none() {
                    in_events = Some(e);
                } else {
                    in_events = Some(in_events.unwrap() | e);
                }
            }
        }

        // If no events, we can't do anything
        if in_events.is_none() {
            bail!("No events found for {}", path);
        }

        // Try to add watch
        let add = inotify.watches().add(in_dir, in_events.unwrap());

        if let Err(e) = add {
            bail!("Unable to add watch: {}", e);
        }

        // WatcheElement creation
        let watch_descriptor = add.unwrap();

        Ok(Self {
            watch_descriptor,
            path: path.to_string(),
            command: command.to_string(),
            file_match,
            check_interval,
            mask: in_events.unwrap(),
        })
    }
}
