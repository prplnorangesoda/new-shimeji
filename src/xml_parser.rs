use std::{borrow::BorrowMut, collections::HashMap, fs, io::Read, sync::Arc};

use derive_more::derive::{Debug, Display, Error};
use xml::reader::XmlEvent;

static VALID_SHIMEJI_ATTRIBUTES: [&str; 2] = ["name", "gravity"];

#[derive(Debug)]
pub struct AnimationXml {
    pub name: String,
    pub fps: Option<f64>,
    pub frames: Vec<FrameXml>,
}

#[derive(Debug)]
pub struct FrameXml {
    pub number: u32,
    pub file_path: String,
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
pub struct XmlReturnData {
    pub shimeji_attributes: HashMap<String, String>,
    pub animations: Vec<AnimationXml>,
    pub name: Arc<str>,
    pub shimeji_height: u32,
    pub shimeji_width: u32,
}
pub fn parse<T: Read>(data: T) -> Result<Box<XmlReturnData>, XmlParseError> {
    let xml_reader = xml::EventReader::new(data);

    let mut shimeji_found = false;
    let mut shimeji_attributes = None;

    let mut inside_animation = false;
    let mut animation_name: Option<String> = None;
    let mut animation_fps: Option<f64> = None;
    let mut animation_frames: Option<Vec<FrameXml>> = None;

    let mut animations: Vec<AnimationXml> = Vec::with_capacity(1);
    for xml_event in xml_reader {
        // dbg!(&xml_event);
        if let Err(x) = xml_event {
            log::error!("{x}");
            break;
        }
        match xml_event.unwrap() {
            XmlEvent::Whitespace(_) => (),
            XmlEvent::StartElement {
                name, attributes, ..
            } => match name.local_name.as_str() {
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
                            .parse::<f64>()
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
            XmlEvent::EndDocument => {
                break;
            }
            XmlEvent::EndElement { name } => match name.local_name.as_str() {
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

    let height = shimeji_attributes
        .remove("height")
        .ok_or(XmlParseError::MissingAttribute {
            attribute: "height",
        })?
        .parse()
        .map_err(|_| XmlParseError::MalformedFile)?;
    let width = shimeji_attributes
        .remove("width")
        .ok_or(XmlParseError::MissingAttribute { attribute: "width" })?
        .parse()
        .map_err(|_| XmlParseError::MalformedFile)?;
    let ret = Box::new(XmlReturnData {
        name: Arc::from(name.as_str()),
        shimeji_height: height,
        shimeji_width: width,
        animations,
        shimeji_attributes,
    });
    log::debug!("Complete return: {ret:#?}");
    Ok(ret)
}
