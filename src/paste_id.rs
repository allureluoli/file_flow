use std::borrow::Cow;
use std::path::{Path, PathBuf};
use rand::{self, Rng};
use rocket::request::FromParam;
use rocket::http::uri::fmt::{FromUriParam, Path as UriPath};

pub struct PasteId<'a>(Cow<'a, str>);



impl PasteId<'_> {
    pub fn new(size: usize) -> PasteId<'static> {
        const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let mut id = String::with_capacity(size);
        let mut rng = rand::thread_rng();
        for _ in 0..size {
            id.push(BASE62[rng.gen::<usize>() % 62] as char);
        }
        PasteId(Cow::Owned(id))
    }

    pub fn file_path(&self) -> PathBuf {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
        Path::new(root).join(self.0.as_ref())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
}
}

impl<'a> FromParam<'a> for PasteId<'a> {
    type Error = &'a str;
    
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        Ok(PasteId(Cow::Borrowed(param)))
    }
}

impl<'a, 'b> FromUriParam<UriPath, &'b PasteId<'a>> for PasteId<'a> {
    type Target = &'b str;
    
    fn from_uri_param(param: &'b PasteId<'a>) -> &'b str {
        param.0.as_ref()
    }
}

impl<'a> AsRef<Path> for PasteId<'a> {
    fn as_ref(&self) -> &Path {
        Path::new(self.0.as_ref())
    }
}
