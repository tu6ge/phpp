use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
};

use indexmap::IndexMap;

use crate::{
    error::ComposerError,
    json::Composer,
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

impl Psr4Data {
    /// append from composer.json
    pub fn append_json(&mut self, json: &Composer) {
        let mut res = Vec::new();
        if let Some(AutoloadEnum::Psr(Autoload {
            psr4: Some(psr), ..
        })) = &json.autoload
        {
            for (key, value) in psr.iter() {
                if let PsrValue::String(value) = value {
                    let mut v = format!("/");
                    //v.push('/');
                    let value = if value.ends_with('/') {
                        value.trim_end_matches('/')
                    } else {
                        &value
                    };
                    v.push_str(value);
                    res.push((key.to_owned(), v));
                }
            }
        }
        for (key, value) in res.iter() {
            self.data
                .entry(key.to_owned())
                .and_modify(|v: &mut Vec<(bool, String)>| {
                    if !v.contains(&(false, value.to_owned())) {
                        v.push((false, value.to_owned()));
                    }
                })
                .or_insert(vec![(false, value.to_owned())]);
        }
    }

    /// append from composer.lock
    pub fn append_lock(&mut self, lock: &ComposerLock) {
        let mut res = Vec::new();
        for item in lock.packages.iter() {
            if let Some(AutoloadEnum::Psr(Autoload {
                psr4: Some(psr), ..
            })) = &item.autoload
            {
                for (key, value) in psr.iter() {
                    if let PsrValue::String(value) = value {
                        let mut v = format!("/{}", item.name.as_ref().unwrap());
                        //v.push('/');
                        let value = if value.ends_with('/') {
                            value.trim_end_matches('/')
                        } else {
                            &value
                        };
                        if !value.is_empty() {
                            v.push('/');
                        }
                        v.push_str(value);
                        res.push((key.to_owned(), v));
                    }
                }
            }
        }
        //res.sort_by(|a, b| b.0.cmp(&a.0));
        for (key, value) in res.iter() {
            self.data
                .entry(key.to_owned())
                .and_modify(|v: &mut Vec<(bool, String)>| {
                    if !v.contains(&(true, value.to_owned())) {
                        v.push((true, value.to_owned()));
                    }
                })
                .or_insert(vec![(true, value.to_owned())]);
        }
        // println!("{:#?}", self.data);
        // todo!()
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

    /// append from composer.json
    pub fn append_json(&mut self, json: &Composer) {
        if let Some(AutoloadEnum::Psr(Autoload {
            files: Some(files), ..
        })) = &json.autoload
        {
            for it in files {
                let con = format!("/{}", it);
                self.insert(false, con);
            }
        }
    }

    /// append from composer.lock
    pub fn append_lock(&mut self, lock: &ComposerLock) {
        for item in lock.packages.iter() {
            if let Some(AutoloadEnum::Psr(Autoload {
                files: Some(files), ..
            })) = &item.autoload
            {
                for it in files {
                    let con = format!("/{}/{}", item.name.as_ref().unwrap(), it);
                    self.insert(true, con);
                }
            }
        }
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
}
