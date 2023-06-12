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

use crate::watch_element::WatchElement;
use inotify::{Inotify, WatchDescriptor};
use std::collections::HashMap;

#[derive(Default)]
/// Manager of events
pub struct WatchManager {
    /// Elements currently watched
    current_elements: HashMap<WatchDescriptor, WatchElement>,

    /// Backup of elements from before transaction start
    previous_elements: HashMap<WatchDescriptor, WatchElement>,

    /// New elements to add after transaction end
    new_elements: Vec<WatchElement>,
}

impl WatchManager {
    /// Starts a new transaction, all events will be backed up
    pub fn begin_transaction(&mut self) {
        self.previous_elements = self.current_elements.clone();
        self.current_elements = HashMap::new();
        self.new_elements = Vec::new();
    }

    /// Adds a new elements, if a similar element exists in the backup, it will be moved to avoid
    /// losses. If not, it will be added to inotify at transaction end
    ///
    /// # Parameters
    ///
    /// * `new_element`: The new element to add
    pub fn add_element(&mut self, new_element: WatchElement) {
        let mut exists = false;
        let mut previous_descriptor = None;
        let mut previous_element = None;

        // We check previous elements if it already exists
        for (descriptor, element) in &self.previous_elements {
            if new_element == *element {
                println!("Already existing element: {}", &element.path);
                exists = true;
                previous_descriptor = Some(descriptor.clone());
                previous_element = Some(element.clone());
            }
        }

        // If it already exists, we just move it to current elements
        if exists {
            self.previous_elements
                .remove(previous_descriptor.as_ref().unwrap());
            self.current_elements
                .insert(previous_descriptor.unwrap(), previous_element.unwrap());
            return;
        }

        // If it does not exist, we put it in new elements
        println!("Event added for {}", &new_element.path);
        self.new_elements.push(new_element);
    }

    /// Ends the transaction, all non-moved elements will be removed from inotify and new ones
    /// will be added
    ///
    /// # Parameters
    ///
    /// * `inotify`: The inotify object where to add events
    pub fn end_transaction(&mut self, inotify: &mut Inotify) {
        // We remove unecessary elements
        // This needs to be done before adding new element to avoid conflicts
        for (descriptor, element) in &self.previous_elements {
            match inotify.rm_watch(descriptor.clone()) {
                Err(e) => {
                    println!("Warning: error while removing inotify watch: {}", e);
                }
                Ok(_) => {
                    println!("Event removed for {}", &element.path);
                }
            };
        }

        // We add newly added elements
        for element in &self.new_elements {
            let wd = inotify.add_watch(element.path.clone(), element.mask);

            match wd {
                Err(e) => {
                    println!("Warning: error while adding inotify watch: {}", e);
                }
                Ok(v) => {
                    self.current_elements.insert(v, element.clone());
                }
            };
        }
    }

    /// Searches an element in the database
    ///
    /// # Parameters
    ///
    /// * `watch_descriptor`: The associated watch descriptor
    pub fn search_element(&mut self, watch_descriptor: &WatchDescriptor) -> Option<&WatchElement> {
        self.current_elements.get(watch_descriptor)
    }
}
