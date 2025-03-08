use kdl::KdlDocument;

use crate::error::NodeMaintainerError;

pub trait IntoKdl: IntoKdlSealed {}

impl IntoKdl for KdlDocument {}
impl IntoKdl for String {}
impl IntoKdl for & str {}
impl IntoKdl for & String {}

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

impl IntoKdlSealed for & str {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError> {
        Ok(self.parse()?)
    }
}

impl IntoKdlSealed for & String {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError> {
        Ok(self.parse()?)
    }
}

pub trait IntoKdlSealed {
    fn into_kdl(self) -> Result<KdlDocument, NodeMaintainerError>;
}
