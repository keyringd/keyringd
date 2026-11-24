use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LargeBlobSupport {
    #[default]
    LargeBlob,
    LargeBlobKey,
    Unsupported,
}
