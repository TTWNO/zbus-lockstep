//! # zbus-lockstep-macros
//!
//! This provides the `validate` macro that builds on `zbus-lockstep`.
#![doc(html_root_url = "https://docs.rs/zbus-lockstep-macros/0.2.1")]

type Result<T> = std::result::Result<T, syn::Error>;

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::ParseStream, parse_macro_input, Ident, ItemStruct, LitStr, Token};

/// Validate a struct's type signature against XML signal body type.
///
/// Retrieves the signal body type from a (collection of) XML file(s) and compares it to the
/// struct's type signature.
///
/// If the XML file(s) are found in the default location, `xml/` or `XML/` of the crate root,
/// or provided as environment variable, `LOCKSTEP_XML_PATH`, the macro can be used without
/// arguments.
///
///
/// # Arguments
///
/// `#[validate]` can take three optional arguments:
///
/// * `xml`: Path to XML file(s) containing the signal definition.
/// * `interface`: Interface name of the signal.
/// * `signal`: Signal name.
///
/// `#[validate(xml: <xml_path>, interface: <interface_name>, member: <member_name>)]`
///
/// ## `xml_path`
///
/// Without an argument, the macro looks for XML file(s) in `xml/` or `XML/` of the crate root.
/// If the definitions are to be found elsewhere, there are two options:
///
/// Use the `xml` argument:
///
/// ```ignore
/// #[validate(xml: "xml")]
/// #[derive(Type)]
/// struct RemoveNodeSignal {
///    name: String,
///    path: OwnedObjectPath,
/// }
/// ```
///
///
/// Alternatively, you can provide the XML directory path as environment variable,
/// `LOCKSTEP_XML_PATH`, which will override both default and the path argument.
///
/// ## `interface`
///
/// If more than one signal with the same name is defined in the XML file(s),
/// the macro will fail and you can provide an interface name to disambiguate.
///
/// ```ignore
/// #[validate(interface: "org.example.Node")]
/// #[derive(Type)]
/// struct RemoveNodeSignal {
///    name: String,
///    path: OwnedObjectPath,
/// }
/// ```
///
///
/// ## `signal`
///
/// If a custom signal name is desired, you can be provided using `signal:`.
///
/// ```ignore
/// #[validate(signal: "RemoveNode")]
/// #[derive(Type)]
/// struct RemoveNodeSignal {
///    name: String,
///    path: OwnedObjectPath,
/// }
/// ```
///
///
/// # Examples
///
/// ```ignore
/// #[validate(xml: "xml")]
/// #[derive(Type)]
/// struct RemoveNodeSignal {
///    name: String,
///    path: OwnedObjectPath,
/// }
/// ```
#[proc_macro_attribute]
pub fn validate(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the macro arguments.
    let args = parse_macro_input!(args as ValidateArgs);

    // Parse the item struct.
    let item_struct = parse_macro_input!(input as ItemStruct);
    let item_name = item_struct.ident.to_string();

    let xml_str = args.xml.as_ref().and_then(|p| p.to_str());

    let xml = match zbus_lockstep::resolve_xml_path(xml_str) {
        Ok(xml) => xml,
        Err(e) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Failed to resolve XML path: {e}"),
            )
            .to_compile_error()
            .into();
        }
    };

    // Store each file's XML as a string in a with the XML's file path as key.
    let mut xml_files: HashMap<PathBuf, String> = HashMap::new();
    let read_dir = std::fs::read_dir(&xml);

    // If the path does not exist, the process lacks permissions to read the path,
    // or the path is not a directory, return an error.
    if let Err(e) = read_dir {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("Failed to read XML directory: {e}"),
        )
        .to_compile_error()
        .into();
    }

    // Iterate over the directory and store each XML file as a string.
    for entry in read_dir.expect("Failed to read XML directory") {
        let entry = entry.expect("Failed to read XML file");

        // Skip directories.
        if entry.path().is_dir() {
            continue;
        }

        if entry.path().extension().expect("File has no extension.") == "xml" {
            let xml =
                std::fs::read_to_string(entry.path()).expect("Unable to read XML file to string");
            xml_files.insert(entry.path().clone(), xml);
        }
    }

    // These are later needed to call `get_signal_body_type`.
    let mut xml_file_path = None;
    let mut interface_name = None;
    let mut signal_name = None;

    // Iterate over `xml_files` and find the signal that is contained in the struct's name.
    // Or if `signal_arg` is provided, use that.
    for (path_key, xml_string) in xml_files {
        let node = zbus::xml::Node::from_str(&xml_string);

        if node.is_err() {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Failed to parse XML file: \"{}\" Err: {}",
                    path_key.to_str().unwrap(),
                    node.err().unwrap()
                ),
            )
            .to_compile_error()
            .into();
        }

        let node = node.unwrap();

        for interface in node.interfaces() {
            // We were called with an interface argument, so if the interface name does not match,
            // skip it.
            if args.interface.is_some() && interface.name() != args.interface.as_ref().unwrap() {
                continue;
            }

            for signal in interface.signals() {
                if args.signal.is_some() && signal.name() != args.signal.as_ref().unwrap() {
                    continue;
                }

                let xml_signal_name = signal.name();

                if args.signal.is_some() && xml_signal_name == args.signal.as_ref().unwrap() {
                    interface_name = Some(interface.name().to_string());
                    signal_name = Some(xml_signal_name.to_string());
                    xml_file_path = Some(path_key.clone());
                    continue;
                }

                if item_name.contains(xml_signal_name) {
                    // If we have found a signal with the same name in an earlier iteration:
                    if interface_name.is_some() && signal_name.is_some() {
                        return syn::Error::new(
                            proc_macro2::Span::call_site(),
                            "Multiple interfaces with the same signal name. Please disambiguate.",
                        )
                        .to_compile_error()
                        .into();
                    }
                    interface_name = Some(interface.name().to_string());
                    signal_name = Some(xml_signal_name.to_string());
                    xml_file_path = Some(path_key.clone());
                }
            }
        }
    }

    // Lets be nice and provide a informative compiler error message.

    // We searched all XML files and did not find a match.
    if interface_name.is_none() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "No interface matching signal name '{}' found.",
                args.signal.unwrap_or_else(|| item_name.clone())
            ),
        )
        .to_compile_error()
        .into();
    }

    // If we did find a matching interface we have also set `xml_file_path` and `signal_name`.

    let interface_name = interface_name.expect("Interface should have been found in search loop.");
    let signal_name = signal_name.expect("Signal should have been found in search loop.");

    let xml_file_path = xml_file_path.expect("XML file path should be found in search loop.");
    let xml_file_path = xml_file_path
        .to_str()
        .expect("XML file path should be valid UTF-8");

    // Create a block to return the item struct with a uniquely named validation test.
    let test_name = format!("test_{item_name}_type_signature");
    let test_name = Ident::new(&test_name, proc_macro2::Span::call_site());

    let item_struct_name = item_struct.ident.clone();
    let item_struct_name = Ident::new(
        &item_struct_name.to_string(),
        proc_macro2::Span::call_site(),
    );

    let item_plus_validation_test = quote! {
        #item_struct

        #[test]
        fn #test_name() {
            use zbus::zvariant::{self, Type};
            use zbus_lockstep::{signatures_are_eq, assert_eq_signatures};

            let xml_file = std::fs::File::open(#xml_file_path).expect(#xml_file_path);

            let item_signature_from_xml = zbus_lockstep::get_signal_body_type(
                xml_file,
                #interface_name,
                #signal_name,
                None
            ).expect("Failed to get signal body type from XML file");

            let item_signature_from_struct = <#item_struct_name as zvariant::Type>::signature();

            assert_eq_signatures!(&item_signature_from_xml, &item_signature_from_struct);
        }
    };

    item_plus_validation_test.into()
}

struct ValidateArgs {
    // Optional path to XML file
    xml: Option<PathBuf>,

    // Optional interface name
    interface: Option<String>,

    // Optional signal name
    signal: Option<String>,
}

impl syn::parse::Parse for ValidateArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut xml = None;
        let mut interface = None;
        let mut signal = None;

        while !input.is_empty() {
            let ident = input.parse::<Ident>()?;
            match ident.to_string().as_str() {
                "xml" => {
                    input.parse::<Token![:]>()?;
                    let lit = input.parse::<LitStr>()?;
                    xml = Some(PathBuf::from(lit.value()));
                }
                "interface" => {
                    input.parse::<Token![:]>()?;
                    let lit = input.parse::<LitStr>()?;
                    interface = Some(lit.value());
                }
                "signal" => {
                    input.parse::<Token![:]>()?;
                    let lit = input.parse::<LitStr>()?;
                    signal = Some(lit.value());
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("Unexpected argument: {ident}"),
                    ))
                }
            }
        }

        Ok(ValidateArgs {
            xml,
            interface,
            signal,
        })
    }
}
