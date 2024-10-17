use std::{collections::HashMap, io::Read as _, path::Path};

use anyhow::Context as _;
use dioxus_logger::tracing::{info, warn};



#[cfg(feature = "server")]
#[derive(Debug)]
pub struct PartLibrary {
    part_map: HashMap<String, Box<[u8]>>,
}
#[cfg(feature = "server")]
impl PartLibrary {
    pub fn new(archive_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(archive_path).context("Failed to open library file")?;
        let reader = std::io::BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader).context("Failed to open file as archive")?;
        let mut part_map = HashMap::new();
        info!("Loading LDraw lib...");
        for i in 0.. {
            let Ok(mut file) = archive.by_index(i) else {
                break;
            };
            let name = file
                .enclosed_name()
                .unwrap()
                .to_str()
                .unwrap()
                .trim_start_matches("ldraw/")
                .trim_start_matches("parts/")
                .trim_start_matches("p/")
                .trim_start_matches("models/")
                .to_string();
            if name.ends_with(".ldr") || name.ends_with(".dat") {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                if part_map
                    .insert(name.clone(), buf.into_boxed_slice())
                    .is_some()
                {
                    warn!("Duplicate ldraw lib file `{name}`");
                }
            }
        }
        info!("Loaded LDraw lib");
        Ok(Self { part_map })
    }
    pub fn get_part(&self, name: impl AsRef<Path>) -> anyhow::Result<Box<[u8]>> {
        let path = name.as_ref().to_str().unwrap().replace("\\", "/");
        let part_file = self.part_map.get(&path.to_string()).unwrap().clone();
        Ok(part_file)
    }
}
