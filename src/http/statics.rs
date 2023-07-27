use std::{path::PathBuf, ffi::OsString, collections::HashMap, fs};

use axum::body::Bytes;
use log::error;
use toml::Table;


pub struct Statics
{
    static_root: PathBuf,
    cache: HashMap<String, StaticEntry>, 

}

pub struct StaticEntry
{
    mime_type: String,
    contents: Option<Bytes>,
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

        let mut static_entries = HashMap::new();

        for (table_name, table_data) in Statics::read_manifest(&manifest_path)?
        {
            match table_data
            {
                toml::Value::Table(static_entry) => {
                    match (static_entry.get("path"), static_entry.get("MIME")) {
                        (Some(toml::Value::String(path)), Some(toml::Value::String(mime))) => {
                            static_entries.insert(table_name, StaticEntry{ mime_type: mime.to_owned(), contents: None, filename: OsString::from(path) });
                        },
                        _ => {return Err(Error::CouldNotLoadManifestFile);}
                    }
                },
                _ => {return Err(Error::CouldNotLoadManifestFile);}
            }
        }
        
        let statics = Statics{static_root: static_root, cache: HashMap::new()};

        return Ok(statics);
    }


    pub fn get_resource(&mut self, resource_file_name: &str) -> Option<Bytes>
    {
        let entry = self.cache.get_mut(resource_file_name)?;

        match &entry.contents
        {
            Some(data) => { Some(data.clone()) },
            None => {
                let data = Statics::read_file(&self.static_root, &entry.filename)?;
                entry.contents = Some(data.clone());
                Some(data)
            }
        }
    }

    fn read_file(root: &PathBuf, path: &OsString) -> Option<Bytes>
    {
        let target = root.join(path);
        match fs::read(&target)
        {
            Ok(data) => { 
                Some(Bytes::copy_from_slice(&data))
            },
            Err(e) => { error!("A file {} listed in the manifest could not be read.  Reason: {}", target.display(), e.to_string()); None}
        }
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