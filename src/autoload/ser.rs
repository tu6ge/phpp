use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

use crate::error::ComposerError;

use super::{FilesData, IsVendor, Psr4Data};

impl Psr4Data {
    fn get_psr4(&self) -> Result<Vec<(String, (IsVendor, String))>, ComposerError> {
        let mut res = Vec::new();

        for (key, value) in self.data.iter() {
            for item in value.iter() {
                res.push((key.clone(), item.clone()));
            }
        }
        res.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(res)
    }

    pub fn write(&self) -> Result<(), ComposerError> {
        let mut content = String::from(
            r#"<?php

// autoload_psr4.php @generated by phpp

$vendorDir = dirname(__DIR__);
$baseDir = dirname($vendorDir);
        
return array(
"#,
        );

        let list = self.get_psr4()?;
        let mut psr4_dir_map = HashMap::new();
        for (key, val) in list.iter() {
            psr4_dir_map
                .entry(key)
                .and_modify(|v: &mut Vec<&(bool, String)>| v.push(val))
                .or_insert(vec![val]);
        }
        let mut psr4_dir_vec = Vec::new();
        for (key, val) in psr4_dir_map.iter() {
            psr4_dir_vec.push((key, val));
        }
        psr4_dir_vec.sort_by(|a, b| b.0.cmp(a.0));
        for (key, val) in psr4_dir_vec.iter() {
            let item_con = format!("    '{}' => array(\n        ", key.replace('\\', "\\\\"),);
            content.push_str(&item_con);

            for (is_vendor, val) in val.iter() {
                let val: &str = if val.ends_with('/') {
                    &val[..val.len() - 1]
                } else {
                    val
                };
                if *is_vendor {
                    content.push_str(&format!("$vendorDir . '{}',", val));
                } else {
                    content.push_str(&format!("$baseDir . '{}',", val));
                }
            }
            content.push_str("\n    ),\n");
        }
        content.push_str(");");

        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload_psr4.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }

    pub(crate) fn to_static(&self) -> (String, String) {
        let mut psr4_length_map = HashMap::new();
        for (key, _) in self.data.iter() {
            let first = key.chars().next().unwrap();
            psr4_length_map
                .entry(first)
                .and_modify(|v: &mut Vec<&String>| v.push(key))
                .or_insert(vec![key]);
        }
        let mut psr4_length_vec = Vec::new();
        for (key, v) in psr4_length_map.iter() {
            psr4_length_vec.push((key, v));
        }
        psr4_length_vec.sort_by(|a, b| b.0.cmp(a.0));

        let mut psr4_length_content = String::new();
        for (ch, vec) in psr4_length_vec.iter() {
            psr4_length_content.push_str(&format!("        '{}' => array (\n", ch));
            for it in vec.iter() {
                psr4_length_content.push_str(&format!(
                    "            '{}' => {},\n",
                    it.replace("\\", "\\\\"),
                    it.len()
                ));
            }
            psr4_length_content.push_str("        ),\n");
        }

        let mut psr4_dir_vec = Vec::new();
        for (key, val) in self.data.iter() {
            psr4_dir_vec.push((key, val));
        }
        psr4_dir_vec.sort_by(|a, b| b.0.cmp(a.0));

        let mut psr4_dir_content = String::new();

        for (key, val) in psr4_dir_vec.iter() {
            psr4_dir_content.push_str(&format!(
                "        '{}' => array(\n",
                key.replace('\\', "\\\\")
            ));
            for (i, (is, it)) in val.iter().enumerate() {
                if *is {
                    psr4_dir_content.push_str(&format!(
                        "            {}=> __DIR__ . '/..' . '/{}',\n",
                        i,
                        &it[..it.len() - 1]
                    ));
                } else {
                    psr4_dir_content.push_str(&format!(
                        "            {}=> __DIR__ . '/../..' . '/{}',\n",
                        i,
                        &it[..it.len() - 1]
                    ));
                }
            }
            psr4_dir_content.push_str("        ),\n");
        }

        (psr4_length_content, psr4_dir_content)
    }
}

impl FilesData {
    pub(crate) fn write(&self) -> Result<(), ComposerError> {
        let mut content = String::from(
            r#"<?php

// autoload_files.php @generated by phpp

$vendorDir = dirname(__DIR__);
$baseDir = dirname($vendorDir);

return array(
"#,
        );

        for (key, (is_vendor, value)) in self.data.iter() {
            let dir = if *is_vendor { "$vendorDir" } else { "$baseDir" };

            content.push_str(&format!("    '{}' => {} . '{}',\n", key, dir, value));
        }

        content.push_str("\n);");

        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload_files.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }

    pub(crate) fn to_static(&self) -> String {
        let mut files_content = String::new();
        for (key, (is_vendor, value)) in self.data.iter() {
            if *is_vendor {
                files_content.push_str(&format!(
                    "        '{}' => __DIR__ . '/..' . '{}',\n",
                    key, value
                ));
            } else {
                files_content.push_str(&format!(
                    "        '{}' => __DIR__ . '/../..' . '{}',\n",
                    key, value
                ));
            }
        }

        files_content
    }
}
