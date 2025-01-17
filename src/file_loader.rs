use std::{collections::HashMap, ffi::OsString, fs::File, sync::Arc};

use derive_more::derive::{Display, Error, From};
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent};

use crate::{shimeji::ShimejiData, ShimejiConfig};

static VALID_SHIMEJI_ATTRIBUTES: [&'static str; 2] = ["name", "gravity"];

#[derive(Debug, Error, Display)]
pub enum ConfigCreationError {
    MissingAttribute { attribute: &'static str },
    MissingItem { item: &'static str },
}

#[derive(Debug)]
pub struct AnimationXml {
    frames: Vec<FrameXml>,
}

#[derive(Debug)]
pub struct FrameXml {}

#[derive(Debug)]
pub enum XmlItem {
    Animation(AnimationXml),
}

pub fn create_config_from_file_name(
    file_name: impl Into<OsString>,
) -> Result<ShimejiConfig, ConfigCreationError> {
    let file_name: OsString = file_name.into();
    let file = File::open(file_name).expect("file to open should exist");
    let parsed = parse_xml_file_for_shimeji_data(file);

    let name = String::from("d");
    return Ok(ShimejiConfig {
        name: Arc::from(name.as_str()),
        data: Arc::new(ShimejiData {}),
    });
}

#[derive(Debug, Error, Display)]
enum XmlError {
    MultipleShimeji,
}
fn parse_xml_file_for_shimeji_data(file: File) -> Result<Vec<XmlItem>, XmlError> {
    let xml_reader = xml::EventReader::new(file);

    let mut in_shimeji = false;
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
                    if in_shimeji {
                        return Err(XmlError::MultipleShimeji);
                    }
                    in_shimeji = true;
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
                break;
            }
            Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                "Shimeji" => {
                    in_shimeji = false;
                }
                _ => continue,
            },
            _ => {
                log::warn!("Unhandled event");
                continue;
            }
        }
    }
    return Ok(vec![]);
}
