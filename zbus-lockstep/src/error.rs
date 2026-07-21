#[non_exhaustive]
#[derive(Debug)]
pub enum LockstepError {
    ArgumentNotFound(String),
    InterfaceNotFound(String),
    MemberNotFound(String),
    PropertyNotFound(String),
    Io(std::io::Error),
    Xml(zbus_xml::Error),
    Variant(zvariant::Error),
}

impl std::error::Error for LockstepError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockstepError::Io(e) => Some(e),
            LockstepError::Xml(e) => Some(e),
            LockstepError::Variant(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for LockstepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockstepError::ArgumentNotFound(name) => write!(f, "Argument \"{name}\" not found."),
            LockstepError::InterfaceNotFound(name) => write!(f, "Interface \"{name}\" not found."),
            LockstepError::MemberNotFound(name) => write!(f, "Member \"{name}\" not found."),
            LockstepError::PropertyNotFound(name) => write!(f, "Property \"{name}\" not found."),
            LockstepError::Io(e) => write!(f, "IO error: {e}"),
            LockstepError::Xml(e) => write!(f, "XML error: {e}"),
            LockstepError::Variant(e) => write!(f, "Variant signature error: {e}"),
        }
    }
}
