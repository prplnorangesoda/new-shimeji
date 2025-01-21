use std::{collections::HashMap, ffi::OsString, fs::File, io::Read, sync::Arc};

use derive_more::derive::{Display, Error, From};
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent};

use crate::{shimeji::ShimejiData, ShimejiConfig};

static VALID_SHIMEJI_ATTRIBUTES: [&str; 2] = ["name", "gravity"];

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
) -> Result<ShimejiConfig, XmlParseError> {
    let file_name: OsString = file_name.into();
    let file = File::open(file_name).expect("file to open should exist");
    let parsed = parse_xml_data_for_shimeji_data(file)?;

    Ok(ShimejiConfig {
        name: parsed.name,
        data: Arc::new(ShimejiData {}),
    })
}

#[derive(Debug, Error, Display)]
pub enum XmlParseError {
    MultipleShimeji,
    NoShimeji,
    MalformedFile,
    MissingAttributes,
}

struct XmlReturnData {
    shimeji_attributes: HashMap<String, String>,
    name: Arc<str>,
}
fn parse_xml_data_for_shimeji_data(file: impl Read) -> Result<Box<XmlReturnData>, XmlParseError> {
    let xml_reader = xml::EventReader::new(file);

    let mut shimeji_found = false;
    let mut shimeji_attributes = None;

    let mut inside_animation = false;
    for xml_event in xml_reader {
        dbg!(&xml_event);
        match xml_event {
            Err(x) => {
                log::error!("{x}");
                break;
            }
            Ok(XmlEvent::Whitespace(_)) => (),
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "Shimeji" => {
                    if shimeji_found {
                        return Err(XmlParseError::MultipleShimeji);
                    }
                    shimeji_found = true;
                    shimeji_attributes =
                        Some(HashMap::with_capacity(VALID_SHIMEJI_ATTRIBUTES.len()));
                    for attr in attributes {
                        let name = attr.name.local_name;
                        shimeji_attributes
                            .as_mut()
                            .unwrap()
                            .insert(name, attr.value);
                    }
                    log::debug!("{0:?}", &shimeji_attributes);
                }
                "Animation" => {
                    if inside_animation {
                        return Err(XmlParseError::MalformedFile);
                    }
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
                "Shimeji" => {}
                "Animation" => {
                    inside_animation = false;
                }
                _ => continue,
            },
            other => {
                log::warn!("Unhandled event: {other:?}");
                continue;
            }
        }
    }
    if !shimeji_found {
        return Err(XmlParseError::NoShimeji);
    }
    let shimeji_attributes = shimeji_attributes.unwrap();
    let name = shimeji_attributes
        .get("name")
        .ok_or(XmlParseError::MissingAttributes)?;
    Ok(Box::new(XmlReturnData {
        name: Arc::from(name.as_str()),
        shimeji_attributes,
    }))
}
