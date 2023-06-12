# Rincron-Mini

Rincron-mini is a software written in Rust who aims to be a replacement for incrontab. This software is compatible with Linux **and only Linux** since inotify doesn't exist on other platforms.

## Installation

Use cargo to install rincron-mini: 

```
cargo install rincron_mini
```

## Configuration

Rincron-Mini uses JSON files as configuration files. You can put them inside `/etc/rincron-mini/` with a .json extension or you can put one at `/etc/rincron-mini.json`

### File format

Each JSON file must contain an array of objects. This is the minimal format:

```json
[
    {
        "path": "/tmp",
        "events": [
            "CREATE",
            "MOVED_TO"
        ],
        "command": "echo \"Event on $#/$@\""
    }, 
    {
        "path": "/dev/null",
        "events": ["IN_ACCESS"],
        "command": "echo \"Event on $#/$@\""
    }
]
```

* `path`: Can be a file or a directory, this is what will be watched
* `events`: One or more inotify events, you can strip the `IN_` from event name
* `command`: A command to execute

### The command parameter

If you want to contextualize the command line, you can use a few wildcards:

* `$@`: The watched file/directory (copies the `path` parameter)
* `$#`: The file or directory name where the event was triggered
* `$$`: A `$` character

### File size watch

When you use rincron-mini for executing commands on moved, copied or uploaded files, you may want to execute the command only if the copy/move/upload is finished. In this case, you can add a `check_interval` parameter with an integer representing the time (in seconds) between two size checks. Once the file size hasn't changed between two checks, the command will be executed

In this example, the file will be checked every 5 seconds:

```json
[
    {
        "path": "/tmp",
        "events": [
            "CREATE",
            "MOVED_TO"
        ],
        "command": "echo \"Event on $#/$@\"",
        "check_interval": 5
    }
]
```

### File name match

Sometimes, you want to execute a command only on one file type. You can do this with the `file_match` command. You can use the `?` and `*` wildcards.

Example with a check on zip files:

```json
[
    {
        "path": "/tmp",
        "events": [
            "CREATE",
            "MOVED_TO"
        ],
        "command": "echo \"Event on $#/$@\"",
        "file_match": "*.zip"
    }
]
```

## Limitations

This sofware is unfortunately not a full incrontab replacement. There are some limitations:

* The `$%` and `$&` are not implemented
* Only the user who has executed the program (probably root) will execute commands, there is no configs for users

I'll try to improve the software, to make it more powerfull.
