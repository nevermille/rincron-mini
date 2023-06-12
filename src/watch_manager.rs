use crate::watch_element::WatchElement;
use inotify::{Inotify, WatchDescriptor};
use std::collections::HashMap;

#[derive(Default)]
pub struct WatchManager {
    current_elements: HashMap<WatchDescriptor, WatchElement>,
    previous_elements: HashMap<WatchDescriptor, WatchElement>,
    new_elements: Vec<WatchElement>,
}

impl WatchManager {
    pub fn begin_transaction(&mut self) {
        self.previous_elements = self.current_elements.clone();
        self.current_elements = HashMap::new();
        self.new_elements = Vec::new();
    }

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
        self.new_elements.push(new_element);
    }

    pub fn end_transaction(&mut self, inotify: &mut Inotify) {
        // We remove unecessary elements
        // This needs to be done before adding new element to avoid conflicts
        for (descriptor, _) in &self.previous_elements {
            if let Err(e) = inotify.rm_watch(descriptor.clone()) {
                println!("Warning: error while removing inotify watch: {}", e);
            }
        }

        // We add newly added elements
        for element in &self.new_elements {
            let wd = inotify.add_watch(element.path.clone(), element.mask.clone());

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

    pub fn search_element(&mut self, watch_descriptor: &WatchDescriptor) -> Option<&WatchElement> {
        self.current_elements.get(watch_descriptor)
    }
}
