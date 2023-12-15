pub mod pattern;
use std::cell::RefCell;
use std::io::{Write, Error, ErrorKind};
use std::fmt;
use std::path::Path;

use crate::pattern::Pattern;

type RefVec<T> = RefCell<Vec<T>>;

#[derive(Debug)]
pub enum DobfError {
    FileParseError,
}

impl fmt::Display for DobfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DobfError::FileParseError => write!(f, "File parse error"),
        }
    }
}

impl std::error::Error for DobfError { }
pub struct DobfInstance {
    path: Box<Path>,
    contents: RefVec<u8>,
    transforms: RefVec<Transform>,
}

impl DobfInstance {
    pub fn new (p: &str) -> Result<DobfInstance, Box<dyn std::error::Error>> {
        let contents = std::fs::read(p)?;

        Ok(DobfInstance {
            path: Box::from(Path::new(p)),
            contents: RefVec::from(contents),
            transforms: RefVec::from(Vec::new()),
        })
    }

    pub fn load_config(&self, mut config: DobfConfig) -> &DobfInstance {
        log::debug!("Parsing config: {}", config.name);
        self.transforms.borrow_mut().append(config.transforms.as_mut());
        self
    }

    pub fn add_transform(&self, t: Transform) -> &DobfInstance {
        self.transforms.borrow_mut().push(t);
        self
    }

    pub fn run(&self) -> Result<(), DobfError> {
        if self.contents.borrow().len() < 1 {
            return Err(DobfError::FileParseError);
        }

        self.transforms.borrow().iter().for_each(|t| {
            log::info!("Running transform: {} ({})", t.name, t.patch.len());
            t.patch(&self.contents);
        });

        Ok(())
    }

    pub fn save(&self, out: Option<String>) -> Result<(), std::io::Error> {
        if self.contents.borrow().len() < 1 {
            return Err(Error::new(ErrorKind::Other, "No data supplied"));
        }

        if let Some(out) = out {
            let out_path = Path::new(&out);
            if out_path.exists() {
                log::warn!("Output file already exists, overwriting");
            }
            log::info!("Saving to: {}", out_path.to_str().unwrap());
            std::fs::File::create(out_path)?.write_all(&self.contents.borrow())?;
            return Ok(());
        }

        let out_path = self.path.as_ref().to_owned()
            .with_file_name(format!("{}_patched", self.path.file_name().unwrap().to_str().unwrap()));

        log::info!("Saving to: {}", out_path.to_str().unwrap());

        std::fs::File::create(out_path)?.write_all(&self.contents.borrow())?;

        Ok(())
    }
}

pub struct Transform {
    name: String,
    pattern: Pattern,
    patch: Pattern,
    order: usize,
}

impl Transform {
    pub fn new(name: &str, p: &str, patch: &str, o: usize) -> Option<Transform> {
        Some(Transform {
            name: String::from(name),
            pattern: Pattern::builder(p).build()?,
            patch: Pattern::builder(patch).simplify_nops().build()?,
            order: o,
        })
    }

    pub fn patch(&self, data: &RefVec<u8>) {
        let matches = self.pattern.matches_all(&data.borrow());
        log::info!("{:4} - Found {} matches", "", matches.len());
        for &i in matches.iter() {
            // TODO: allow it to use the .is_wildcard() on the patch pattern instead of the search pattern
            let final_patch = self.patch.iter().enumerate().map(|(i, b)| if self.patch.is_wildcard(i) { data.borrow()[i] } else { *b }).collect::<Vec<u8>>();

            data.borrow_mut().splice(i..i+final_patch.len(), final_patch);
        }
    }
}

pub struct DobfConfig {
    pub name: String,
    pub transforms: Vec<Transform>,
}

impl DobfConfig {
    // TODO: fix this mess
    pub fn new(path: &str) -> Result<DobfConfig, Box<dyn std::error::Error>> {
        let mut config = DobfConfig {
            name: String::from(""),
            transforms: Vec::new(),
        };

        let contents = std::fs::read_to_string(path)?;
        let parsed = contents.parse::<toml::Value>()?;

        let table = parsed.as_table().unwrap();

        config.name = table.get("name").unwrap().as_str().unwrap().to_owned();

        let transforms = table.keys().filter(|f| table.get(*f).unwrap().as_table().is_some());
        
        for transform in transforms {
            let transform_table = table.get(transform).unwrap().as_table().unwrap();
            let pattern = transform_table.get("pattern").unwrap().as_str().unwrap();
            let patch = transform_table.get("patch").unwrap().as_str().unwrap();
            let order = transform_table.get("order").expect("Did you forget order?").as_integer().unwrap_or_default() as usize; // had to do this because toml-rs doesnt read in the order its written, alphabetically sorted i think
            config.transforms.push(Transform::new(transform, pattern, patch, order).unwrap()); // double slice??
        }

        config.transforms.sort_by(|a, b| a.order.cmp(&b.order));

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_new() {
        let t = Transform::new("test", "48 80 80 8B", "48 90 90 8B", 0).unwrap();
        let v: RefVec<u8> = RefVec::new(vec![0x48, 0x80, 0x80, 0x8B]);
        t.patch(&v);
        assert_eq!(v.borrow().to_vec(), [0x48, 0x66, 0x90, 0x8B]);
    }

    #[test]
    fn test_wildcard_transform() {
        let t = Transform::new("wildcard-test", "E8 05 14 23 55", "E8 ? ? ? 90", 0).unwrap();
        let v: RefVec<u8> = RefVec::new(vec![0xE8, 0x05, 0x14, 0x23, 0x55]);
        t.patch(&v);
        assert_eq!(v.borrow().to_vec(), [0xE8, 0x05, 0x14, 0x23, 0x90]);
    }
}
