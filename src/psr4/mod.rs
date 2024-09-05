use indexmap::IndexMap;

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
