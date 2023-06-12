use std::path::Path;

#[derive(Clone)]
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
            self.size = 0;
        }

        // Same with metadata reading
        let metadata = std::fs::metadata(&self.path);
        if let Err(e) = &metadata {
            print!("Warning: error while reading file metadata: {}", e);
            self.size = 0;
        }

        let new_size = metadata.unwrap().len();

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
