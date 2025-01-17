use std::{collections::HashMap, ffi::OsString, fs::File, sync::Arc};

use derive_more::derive::{Display, Error, From};
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent};

use crate::{shimeji::ShimejiData, ShimejiConfig};

static VALID_SHIMEJI_ATTRIBUTES: [&'static str; 2] = ["name", "gravity"];

#[derive(Debug, From, Error, Display)]
pub enum ConfigCreationError {
    MissingAttribute { attribute: &'static str },
}
pub fn create_config_from_file(
    file_name: impl Into<OsString>,
) -> Result<ShimejiConfig, ConfigCreationError> {
    let file_name: OsString = file_name.into();

    let file = File::open(file_name).expect("File name should be valid");

    let xml_reader = xml::EventReader::new(file);

    let mut document_ended_successfully = false;
    for xml_event in xml_reader {
        match xml_event {
            Err(x) => {
                log::error!("{x}");
                break;
            }
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "Shimeji" => {
                    let mut attributes_retrieved =
                        HashMap::with_capacity(VALID_SHIMEJI_ATTRIBUTES.len());
                    for attr in attributes {
                        let name = attr.name.local_name;
                        attributes_retrieved.insert(name, attr.value);
                    }
                    log::debug!("{attributes_retrieved:?}");
                }
                _ => {
                    log::debug!("Unrecognized local_name: {}", name.local_name);
                    continue;
                }
            },
            Ok(XmlEvent::EndDocument) => {
                document_ended_successfully = true;
                break;
            }
            _ => {
                log::warn!("Unhandled event");
                continue;
            }
        }
    }
    let name = String::from("d");
    return Ok(ShimejiConfig {
        name: Arc::from(name.as_str()),
        data: Arc::new(ShimejiData {}),
    });
}
