//! Define our `Config` and `Worker`.
//! Implement how to take the `config.toml` and turn it into a `Config { files: Vec<ManagedObject> }`.
//! Describes the `Worker` object, which is how we communicate back to our `main()` thread
//! about how our `task` is going.
extern crate serde;
extern crate toml;

use crate::hmerror::HMError;

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::{
  fmt, fs, io,
  io::{prelude::*, BufReader},
  path::{Path, PathBuf},
  string::String,
};
use toml::value;

///
/// Allow us to communicate meaningfully back to `main()` thread.
///
#[derive(Debug, Clone, Hash)]
pub struct Worker {
  pub name: String,
  pub status: Option<i32>,
  pub completed: bool,
}

impl<'a> Worker {
  // i'll get to you
  #[allow(dead_code)]
  pub fn new() -> Worker {
    Worker {
      name: String::from(""),
      status: Some(1),
      completed: false,
    }
  }
}
impl PartialEq for Worker {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}
impl Eq for Worker {}

#[derive(Debug)]
pub enum OS {
  Windows,
  Unknown,
  Linux(LinuxDistro),
}

#[derive(Debug)]
pub enum LinuxDistro {
  Fedora,
  Debian,
  Ubuntu,
  Arch,
}

///
/// Windows or Linux? If Linux, let's determine our distro, because package managers and stuff.
///
pub fn determine_os() -> OS {
  match sys_info::os_type() {
    Ok(s) => match s.to_ascii_lowercase().as_str() {
      "linux" => match sys_info::linux_os_release() {
        Ok(l) => {
          let a: String = l.name.unwrap().to_ascii_lowercase();
          if a.contains("fedora") {
            OS::Linux(LinuxDistro::Fedora)
          } else if a.contains("debian") {
            OS::Linux(LinuxDistro::Debian)
          } else if a.contains("ubuntu") {
            OS::Linux(LinuxDistro::Ubuntu)
          } else {
            OS::Unknown
          }
        }
        Err(_e) => OS::Unknown,
      },
      "windows" => OS::Windows,
      _ => OS::Unknown,
    },
    Err(_e) => OS::Unknown,
  }
}

/// We're a super-set of all the kinds of `ManagedObject`s we can be.
/// Just don't use the fields you don't wanna use.
/// A simple `ManagedObject` is a name, source, destination, and method (currently only symlink).
/// The simple `ManagedObject` would just be symlinked to its destination.
/// Complex `ManagedObject`s include solutions, which are executed scripts.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManagedObject {
  pub name: String,
  pub source: String,
  pub file: String,
  pub destination: String,
  pub method: String,
  pub task: String,
  pub solution: String,
  pub dependencies: Vec<String>,
  pub satisfied: bool,
}

impl ManagedObject {
  /// quite simply, if we're a task, we'll have a `solution`.
  pub fn is_task(&self) -> bool {
    return !self.solution.is_empty();
  }
  pub fn set_satisfied(&mut self) -> () {
    self.satisfied = true;
  }
}

//impl Drop for ManagedObject {
//  fn drop(&mut self) {
//    eprintln!("dropping {}", self.name);
//  }
//}

impl fmt::Display for ManagedObject {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "{} {} {} {} {} {} {} {}",
      self.name,
      self.file,
      self.source,
      self.method,
      self.destination,
      self.task,
      self.solution,
      self.satisfied
    )
  }
}

impl Default for ManagedObject {
  fn default() -> Self {
    ManagedObject {
      name: String::from(""),
      source: String::from(""),
      file: String::from(""),
      destination: String::from(""),
      method: String::from(""),
      task: String::from(""),
      solution: String::from(""),
      dependencies: Vec::new(),
      satisfied: false,
    }
  }
}

/// Represents just the file `config.toml` and contains a vector of things
/// that shall become `ManagedObject`s.
#[derive(Deserialize, Clone)]
pub struct Config {
  #[serde(rename = "obj", deserialize_with = "deserialize_files")]
  pub files: Vec<(String, value::Value)>,
}

impl Default for Config {
  fn default() -> Self {
    Config { files: Vec::new() }
  }
}

/*
  this is all such terrible rust please don't look at it
*/
/// pls do not view src for this, am not proud
impl fmt::Display for Config {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut mos: Vec<ManagedObject> = Vec::new();
    for _f in self.files.iter() {
      let mut mo = ManagedObject::default();
      mo.name = _f.0.to_owned();
      match _f.1.get("file") {
        None => (),
        Some(_x) => {
          mo.file = String::from(_x.as_str().unwrap());
        }
      }
      match _f.1.get("source") {
        None => (),
        Some(_x) => {
          mo.source = String::from(_x.as_str().unwrap());
        }
      }
      match _f.1.get("method") {
        None => (),
        Some(_x) => {
          mo.method = String::from(_x.as_str().unwrap());
        }
      }
      match _f.1.get("destination") {
        None => (),
        Some(_x) => {
          mo.destination = String::from(_x.as_str().unwrap());
        }
      }
      match _f.1.get("solution") {
        None => (),
        Some(_x) => {
          mo.solution = String::from(_x.as_str().unwrap());
        }
      }
      match _f.1.get("task") {
        None => (),
        Some(_x) => {
          mo.task = String::from(_x.as_str().unwrap());
        }
      }
      match _f.1.get("dependencies") {
        None => (),
        Some(_x) => {
          let _f = _x.as_str().unwrap();
          // thanks https://stackoverflow.com/a/37547426
          mo.dependencies = _f.split(", ").map(|s| s.to_string()).collect();
        }
      }
      mos.push(mo);
    }
    write!(f, "{:#?}", mos)
  }
}

#[allow(dead_code)]
pub fn get_mo(_n: String) -> Result<ManagedObject, HMError> {
  unimplemented!("not done")
}

/// This takes our file/task array and turns them into `ManagedObjects`,
/// to be stuffed into the `Config`.
pub fn deserialize_files<'de, D>(deserializer: D) -> Result<Vec<(String, value::Value)>, D::Error>
where
  D: Deserializer<'de>,
{
  let mut files: Vec<(String, value::Value)> = Vec::new();
  let raw_files: Vec<value::Table> = Deserialize::deserialize(deserializer)?;
  for mut entry in raw_files {
    if let Some(name) = entry.remove("file") {
      if let Some(name) = name.as_str() {
        files.push((name.to_owned(), value::Value::Table(entry)));
      }
    } else if let Some(name) = entry.remove("task") {
      if let Some(name) = name.as_str() {
        files.push((name.to_owned(), value::Value::Table(entry)));
      }
    }
  }
  Ok(files)
}

/// Convenience function that allows getting a HashMap from a `Config` of
/// the `ManagedObject`s within.
pub fn as_managed_objects(config: Config) -> HashMap<String, ManagedObject> {
  let mut mos: HashMap<String, ManagedObject> = HashMap::new();
  for _f in config.files.iter() {
    let mut mo = ManagedObject::default();
    mo.name = _f.0.to_owned();
    match _f.1.get("solution") {
      None => (),
      Some(_x) => {
        mo.solution = String::from(_x.as_str().unwrap());
      }
    }
    match _f.1.get("task") {
      None => (),
      Some(_x) => {
        mo.task = String::from(_x.as_str().unwrap());
      }
    }
    match _f.1.get("source") {
      None => (),
      Some(_x) => {
        mo.source = String::from(_x.as_str().unwrap());
      }
    }
    match _f.1.get("method") {
      None => (),
      Some(_x) => {
        mo.method = String::from(_x.as_str().unwrap());
      }
    }
    match _f.1.get("destination") {
      None => (),
      Some(_x) => {
        mo.destination = String::from(_x.as_str().unwrap());
      }
    }
    match _f.1.get("dependencies") {
      None => (),
      Some(_x) => {
        let _f = _x.as_str().unwrap();
        // thanks https://stackoverflow.com/a/37547426
        mo.dependencies = _f.split(", ").map(|s| s.to_string()).collect();
      }
    }
    mos.insert(mo.name.clone(), mo);
  }
  return mos;
}

/// Take a big, fat guess.
/// Open the specified file. We've already made sure our Path and stuff
/// look good.
fn open_config(file: &str) -> io::Result<fs::File> {
  fs::File::open(file)
}

/// Open our config file and read the entire contents into hopefully
/// valid toml. Either we gucci and return back a `Config` made of toml,
/// or we explain what went wrong with the `toml` Err.
pub fn deserialize_file(file: &str) -> Result<Config, String> {
  let mut contents = String::new();
  let g = match open_config(file) {
    Ok(_a) => _a,
    Err(e) => return Err(e.to_string()),
  };
  let mut file_contents = BufReader::new(g);
  match file_contents.read_to_string(&mut contents) {
    Ok(v) => v,
    Err(_e) => 0,
  };
  if cfg!(debug_assertions) {
    println!("file: {}", &file);
  }
  toml::from_str(&contents).or_else(|e| Err(e.to_string()))
}

/// Make sure $XDG_CONFIG_DIR exists.
/// On Linux and similar this is /home/\<username\>/.config;
/// macOS /Users/\<username\>/.config.
///
/// Assuming it does exist or we can create it, stuff config.toml on the end of it
/// and return `Ok(my_path_buf/config.toml)`.
///
/// This is safe because `ensure_config_dir()` is called only after we already know
/// the user didn't specify a `config.toml` path themselves. We must check our
/// default expected location for it.
pub fn ensure_config_dir() -> Result<PathBuf, &'static str> {
  // get /home/<username>/.config, if exists...
  match dirs::config_dir() {
    Some(p) => {
      // if something
      // creates a PathBuf from $XDG_CONFIG_DIR
      let whole_path = p.join(Path::new("homemaker"));
      match fs::create_dir_all(&whole_path) {
        /*
        then when we return it, do the entire config dir path (/home/hlmtre/.config)
        and add our config file to the end of the PathBuf
        ```
        Ok(("/home/hlmtre/.config").join("config.toml"))
        ```
        sort of as a pseudocodey example
         */
        Ok(()) => return Ok(PathBuf::from(&whole_path.join("config.toml"))),
        Err(_e) => return Err("Couldn't create config dir!"),
      }
    }
    // if dirs::config_path() call doesn't return anything
    None => return Err("Couldn't get config directory from $XDG"),
  };
}
