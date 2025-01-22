use std::{
    borrow::BorrowMut,
    collections::HashMap,
    ffi::OsString,
    fs::{self, File},
    io::Read,
    sync::Arc,
};

use anyhow::Error;
use derive_more::derive::{Debug, Display, Error, From};
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent};

use crate::{shimeji::ShimejiData, ShimejiConfig};

static VALID_SHIMEJI_ATTRIBUTES: [&str; 2] = ["name", "gravity"];

#[derive(Debug)]
pub struct AnimationXml {
    name: String,
    fps: Option<u32>,
    frames: Vec<FrameXml>,
}

#[derive(Debug)]
pub struct FrameXml {
    number: u32,
    file_path: String,
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
    MissingAttribute { attribute: &'static str },
    MissingImageFile { file_path: String },
}

#[derive(Debug)]
struct XmlReturnData {
    shimeji_attributes: HashMap<String, String>,
    animations: Vec<AnimationXml>,
    name: Arc<str>,
}
fn parse_xml_data_for_shimeji_data(file: impl Read) -> Result<Box<XmlReturnData>, XmlParseError> {
    let xml_reader = xml::EventReader::new(file);

    let mut shimeji_found = false;
    let mut shimeji_attributes = None;

    let mut inside_animation = false;
    let mut animation_name: Option<String> = None;
    let mut animation_fps: Option<u32> = None;
    let mut animation_frames: Option<Vec<FrameXml>> = None;

    let mut animations: Vec<AnimationXml> = Vec::with_capacity(1);
    for xml_event in xml_reader {
        // dbg!(&xml_event);
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
                    inside_animation = true;
                    animation_frames = Some(vec![]);

                    // return the fps and parse it if it exists, otherwise send None
                    animation_fps = Some(
                        attributes
                            .iter()
                            .find(|attr| attr.name.local_name == "fps")
                            .ok_or(XmlParseError::MissingAttribute { attribute: "fps" })?
                            .value
                            .parse::<u32>()
                            .map_err(|_| XmlParseError::MalformedFile)?,
                    );
                    animation_name = Some(
                        attributes
                            .into_iter()
                            .find(|attr| &attr.name.local_name == "name")
                            .ok_or(XmlParseError::MissingAttribute { attribute: "name" })?
                            .value,
                    )
                }
                "frame" => {
                    if !inside_animation {
                        return Err(XmlParseError::MalformedFile);
                    }
                    let frames = animation_frames.borrow_mut().as_mut().unwrap();
                    let mut attr_map = HashMap::new();
                    for attr in attributes {
                        attr_map.insert(attr.name.local_name, attr.value);
                    }

                    let file_name = attr_map
                        .remove("file")
                        .ok_or(XmlParseError::MissingAttribute { attribute: "file" })?;
                    let frame_number = attr_map
                        .remove("number")
                        .ok_or(XmlParseError::MissingAttribute {
                            attribute: "number",
                        })?
                        .parse::<u32>()
                        .map_err(|_| XmlParseError::MalformedFile)?;

                    let file_exists = fs::exists(&file_name).unwrap();
                    if !file_exists {
                        return Err(XmlParseError::MissingImageFile {
                            file_path: file_name,
                        });
                    }
                    let ret = FrameXml {
                        file_path: file_name,
                        number: frame_number,
                    };
                    frames.push(ret);
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
                    let name = animation_name.take().unwrap();
                    let frames = animation_frames.take().unwrap();
                    let fps = animation_fps.take();

                    if frames.is_empty() {
                        return Err(XmlParseError::MalformedFile);
                    }

                    animations.push(AnimationXml { name, fps, frames })
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
    let mut shimeji_attributes = shimeji_attributes.unwrap();
    let name = shimeji_attributes
        .remove("name")
        .ok_or(XmlParseError::MissingAttribute { attribute: "name" })?;

    let ret = Box::new(XmlReturnData {
        name: Arc::from(name.as_str()),
        animations,
        shimeji_attributes,
    });
    log::debug!("Complete return: {ret:#?}");
    Ok(ret)
}
