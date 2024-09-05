use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
};

use indexmap::IndexMap;

use crate::{
    error::ComposerError,
    package::{Autoload, AutoloadEnum, ComposerLock, PsrValue},
};

mod de;
mod ser;

type IsVendor = bool;

#[derive(Debug, Default)]
pub(crate) struct Psr4Data {
    data: IndexMap<String, Vec<(IsVendor, String)>>,
}

#[derive(Debug, Default)]
pub(crate) struct FilesData {
    data: IndexMap<String, (IsVendor, String)>,
}

#[derive(Debug, Default)]
pub(crate) struct StaticData {
    files: String,
    psr4_length: String,
    psr4_dir: String,
}

impl From<&ComposerLock> for Psr4Data {
    fn from(value: &ComposerLock) -> Self {
        let mut res = Vec::new();
        for item in value.packages.iter() {
            if let Some(AutoloadEnum::Psr(Autoload {
                psr4: Some(psr), ..
            })) = &item.autoload
            {
                for (key, value) in psr.iter() {
                    if let PsrValue::String(value) = value {
                        let mut v = item.name.as_ref().unwrap().clone();
                        v.push('/');
                        v.push_str(value);
                        res.push((key.to_owned(), v));
                    }
                }
            }
        }
        res.sort_by(|a, b| b.0.cmp(&a.0));

        let mut data = IndexMap::new();
        for (key, value) in res.iter() {
            data.entry(key.to_owned())
                .and_modify(|v: &mut Vec<(bool, String)>| {
                    v.push((true, value.to_owned()));
                })
                .or_insert(vec![(true, value.to_owned())]);
        }

        Self { data }
    }
}

impl FilesData {
    pub fn insert(&mut self, is_vendor: IsVendor, value: String) -> Option<(IsVendor, String)> {
        use sha1::Digest;
        use sha1::Sha1;

        let mut hasher = Sha1::new();
        hasher.update(&value.as_bytes());
        let sha1 = hasher.finalize();
        let key = hex::encode(sha1);

        self.data.insert(key, (is_vendor, value))
    }
}

impl From<&ComposerLock> for FilesData {
    fn from(value: &ComposerLock) -> Self {
        let mut this = Self::default();
        for item in value.packages.iter() {
            if let Some(AutoloadEnum::Psr(Autoload {
                files: Some(files), ..
            })) = &item.autoload
            {
                for it in files {
                    let con = format!("/{}/{}", item.name.as_ref().unwrap(), it);
                    this.insert(true, con);
                }
            }
        }

        this
    }
}

impl StaticData {
    pub fn from(files: &FilesData, psr4: &Psr4Data) -> Self {
        let files = files.to_static();
        let (psr4_length, psr4_dir) = psr4.to_static();

        Self {
            files,
            psr4_length,
            psr4_dir,
        }
    }

    pub fn write(&self) -> Result<(), ComposerError> {
        let content = include_str!("../../asset/autoload_static.php");

        let content = content.replace("__FILES_CONTENT__", &self.files);
        let content = content.replace("__PSR4_LENGTH__", &self.psr4_length);
        let content = content.replace("__PSR4_DIRS__", &self.psr4_dir);

        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload_static.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
}
