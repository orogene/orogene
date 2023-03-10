use kdl::KdlDocument;

use crate::error::NodeMaintainerError;

pub trait IntoKdl: IntoKdlSealed {}

impl IntoKdl for KdlDocument {}
impl IntoKdl for String {}
impl<'a> IntoKdl for &'a str {}
impl<'a> IntoKdl for &'a String {}

impl IntoKdlSealed for KdlDocument {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError> {
        Ok(self)
    }
}

impl IntoKdlSealed for String {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError> {
        Ok(self.parse()?)
    }
}

impl<'a> IntoKdlSealed for &'a str {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError> {
        Ok(self.parse()?)
    }
}

impl<'a> IntoKdlSealed for &'a String {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError> {
        Ok(self.parse()?)
    }
}

pub trait IntoKdlSealed {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError>;
}
