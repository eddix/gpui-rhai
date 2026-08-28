use crate::ToastRegion;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToastVariant {
    Neutral,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToastItemSpec {
    pub id: String,
    pub title: String,
    pub message: String,
    pub variant: ToastVariant,
    pub region: ToastRegion,
    pub duration_ms: u64,
    pub paused: bool,
    pub dismissible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToastHostSpec {
    pub key: String,
    pub items: Vec<ToastItemSpec>,
    pub max_visible: usize,
}
