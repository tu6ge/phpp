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
