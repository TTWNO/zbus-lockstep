//! # zbus-lockstep
//!
//! `zbus-lockstep` is a library for retrieving `DBus` type signatures from XML descriptions
//! and comparing those with the signature of your type signatures to ensure that they are
//! compatible.
//!
//! `zbus-lockstep`'s intended use is in tests, such that it will assure your types conform
//! to XML definitions with `cargo test`.
#![doc(html_root_url = "https://docs.rs/zbus-lockstep/0.1.0")]
#![allow(clippy::missing_errors_doc)]

pub mod marshall;
use std::io::Read;

pub use marshall::signatures_are_eq;
use zbus::{
    xml::{Arg, Node},
    zvariant::Signature,
    Error::{InterfaceNotFound, MissingParameter},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Retrieves a signal's body type signature from `DBus` XML.
///
/// If you provide an argument name, then the signature of that argument is returned.
/// If you do not provide an argument name, then the signature of all arguments is returned.    
///
/// # Examples
///
/// ```rust
/// use std::fs::File;
/// use std::io::{Seek, SeekFrom, Write};
/// use tempfile::tempfile;
/// use zbus::zvariant::{Signature, Type};
/// use zbus::zvariant::OwnedObjectPath;
/// use zbus_lockstep::get_signal_body_type;
/// use zbus_lockstep::signatures_are_eq;
/// use zbus_lockstep::assert_eq_signatures;
///
/// let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
/// <node xmlns:doc="http://www.freedesktop.org/dbus/1.0/doc.dtd">
/// <interface name="org.freedesktop.bolt1.Manager">
///   <signal name="DeviceAdded">
///    <arg name="device" type="o"/>
///  </signal>
/// </interface>
/// </node>
/// "#;
///
/// let mut xml_file: File = tempfile().unwrap();   
/// xml_file.write_all(xml.as_bytes()).unwrap();
/// xml_file.seek(SeekFrom::Start(0)).unwrap();
///
/// #[derive(Debug, PartialEq, Type)]
/// struct DeviceEvent {
///    device: OwnedObjectPath,
/// }
///
/// let interface_name = "org.freedesktop.bolt1.Manager";
/// let member_name = "DeviceAdded";
///
/// let signature = get_signal_body_type(xml_file, interface_name, member_name, None).unwrap();
///
/// // Single `DBus` type codes, here 'o' are returned as a single character.
/// // Also, signal body types (often) omit the struct or tuple type parentheses.
///
/// assert_ne!(signature, DeviceEvent::signature());
///
/// // However, the signatures are equivalent.
///
/// assert!(signatures_are_eq(&signature, &DeviceEvent::signature()));  
///
/// // If you want to check that the signatures are equivalent, you can use the
/// // assert_eq_signatures! macro.
///     
/// assert_eq_signatures!(&signature, &DeviceEvent::signature());
/// ```
///
/// # Notes
///
/// See [`marshall::signatures_are_eq`] for more information about
/// comparing signatures.
pub fn get_signal_body_type<'a>(
    mut xml: impl Read,
    interface_name: &str,
    member_name: &str,
    arg: Option<&str>,
) -> Result<Signature<'a>> {
    let node = Node::from_reader(&mut xml)?;

    let interfaces = node.interfaces();
    let interface = interfaces
        .iter()
        .find(|iface| iface.name() == interface_name)
        .ok_or(InterfaceNotFound)?;

    let singals = interface.signals();
    let signal = singals
        .iter()
        .find(|signal| signal.name() == member_name)
        .ok_or(MissingParameter("no signal matching supplied member"))?;

    let signature = {
        if let Some(arg_name) = arg {
            let args = signal.args();
            let arg = args
                .iter()
                .find(|arg| arg.name() == Some(arg_name))
                .ok_or(MissingParameter("no matching argument found"))?;
            arg.ty().to_owned()
        } else {
            signal.args().into_iter().map(Arg::ty).collect::<String>()
        }
    };
    Ok(Signature::from_string_unchecked(signature))
}

/// Gets the signature of a property's type from XML.
///
/// # Examples
///     
/// ```rust
/// use std::fs::File;
/// use std::io::{Seek, SeekFrom, Write};
/// use tempfile::tempfile;
/// use zbus::zvariant::Type;
/// use zbus_lockstep::get_property_type;
///     
/// #[derive(Debug, PartialEq, Type)]
/// struct InUse(bool);
///     
/// let xml = String::from(r#"
/// <node>
/// <interface name="org.freedesktop.GeoClue2.Manager">
///   <property type="b" name="InUse" access="read"/>
/// </interface>
/// </node>
/// "#);
///
/// let mut xml_file: File = tempfile().unwrap();
/// xml_file.write_all(xml.as_bytes()).unwrap();
/// xml_file.seek(SeekFrom::Start(0)).unwrap();
///     
/// let interface_name = "org.freedesktop.GeoClue2.Manager";
/// let property_name = "InUse";
///
/// let signature = get_property_type(xml_file, interface_name, property_name).unwrap();
/// assert_eq!(signature, InUse::signature());
/// ```
pub fn get_property_type<'a>(
    mut xml: impl Read,
    interface_name: &str,
    property_name: &str,
) -> Result<Signature<'a>> {
    let node = Node::from_reader(&mut xml)?;

    let interfaces = node.interfaces();
    let interface = interfaces
        .iter()
        .find(|iface| iface.name() == interface_name)
        .ok_or(InterfaceNotFound)?;

    let properties = interface.properties();
    let property = properties
        .iter()
        .find(|property| property.name() == property_name)
        .ok_or(MissingParameter("no property matching supplied member"))?;

    let signature = property.ty().to_owned();
    Ok(Signature::from_string_unchecked(signature))
}

/// Gets the signature of a method's return type from XML.
///
/// If you provide an argument name, then the signature of that argument is returned.
/// If you do not provide an argument name, then the signature of all arguments is returned.
///     
///     
/// # Examples
///     
/// ```rust
/// use std::fs::File;
/// use std::io::{Seek, SeekFrom, Write};
/// use tempfile::tempfile;
/// use zbus::zvariant::Type;
/// use zbus_lockstep::get_method_return_type;
///     
/// #[derive(Debug, PartialEq, Type)]
/// #[repr(u32)]
/// enum Role {
///     Invalid,
///     TitleBar,
///     MenuBar,
///     ScrollBar,
/// }
///
/// let xml = String::from(r#"
/// <node>
/// <interface name="org.a11y.atspi.Accessible">
///    <method name="GetRole">
///       <arg name="role" type="u" direction="out"/>
///   </method>
/// </interface>
/// </node>
/// "#);
///
/// let mut xml_file: File = tempfile().unwrap();
/// xml_file.write_all(xml.as_bytes()).unwrap();
/// xml_file.seek(SeekFrom::Start(0)).unwrap();
///
/// let interface_name = "org.a11y.atspi.Accessible";
/// let member_name = "GetRole";
///     
/// let signature = get_method_return_type(xml_file, interface_name, member_name, None).unwrap();
/// assert_eq!(signature, Role::signature());
/// ```
pub fn get_method_return_type<'a>(
    mut xml: impl Read,
    interface_name: &str,
    member_name: &str,
    arg_name: Option<&str>,
) -> Result<Signature<'a>> {
    let node = Node::from_reader(&mut xml)?;

    let interfaces = node.interfaces();
    let interface = interfaces
        .iter()
        .find(|iface| iface.name() == interface_name)
        .ok_or(InterfaceNotFound)?;

    let methods = interface.methods();
    let method = methods
        .iter()
        .find(|method| method.name() == member_name)
        .ok_or(MissingParameter("no method matches supplied member"))?;

    let args = method.args();

    let signature = if arg_name.is_some() {
        args.iter()
            .find(|arg| arg.name() == arg_name)
            .ok_or(MissingParameter("no matching argument found"))?
            .ty()
            .to_owned()
    } else {
        args.iter()
            .filter(|arg| arg.direction() == Some("out"))
            .map(|arg| arg.ty())
            .collect::<String>()
    };

    Ok(Signature::from_string_unchecked(signature))
}

/// Gets the signature of a method's argument type from XML.
///
/// Useful when one or more arguments, used to call a method, outline a useful type.
///
/// If you provide an argument name, then the signature of that argument is returned.
/// If you do not provide an argument name, then the signature of all arguments to the call is
/// returned.
///
/// # Examples
///
/// ```rust
/// use std::fs::File;
/// use std::collections::HashMap;
/// use std::io::{Seek, SeekFrom, Write};
/// use tempfile::tempfile;
/// use zbus::zvariant::{Type, Value};
/// use zbus_lockstep::get_method_args_type;
/// use zbus_lockstep::assert_eq_signatures;
/// use zbus_lockstep::signatures_are_eq;
///
/// let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
/// <node xmlns:doc="http://www.freedesktop.org/dbus/1.0/doc.dtd">
///  <interface name="org.freedesktop.Notifications">
///    <method name="Notify">
///      <arg type="s" name="app_name" direction="in"/>
///      <arg type="u" name="replaces_id" direction="in"/>
///      <arg type="s" name="app_icon" direction="in"/>
///      <arg type="s" name="summary" direction="in"/>
///      <arg type="s" name="body" direction="in"/>
///      <arg type="as" name="actions" direction="in"/>
///      <arg type="a{sv}" name="hints" direction="in"/>
///      <arg type="i" name="expire_timeout" direction="in"/>
///      <arg type="u" name="id" direction="out"/>
///    </method>
///  </interface>
/// </node>
/// "#;
///
/// #[derive(Debug, PartialEq, Type)]
/// struct Notification<'a> {
///    app_name: String,
///    replaces_id: u32,
///    app_icon: String,
///    summary: String,
///    body: String,
///    actions: Vec<String>,
///    hints: HashMap<String, Value<'a>>,  
///    expire_timeout: i32,
/// }
///
/// let mut xml_file = tempfile().unwrap();
/// xml_file.write_all(xml.as_bytes()).unwrap();
/// xml_file.seek(SeekFrom::Start(0)).unwrap();
///
/// let interface_name = "org.freedesktop.Notifications";
/// let member_name = "Notify";
///     
/// let signature = get_method_args_type(xml_file, interface_name, member_name, None).unwrap();
/// assert_eq_signatures!(&signature, &Notification::signature());
/// ```
pub fn get_method_args_type<'a>(
    mut xml: impl Read,
    interface_name: &str,
    member_name: &str,
    arg_name: Option<&str>,
) -> Result<Signature<'a>> {
    let node = Node::from_reader(&mut xml)?;

    let interfaces = node.interfaces();
    let interface = interfaces
        .iter()
        .find(|iface| iface.name() == interface_name)
        .ok_or(InterfaceNotFound)?;

    let methods = interface.methods();
    let method = methods
        .iter()
        .find(|method| method.name() == member_name)
        .ok_or(MissingParameter("no method matches supplied member"))?;

    let args = method.args();

    let signature = if arg_name.is_some() {
        args.iter()
            .find(|arg| arg.name() == arg_name)
            .ok_or(MissingParameter("no matching argument found"))?
            .ty()
            .to_owned()
    } else {
        args.iter()
            .filter(|arg| arg.direction() == Some("in"))
            .map(|arg| arg.ty())
            .collect::<String>()
    };

    Ok(Signature::from_string_unchecked(signature))
}

#[cfg(test)]
mod test {
    use std::io::{Seek, SeekFrom, Write};

    use tempfile::tempfile;
    use zbus::zvariant::{OwnedObjectPath, Type};

    use crate::get_signal_body_type;

    #[test]
    fn test_get_signature_of_cache_add_accessible() {
        #[derive(Debug, PartialEq, Type)]
        struct Accessible {
            name: String,
            path: OwnedObjectPath,
        }

        #[derive(Debug, PartialEq, Type)]
        struct CacheItem {
            obj: Accessible,
            application: Accessible,
            parent: Accessible,
            index_in_parent: i32,
            child_count: i32,
            interfaces: Vec<String>,
            name: String,
            role: u32,
            description: String,
            state_set: Vec<u32>,
        }

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <node xmlns:doc="http://www.freedesktop.org/dbus/1.0/doc.dtd">
                <interface name="org.a11y.atspi.Cache">
                    <signal name="AddAccessible">
                        <arg name="nodeAdded" type="((so)(so)(so)iiassusau)"/>
                        <annotation name="org.qtproject.QtDBus.QtTypeName.In0" value="QSpiAccessibleCacheItem"/>
                    </signal>
                </interface>
            </node>
        "#;

        let mut xml_file = tempfile().unwrap();
        xml_file.write_all(xml.as_bytes()).unwrap();
        xml_file.seek(SeekFrom::Start(0)).unwrap();

        let interface_name = "org.a11y.atspi.Cache";
        let member_name = "AddAccessible";

        let signature = get_signal_body_type(xml_file, interface_name, member_name, None).unwrap();
        assert_eq!(signature, CacheItem::signature());
    }
}