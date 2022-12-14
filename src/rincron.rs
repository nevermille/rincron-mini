use std::{
    collections::HashMap,
    ffi::OsStr,
    io::ErrorKind,
    path::Path,
    process::{Command, Stdio},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use inotify::{Inotify, WatchDescriptor, WatchMask};

use nix::unistd::{fork, setsid, ForkResult};
use serde_json::Value;
use simple_error::bail;

pub struct Rincron {
    inotify: Inotify,
    config: HashMap<WatchDescriptor, (String, Vec<String>)>,
    sigterm: Arc<AtomicBool>,
    reload: Arc<AtomicBool>,
}

impl Rincron {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inotify: Inotify::init()?,
            config: HashMap::new(),
            sigterm: Arc::new(AtomicBool::new(false)),
            reload: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn event_name_to_value(name: &str) -> Option<WatchMask> {
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
            _ => None,
        }
    }

    pub fn read_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Check if config file exists
        let cfg_file = Path::new("/etc/rincron-mini.json");

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
            if !value.is_object() {
                println!("One item is not an object: {}", value);
                continue;
            }

            let dir = value.get("dir");
            let events = value.get("events");
            let command = value.get("command");

            if dir.is_none() || events.is_none() || command.is_none() {
                println!("One parameter is missing between \"dir\", \"events\" and \"command\"");
                continue;
            }

            let dir = dir.unwrap();
            let events = events.unwrap();
            let command = command.unwrap();

            if !dir.is_string() {
                println!("\"dir\" must be a string");
                continue;
            }

            if !events.is_array() {
                println!("\"events\" must be an array");
                continue;
            }

            if !command.is_array() {
                println!("\"command\" must be an array");
                continue;
            }

            let dir = dir.as_str().unwrap();
            let events = events.as_array().unwrap();
            let command = command.as_array().unwrap();

            let dir_path = Path::new(dir);

            if !dir_path.exists() {
                println!("\"{}\" does not exist", dir);
                continue;
            }

            let in_dir = dir;
            let mut in_events: Option<WatchMask> = None;
            let mut in_command: Vec<String> = Vec::new();

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

            if in_events.is_none() {
                println!("No event found for {}", dir);
                continue;
            }

            for parameter in command {
                if !parameter.is_string() {
                    println!("One parameter is not a string: {}", parameter);
                    continue;
                }

                in_command.push(parameter.as_str().unwrap().to_string());
            }

            let add = self.inotify.add_watch(in_dir, in_events.unwrap());

            if let Err(e) = add {
                println!("Unable to add watch: {}", e);
                continue;
            }

            let watch = add.unwrap();
            self.config.insert(watch, (in_dir.to_string(), in_command));

            println!("Event added for {}", in_dir);
        }

        Ok(())
    }

    pub fn truncate(&mut self) {
        for (wd, (path, _)) in &self.config {
            let remove = self.inotify.rm_watch(wd.clone());

            if let Err(e) = remove {
                println!("Unable to remove watch for {}: {}", path, e);
            }
        }

        self.config = HashMap::new();
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
                println!("Exiting rincron");
                break;
            }

            if self.reload.load(std::sync::atomic::Ordering::Relaxed) {
                println!("Reloading rincron");
                self.reload
                    .store(false, std::sync::atomic::Ordering::Relaxed);

                self.truncate();
                let load = self.read_config();

                if let Err(e) = load {
                    println!("Error while loading config: {}", e);
                }

                continue;
            }

            if let Err(e) = events {
                if e.kind() != ErrorKind::WouldBlock {
                    println!("Error while reading events: {}", e);
                }

                continue;
            }

            let events = events.unwrap();

            for event in events {
                let event_config = self.config.get(&event.wd);

                if event_config.is_none() {
                    continue;
                }

                let (path, command) = event_config.unwrap();
                let file = event.name.unwrap_or_else(|| OsStr::new(""));

                let mut final_command: Vec<String> = Vec::new();
                let escaped_path = shell_escape::escape(path.into());
                let escaped_file = shell_escape::escape(file.to_string_lossy());

                for param in command {
                    let converted = param
                        .replace("$@", &escaped_path)
                        .replace("$#", &escaped_file)
                        .replace("$$", "$");

                    final_command.push(converted);
                }

                let fork = unsafe { fork() };

                if let Err(e) = fork {
                    println!("Error while forking process: {}", e);
                    continue;
                }

                let fork = fork.unwrap();

                if let ForkResult::Child = fork {
                    let _ = setsid();

                    println!("CMD({}) => {}", path, final_command.join(" "));

                    let cmd = Command::new("bash")
                        .arg("-c")
                        .arg(final_command.join(" "))
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .stdin(Stdio::null())
                        .output();

                    if let Err(e) = cmd {
                        println!("Unable to launch command: {}", e);
                    }

                    std::process::exit(0);
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Default for Rincron {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
