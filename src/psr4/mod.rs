use indexmap::IndexMap;

mod de;
mod ser;

type isVendor = bool;

#[derive(Debug, Default)]
pub(crate) struct Psr4Data {
    data: IndexMap<String, Vec<(isVendor, String)>>,
}
