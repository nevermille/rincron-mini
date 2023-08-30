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

use crate::file_check::FileCheck;
use crate::watch_element::WatchElement;
use crate::watch_manager::WatchManager;
use glob::glob;
use inotify::Inotify;
use serde_json::Value;
use simple_error::bail;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use wildmatch::WildMatch;

/// The main program
pub struct Rincron {
    /// The inotify object
    inotify: Inotify,

    /// The events manager
    manager: WatchManager,

    /// The files to check
    file_checks: Vec<FileCheck>,

    /// The files to execute
    file_executions: Vec<FileCheck>,

    /// The sigterm signal
    sigterm: Arc<AtomicBool>,

    /// The sigusr1 signal
    reload: Arc<AtomicBool>,

    /// The delay between event watches in milliseconds
    watch_interval: u64,

    /// The spawned children
    child_processes: Vec<Child>,

    /// The config root
    config_root: String,
}

impl Rincron {
    /// Initiolizes ricron with inotify
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inotify: Inotify::init()?,
            manager: WatchManager::default(),
            file_checks: Vec::new(),
            file_executions: Vec::new(),
            sigterm: Arc::new(AtomicBool::new(false)),
            reload: Arc::new(AtomicBool::new(false)),
            watch_interval: 100,
            child_processes: Vec::new(),
            config_root: Self::get_config_root(),
        })
    }

    /// Returns the config directory for the current user
    fn get_config_root() -> String {
        let home_path = dirs::home_dir();

        if home_path.is_none() {
            return "/etc".to_string();
        }
        let home_pathbuf = home_path.unwrap();
        let home_path = home_pathbuf.to_string_lossy();

        if home_path == "/root" {
            return "/etc".to_string();
        }

        format!("{}/.config", home_path)
    }

    /// Reads all config files
    ///
    /// Config files are found in /etc/rincron-mini directory
    /// If you don't want a folder, you can use /etc/rincron-mini.json
    pub fn read_configs(&mut self) {
        let config_file = format!("{}/rincron-mini.json", &self.config_root);
        let config_dir_pattern = format!("{}/rincron-mini/*.json", &self.config_root);

        self.manager.begin_transaction();

        println!("Checking config file {}", &config_file);

        // First we check the main config file
        if Path::new(&config_file).exists() {
            if let Err(e) = self.read_config(&config_file) {
                println!("Error while reading config file {}: {}", &config_file, e);
            }
        }

        println!("Scanning config files {}", &config_dir_pattern);

        // After that, we check the folder for more config files
        let files = glob(&config_dir_pattern);

        // It's horrible but I don't know how to properly write this (yet)
        if let Ok(v) = files {
            // We process each entry found in glob scanning
            for entry in v {
                // I don't know why but you can have sub errors
                match entry {
                    // Finally, a found config file
                    Ok(p) => {
                        println!("Config file found: {}", p.display());
                        if let Err(e) = self.read_config(&p.to_string_lossy()) {
                            println!("Error while reading config file {}: {}", p.display(), e);
                        }
                    }
                    // I don't know how this error is triggered
                    Err(e) => {
                        println!("Error while scanning config files: {}", e);
                    }
                }
            }
        }

        self.manager.end_transaction(&mut self.inotify);
    }

    /// Reads a config file
    ///
    /// # Parameters
    ///
    /// * `path`: The config file path
    pub fn read_config(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Check if config file exists
        let cfg_file = Path::new(path);

        if !cfg_file.exists() {
            bail!("The config file {} doesn't exist", path);
        }

        // Read config file
        let cfg_string = std::fs::read_to_string(cfg_file);

        if let Err(e) = cfg_string {
            bail!("Error while reading config file: {}", e.to_string());
        }

        // Deserialize JSON
        let cfg_string = cfg_string.unwrap();
        let cfg_json = serde_json::from_str(&cfg_string);

        if let Err(e) = cfg_json {
            bail!("Error while deserializing JSON: {}", e.to_string());
        }

        let cfg_json: Value = cfg_json.unwrap();

        // Read all dirs
        if !cfg_json.is_array() {
            bail!("Config JSON must be an array");
        }

        let cfg_array = cfg_json.as_array().unwrap();

        for value in cfg_array {
            let we = WatchElement::from_json_value(value, &mut self.inotify);

            match we {
                Err(e) => println!("Error during parsing: {}", e),
                Ok(v) => self.manager.add_element(v),
            }
        }
        Ok(())
    }

    /// Hook vars to system signals
    pub fn hook_signals(&mut self) {
        // SIGINT managment
        let hook =
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&self.sigterm));
        if hook.is_err() {
            println!("WARNING! Unable to catch SIGINT signal. Program will continue running but might not exit properly");
        }

        // SIGTERM managment
        let hook =
            signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&self.sigterm));
        if hook.is_err() {
            println!("WARNING! Unable to catch SIGTERM signal. Program will continue running but might not exit properly");
        }

        // SIGTERM managment
        let hook =
            signal_hook::flag::register(signal_hook::consts::SIGUSR1, Arc::clone(&self.reload));
        if hook.is_err() {
            println!("WARNING! Unable to catch SIGUSR1 signal. Program will continue running but you may not be able to reload configs");
        }
    }

    /// Check if children have exited
    pub fn watch_children(&mut self) {
        // We watch spawned childs to report exit status
        let mut finished_children = Vec::new();
        for (index, child) in self.child_processes.iter_mut().enumerate() {
            match child.try_wait() {
                Err(e) => {
                    println!("Error while checking child {}: {}", child.id(), e);
                    finished_children.push(index);
                }
                Ok(Some(v)) => {
                    println!("Child {} exited with {}", child.id(), v);
                    finished_children.push(index);
                }
                _ => { /* Not exited*/ }
            }
        }

        // We need indexes in reverse order to not remove wrong children
        finished_children.sort();
        finished_children.reverse();

        // Time to remove finished children, now that the var is free from borrows
        for i in finished_children {
            self.child_processes.remove(i);
        }
    }

    /// Read all events from inotify
    ///
    /// # Parameters
    ///
    /// * `buffer`: A buffer to write events
    pub fn watch_events(&mut self, buffer: &mut [u8]) {
        // Read inotify events buffer
        let events = self.inotify.read_events(buffer);

        if let Err(e) = events {
            // We need to notify for any error not related to an empty buffer
            if e.kind() != ErrorKind::WouldBlock {
                println!("Error while reading events: {}", e);
            }

            std::thread::sleep(Duration::from_millis(self.watch_interval));
            return;
        }
        let events = events.unwrap();

        // Events management
        for event in events {
            // We need more info for this descriptor
            let event_config = self.manager.search_element(&event.wd);

            // We do nothing if element not found
            if event_config.is_none() {
                continue;
            }

            let element = event_config.unwrap();
            let file = event.name.unwrap_or_else(|| OsStr::new(""));
            let escaped_path = shell_escape::escape((&element.path).into());
            let escaped_file = shell_escape::escape(file.to_string_lossy());
            let full_path = Path::new(&escaped_path.to_string()).join(&escaped_file.to_string());

            println!("Event found for {} ({})", &escaped_path, &escaped_file);

            // If the file does not match the desired string, we don't do anything
            if !element.file_match.is_empty()
                && !WildMatch::new(&element.file_match).matches(&escaped_file)
            {
                println!(
                    "File {} does not match {}, event discarded",
                    &escaped_file, &element.file_match
                );
                continue;
            }

            // Command line creation
            let converted_cmd = element
                .command
                .replace("$@", &escaped_path)
                .replace("$#", &escaped_file)
                .replace("$$", "$");

            // File information creation
            let fc = FileCheck::new(
                &full_path.to_string_lossy(),
                element.check_interval * 1000,
                &converted_cmd,
            );

            // If a size check is needed, we put it in file checks instead of file executions
            if element.check_interval == 0 {
                self.file_executions.push(fc);
            } else {
                self.file_checks.push(fc);
            }
        }
    }

    /// Substract elapsed time for all files checkers
    pub fn file_watch_tick(&mut self) {
        for file in &mut self.file_checks {
            file.tick(self.watch_interval as i64);
        }
    }

    /// Watch all file sizes
    pub fn file_watch(&mut self) {
        let mut finished_files = Vec::new();

        for (index, file) in &mut self.file_checks.iter_mut().enumerate() {
            // If file did not change, the upload/copy is considered finished
            if !file.has_changed() {
                println!("File {} is now ready for execution", &file.path);
                self.file_executions.push(file.clone());
                finished_files.push(index);
            }
        }

        // We delete finished file checks
        finished_files.sort();
        finished_files.reverse();

        for i in finished_files {
            self.file_checks.remove(i);
        }
    }

    /// Executes files
    pub fn file_execute(&mut self) {
        for file in &self.file_executions {
            println!("CMD({}) => {}", &file.path, &file.cmd);

            let cmd = Command::new("bash")
                .arg("-c")
                .arg(&file.cmd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .spawn();

            match cmd {
                Err(e) => {
                    println!("Unable to launch command: {}", e);
                }
                Ok(v) => {
                    println!("Child {} spawned", v.id());
                    self.child_processes.push(v);
                }
            };
        }

        self.file_executions = Vec::new();
    }

    /// Executes the main loop
    pub fn execute(&mut self) {
        let mut buffer = [0; 1024];

        self.read_configs();
        self.hook_signals();

        loop {
            // Exit requested
            if self.sigterm.load(std::sync::atomic::Ordering::Relaxed) {
                println!("Exiting rincron, thanks for using it");
                break;
            }

            // Reload requested
            if self.reload.load(std::sync::atomic::Ordering::Relaxed) {
                println!("Reloading rincron");
                self.reload
                    .store(false, std::sync::atomic::Ordering::Relaxed);

                self.read_configs();
                continue;
            }

            // Main program
            self.watch_children();
            self.file_watch_tick();
            self.watch_events(&mut buffer);
            self.file_watch();
            self.file_execute();
        }
    }
}
