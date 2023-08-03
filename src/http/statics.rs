use std::{path::PathBuf, collections::HashMap, fs};

use axum::body::Bytes;
use log::{error, debug};
use toml::Table;


pub struct Statics
{
    cache: HashMap<String, StaticEntry>, 
}

pub struct StaticEntry
{
    mime_type: String,
    contents: Bytes,
}

impl Statics
{
    pub fn with_root(static_root: &PathBuf) -> Result<Statics, Error>
    {
        debug!("Starting with_root");
        if !(static_root.exists() && static_root.is_dir()) 
        {
            error!("The directory represented by {} does not exist.", static_root.display());
            return Err(Error::StaticDirNotExists);
        }

        let mut manifest_path = static_root.clone();
        manifest_path.push("manifest.toml");
        debug!("Attempting to load manifest from {}", manifest_path.display());

        let mut static_entries = HashMap::new();

        for (table_name, table_data) in Statics::read_manifest(&manifest_path)?
        {
            
            debug!("Processing entries for table {}", table_name);
            debug!("is table_data a table? {}", table_data.is_table());
            match table_data
            {
                toml::Value::Table(static_entry) => {
                    static_entry.keys().into_iter().for_each(|k| debug!("Key name: {}", k));

                    match (static_entry.get("path"), static_entry.get("MIME")) {
                        (Some(toml::Value::String(path)), Some(toml::Value::String(mime))) => {
                            let static_path = PathBuf::from(path);
                            let data = Statics::read_file(&static_root, &static_path)?;

                            debug!("Storing file data for key {}, MIME type {}", table_name, mime);

                            static_entries.insert(table_name, StaticEntry{ mime_type: mime.to_owned(), contents: data });
                        },
                        _ => {
                            error!("Table data table type did not have expected keys path and mime.");
                            return Err(Error::CouldNotLoadManifestFile);
                        }
                    }
                },
                _ => {
                    error!("Table data is not the expected table type.");
                    return Err(Error::CouldNotLoadManifestFile);
                }
            }
        }
        
        let statics = Statics{cache: static_entries};

        return Ok(statics);
    }


    pub fn get_resource(&self, resource_file_name: &str) -> Option<Bytes>
    {
        debug!("Processing request to publish data for static resource {}", resource_file_name);
        match self.cache.get(resource_file_name)
        {
            Some(entry) => { Some(entry.contents.clone()) },
            None => { None }
        }
    }

    pub  fn get_mime<'a>(&'a self, resource_file_name: &str) -> Option<&'a str>
    {
        let mime = self.cache.get(resource_file_name)?;

        Some(&mime.mime_type)
    }

    fn read_file(root: &PathBuf, path: &PathBuf) -> Result<Bytes, Error>
    {
        let target = root.join(path);
        match fs::read(&target)
        {
            Ok(data) => { 
                Ok(Bytes::copy_from_slice(&data))
            },
            Err(e) => { 
                error!("A file {} listed in the manifest could not be read.  Reason: {}", target.display(), e.to_string());
                match path.to_str()
                {
                    Some(path_str) => {
                        Err(Error::CouldNotLoadStaticFile(String::from(path_str)))
                    },
                    None => {Err(Error::FilePathNotStringable)}
                }
                
            }
        }
    }


    fn read_manifest(manifest_path: &PathBuf) -> Result<Table, Error>
    {
        debug!("Starting read_manifest");

        if !manifest_path.is_file()
        {
            error!("The object represented by {} is not a file.", manifest_path.display());
            return Err(Error::ManifestNotExists);
        }

        let manifest_contents = match std::fs::read(manifest_path) 
        {
            Ok(data) => match String::from_utf8(data) {
                Ok(manifest_contents) => {
                    debug!("Manifest contents read"); 
                    manifest_contents
                },
                Err(e) => {
                    error!("Manifest contents could not be read as UTF-8: {}", e.to_string());
                    return Err(Error::CouldNotLoadManifestFile)
                }
            },
            Err(e) => {
                error!("Could not open file: {}", e.to_string());
                return Err(Error::CouldNotLoadManifestFile)
            },
        };

        debug!("Completed load of manifest.toml contents.");

        match manifest_contents.parse::<Table>()
        {
            Ok(decoded) => Ok(decoded),
            Err(e) => {
                error!("Could not parse the contents of the file into a TOML table.  Reason: {}", e.message());
                return Err(Error::CouldNotLoadManifestFile)
            },
        }
    }

}

pub enum Error
{
    StaticDirNotExists,
    ManifestNotExists,
    CouldNotLoadManifestFile,
    CouldNotLoadStaticFile(String),
    FilePathNotStringable,
}