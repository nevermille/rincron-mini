use crate::file_check::FileCheck;
use crate::watch_element::WatchElement;
use crate::watch_manager::WatchManager;
use inotify::{Inotify, WatchDescriptor, WatchMask};
use nix::unistd::{fork, setsid, ForkResult};
use serde_json::Value;
use simple_error::bail;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

pub struct Rincron {
    inotify: Inotify,
    manager: WatchManager,
    file_checks: Vec<FileCheck>,
    sigterm: Arc<AtomicBool>,
    reload: Arc<AtomicBool>,
    watch_interval: i64,
}

impl Rincron {
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inotify: Inotify::init()?,
            manager: WatchManager::default(),
            file_checks: Vec::new(),
            sigterm: Arc::new(AtomicBool::new(false)),
            reload: Arc::new(AtomicBool::new(false)),
            watch_interval: 100,
        })
    }

    pub fn read_configs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.manager.begin_transaction();
        self.read_config("/etc/rincron-mini.json");
        self.manager.end_transaction(&mut self.inotify);

        Ok(())
    }

    pub fn read_config(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Check if config file exists
        let cfg_file = Path::new(path);

        if !cfg_file.exists() {
            println!("The config file doesn't exist!");
            bail!("The config file doesn't exist!");
        }

        // Read config file
        let cfg_string = std::fs::read_to_string(cfg_file);

        if let Err(e) = cfg_string {
            println!("Error while reading config file: {}", e);
            bail!("Error while reading config file: {}", e.to_string());
        }

        // Deserialize JSON
        let cfg_string = cfg_string.unwrap();
        let cfg_json = serde_json::from_str(&cfg_string);

        if let Err(e) = cfg_json {
            println!("Error while deserializing JSON: {}", e);
            bail!("Error while deserializing JSON: {}", e.to_string());
        }

        let cfg_json: Value = cfg_json.unwrap();

        // Read all dirs
        if !cfg_json.is_array() {
            println!("Config JSON must be an array");
            bail!("Config JSON must be an array");
        }

        let cfg_array = cfg_json.as_array().unwrap();

        for value in cfg_array {
            let we = WatchElement::from_json_value(value, &mut self.inotify);

            match we {
                Err(e) => println!("Error during parsing: {}", e),
                Ok(v) => {
                    println!("Event added for {}", &v.path);
                    self.manager.add_element(v)
                }
            }
        }
        Ok(())
    }

    pub fn execute(&mut self) {
        let mut buffer = [0; 1024];

        let hook =
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&self.sigterm));

        if hook.is_err() {
            println!("WARNING! Unable to catch SIGINT signal. Program will continue running but child processes might be killed when rincron is stopped");
        }

        let hook =
            signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&self.sigterm));

        if hook.is_err() {
            println!("WARNING! Unable to catch SIGTERM signal. Program will continue running but child processes might be killed when rincron is stopped");
        }

        let hook =
            signal_hook::flag::register(signal_hook::consts::SIGUSR1, Arc::clone(&self.reload));

        if hook.is_err() {
            println!("WARNING! Unable to catch SIGUSR1 signal. Program will continue running but you may not be able to reload");
        }

        loop {
            let events = self.inotify.read_events(&mut buffer);

            if self.sigterm.load(std::sync::atomic::Ordering::Relaxed) {
                println!("Exiting rincron, thanks for using it");
                break;
            }

            if self.reload.load(std::sync::atomic::Ordering::Relaxed) {
                println!("Reloading rincron");
                self.reload
                    .store(false, std::sync::atomic::Ordering::Relaxed);

                let load = self.read_configs();

                if let Err(e) = load {
                    println!("Error while loading config: {}", e);
                }

                continue;
            }

            if let Err(e) = events {
                if e.kind() != ErrorKind::WouldBlock {
                    println!("Error while reading events: {}", e);
                }

                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            let events = events.unwrap();

            for event in events {
                let event_config = self.manager.search_element(&event.wd);

                if event_config.is_none() {
                    continue;
                }

                let element = event_config.unwrap();
                let file = event.name.unwrap_or_else(|| OsStr::new(""));
                let escaped_path = shell_escape::escape((&element.path).into());
                let escaped_file = shell_escape::escape(file.to_string_lossy());

                let converted = element
                    .command
                    .replace("$@", &escaped_path)
                    .replace("$#", &escaped_file)
                    .replace("$$", "$");

                println!("CMD({}) => {}", element.path, &converted);

                let cmd = Command::new("bash")
                    .arg("-c")
                    .arg(&converted)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .stdin(Stdio::null())
                    .spawn();

                if let Err(e) = cmd {
                    println!("Unable to launch command: {}", e);
                }
            }
        }
    }
}
