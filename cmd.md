# Cargo Mate (cm) Command Reference

## Basic Commands

- `cm --help`  
  Show help information.

- `cm --version`  
  Show version information.

---

## Journey

**Usage:**  
`cm journey <COMMAND>`

| Command     | Description                           |
|-------------|---------------------------------------|
| *(no args)* | Show journey overview                 |
| record      | Record a new journey entry            |
| play        | Replay a recorded journey             |
| list        | List all recorded journeys            |
| export      | Export journeys to external format    |
| import      | Import journeys from external format  |
| publish     | Publish a journey for sharing         |
| download    | Download a published journey          |
| search      | Search through journey entries        |
| published   | List published journeys               |
| help        | Show help for journey commands        |

---

## Anchor

**Usage:**  
`cm anchor <COMMAND>`

| Command     | Description                                 |
|-------------|---------------------------------------------|
| *(no args)* | Show anchor overview                        |
| save        | Save current project state as anchor        |
| restore     | Restore project to saved anchor state       |
| list        | List all saved anchors                      |
| show        | Show details of a specific anchor           |
| diff        | Compare current state with anchor           |
| auto        | Auto-save anchor for project                |
| help        | Show help for anchor commands               |

---

## Log

**Usage:**  
`cm log <COMMAND>`

| Command     | Description                                 |
|-------------|---------------------------------------------|
| *(no args)* | Show log overview                           |
| add         | Add a new log entry                         |
| search      | Search through log entries                  |
| timeline    | Display log entries in timeline view        |
| export      | Export logs to external format              |
| analyze     | Analyze log patterns and statistics         |
| help        | Show help for log commands                  |

---

## Tide

**Usage:**  
`cm tide <COMMAND>`

| Command     | Description                                 |
|-------------|---------------------------------------------|
| *(no args)* | Show tide overview                          |
| show        | Display tide information                    |
| analyze     | Analyze tide patterns                       |
| export      | Export tide data                            |
| help        | Show help for tide commands                 |

---

## Map

**Usage:**  
`cm map <COMMAND>`

| Command     | Description                                 |
|-------------|---------------------------------------------|
| *(no args)* | Show map overview                           |
| show        | Display project dependency map              |
| analyze     | Analyze project structure                   |
| export      | Export map data                             |
| path        | Show dependency paths                       |
| help        | Show help for map commands                  |

---

## Mutiny

**Usage:**  
`cm mutiny <COMMAND>`

| Command        | Description                                 |
|----------------|---------------------------------------------|
| *(no args)*    | Show mutiny overview                        |
| activate       | Activate mutiny mode                        |
| deactivate     | Deactivate mutiny mode                      |
| allow-warnings | Allow warnings in mutiny mode               |
| skip-tests     | Skip tests in mutiny mode                   |
| force          | Force operations in mutiny mode             |
| yolo           | Enable YOLO mode (maximum risk)             |
| status         | Show current mutiny status                  |
| help           | Show help for mutiny commands               |

---

## Config

**Usage:**  
`cm config <COMMAND>`

| Command   | Description                                 |
|-----------|---------------------------------------------|
| *(no args)* | Show config overview                      |
| set       | Set a config value                          |
| get       | Get a config value                          |
| list      | List all config                             |
| init      | Initialize local config                     |
| shortcut  | Add a shortcut                              |
| hook      | Add a hook                                  |
| help      | Show help for config commands               |

---

## Version

**Usage:**  
`cm version <COMMAND>`

| Command         | Description                                 |
|-----------------|---------------------------------------------|
| *(no args)*     | Show version overview                       |
| init            | Initialize version management               |
| info            | Show version information                    |
| increment       | Increment version number                    |
| set             | Set specific version                        |
| history         | Show version history                        |
| update-cargo    | Update Cargo.toml version                   |
| config          | Manage version configuration                |
| config enable   | Enable version feature                      |
| config disable  | Disable version feature                     |
| config policy   | Set version policy                          |
| config show     | Show version configuration                  |
| help            | Show help for version commands              |

---

## View

**Usage:**  
`cm view <COMMAND>`

| Command     | Description                                 |
|-------------|---------------------------------------------|
| *(no args)* | Show view overview                          |
| errors      | View all errors and warnings                |
| artifacts   | View generated files and locations          |
| scripts     | View build script outputs                   |
| history     | View detailed build history                 |
| checklist   | View checklist and fixes                    |
| all         | View all results in one place               |
| latest      | Quick view of latest issues                 |
| open        | Open results in file explorer               |
| help        | Show help for view commands                 |

---

## Optimize

**Usage:**  
`cm optimize <COMMAND>`

| Command         | Description                                             |
|-----------------|--------------------------------------------------------|
| *(no args)*     | Show optimization overview                             |
| aggressive      | Apply aggressive optimizations for maximum speed        |
| balanced        | Apply balanced optimizations for good speed/stability   |
| conservative    | Apply conservative optimizations for maximum stability  |
| custom          | Apply custom optimizations with specific values         |
| status          | Show current optimization status                        |
| recommendations | Show optimization recommendations                      |
| restore         | Restore original Cargo.toml from backup                 |
| help            | Show help for optimize commands                         |

---

## Top-Level Commands

**Usage:**  
`cm <COMMAND>`

| Command    | Description                                              |
|------------|----------------------------------------------------------|
| init       | Initialize a new cargo-mate project                      |
| install    | Install cargo-mate system-wide                           |
| activate   | Activate cargo-mate for current project                  |
| checklist  | Show project checklist                                   |
| history    | Show command history                                     |
| exec       | Execute cargo command (fallback for standard cargo cmds) |
| help       | Show help for top-level commands                         |