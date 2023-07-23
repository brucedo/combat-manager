use std::{path::PathBuf, ffi::OsString, collections::HashMap};

use toml::Table;


pub struct Statics
{
    static_root: PathBuf,
    cache: HashMap<String, StaticEntry>, 

}

pub struct StaticEntry
{
    mime_type: String,
    contents: Option<Vec<u8>>,
    filename: OsString,
}

impl Statics
{
    pub fn with_root(static_root: PathBuf) -> Result<Statics, Error>
    {
        if !(static_root.exists() && static_root.is_dir()) 
        {
            return Err(Error::StaticDirNotExists);
        }

        let mut manifest_path = static_root.clone();
        manifest_path.push("manifest.toml");

        let manifest = Statics::read_manifest(&manifest_path)?;
        
        let statics = Statics{static_root: static_root, cache: HashMap::new()};

        return Ok(statics);
    }


    fn read_manifest(manifest_path: &PathBuf) -> Result<Table, Error>
    {
        if !manifest_path.is_file()
        {
            return Err(Error::ManifestNotExists);
        }

        let manifest_contents = match std::fs::read(manifest_path) 
        {
            Ok(data) => match String::from_utf8(data) {
                Ok(manifest_contents) => manifest_contents,
                Err(_) => return Err(Error::CouldNotLoadManifestFile)
            },
            Err(_) => return Err(Error::CouldNotLoadManifestFile),
        };

        

        match manifest_contents.parse::<Table>()
        {
            Ok(decoded) => Ok(decoded),
            Err(_) => return Err(Error::CouldNotLoadManifestFile),
        }
    }

}

pub enum Error
{
    StaticDirNotExists,
    ManifestNotExists,
    CouldNotLoadManifestFile,
    CouldNotLoadStaticFile(String),
}