#![deny(
	absolute_paths_not_starting_with_crate,
	keyword_idents,
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_abi,
	missing_copy_implementations,
	non_ascii_idents,
	nonstandard_style,
	noop_method_call,
	pointer_structural_match,
	private_in_public,
	rust_2018_idioms,
	unused_qualifications
)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::str::FromStr;

use hard_xml::xmlparser::{ElementEnd, Token};
use hard_xml::{XmlError, XmlRead};
use serde::Serialize;
use ureq::Agent;

#[derive(Debug, Serialize)]
struct Dictionary<'a> {
	lojban_to_english: Vec<Word<'a>>,
	english_to_lojban: Vec<NlWord<'a>>,
}

impl<'input: 'a, 'a> XmlRead<'input> for Dictionary<'a> {
	fn from_reader(reader: &mut hard_xml::XmlReader<'input>) -> hard_xml::XmlResult<Self> {
		let mut lojban_to_english = None;
		let mut english_to_lojban = None;

		reader.read_till_element_start("dictionary")?;

		if let Some((key, _value)) = reader.find_attribute()? {
			return Err(XmlError::UnknownField {
				name: "Dictionary".to_owned(),
				field: key.to_owned(),
			});
		}

		if let Token::ElementEnd {
			end: ElementEnd::Empty,
			..
		} = reader.next().unwrap()?
		{
			return Err(XmlError::MissingField {
				name: "Dictionary".into(),
				field: "early end".into(),
			});
		}

		while let Some(tag) = reader.find_element_start(Some("dictionary"))? {
			if tag != "direction" {
				return Err(XmlError::UnknownField {
					name: "Dictionary".to_owned(),
					field: tag.to_owned(),
				});
			}

			reader.read_till_element_start("direction")?;

			let mut from = None;
			let mut to = None;
			while let Some((key, value)) = reader.find_attribute()? {
				match key {
					"from" => from = Some(value),
					"to" => to = Some(value),
					_ => {
						return Err(XmlError::UnknownField {
							name: "direction".into(),
							field: key.into(),
						})
					}
				}
			}

			if let Token::ElementEnd {
				end: ElementEnd::Empty,
				..
			} = reader.next().unwrap()?
			{
				return Err(XmlError::MissingField {
					name: "direction".into(),
					field: "early end".into(),
				});
			}

			match (from.as_deref(), to.as_deref()) {
				(Some("lojban"), Some("English")) => {
					let mut words = Vec::new();
					while let Some(tag) = reader.find_element_start(Some("direction"))? {
						if tag != "valsi" {
							return Err(XmlError::MissingField {
								name: "lojban-to-english".into(),
								field: "valsi".into(),
							});
						}
						words.push(Word::from_reader(reader)?);
					}
					lojban_to_english = Some(words);
				}
				(Some("English"), Some("lojban")) => {
					let mut words = Vec::new();
					while let Some(tag) = reader.find_element_start(Some("direction"))? {
						if tag != "nlword" {
							return Err(XmlError::MissingField {
								name: "english-to-lojban".into(),
								field: "nlword".into(),
							});
						}
						words.push(NlWord::from_reader(reader)?);
					}
					english_to_lojban = Some(words);
				}
				_ => {
					return Err(XmlError::UnknownField {
						name: "Dictionary".into(),
						field: "unknown direction".into(),
					})
				}
			}
		}

		let lojban_to_english = lojban_to_english.ok_or_else(|| XmlError::MissingField {
			name: "Dictionary".into(),
			field: "lojban to english".into(),
		})?;
		let english_to_lojban = english_to_lojban.ok_or_else(|| XmlError::MissingField {
			name: "Dictionary".into(),
			field: "english to lojban".into(),
		})?;

		Ok(Dictionary {
			lojban_to_english,
			english_to_lojban,
		})
	}
}

#[derive(XmlRead, Serialize, Debug)]
#[xml(strict(unknown_attribute, unknown_element), tag = "nlword")]
struct NlWord<'a> {
	#[xml(attr = "word")]
	word: Cow<'a, str>,
	#[xml(attr = "sense")]
	#[serde(skip_serializing_if = "Option::is_none")]
	sense: Option<Cow<'a, str>>,
	#[xml(attr = "place")]
	#[serde(skip_serializing_if = "Option::is_none")]
	place: Option<u32>,
	#[xml(attr = "valsi")]
	valsi: Cow<'a, str>,
}

#[derive(XmlRead, Serialize, Debug)]
#[xml(strict(unknown_attribute, unknown_element), tag = "valsi")]
struct Word<'a> {
	#[xml(attr = "word")]
	word: Cow<'a, str>,
	#[xml(attr = "type")]
	#[serde(rename = "type")]
	ty: WordType,
	#[xml(attr = "unofficial", default)]
	unofficial: bool,
	#[xml(flatten_text = "rafsi")]
	#[serde(skip_serializing_if = "<[_]>::is_empty")]
	rafsi: Vec<Cow<'a, str>>,
	#[xml(flatten_text = "selmaho")]
	#[serde(skip_serializing_if = "Option::is_none")]
	selmaho: Option<Cow<'a, str>>,
	#[xml(child = "user")]
	user: User<'a>,
	#[xml(flatten_text = "definition")]
	definition: Cow<'a, str>,
	#[xml(flatten_text = "definitionid")]
	definition_id: u32,
	#[xml(flatten_text = "notes")]
	#[serde(skip_serializing_if = "Option::is_none")]
	notes: Option<Cow<'a, str>>,
	#[xml(child = "glossword")]
	#[serde(skip_serializing_if = "<[_]>::is_empty")]
	glosses: Vec<GlossWord<'a>>,
	#[xml(child = "keyword")]
	#[serde(skip_serializing_if = "<[_]>::is_empty")]
	keywords: Vec<Keyword<'a>>,
}

#[derive(XmlRead, Serialize, Debug)]
#[xml(strict(unknown_attribute, unknown_element), tag = "keyword")]
struct Keyword<'a> {
	#[xml(attr = "word")]
	word: Cow<'a, str>,
	#[xml(attr = "place")]
	place: u32,
	#[xml(attr = "sense")]
	#[serde(skip_serializing_if = "Option::is_none")]
	sense: Option<Cow<'a, str>>,
}

#[derive(XmlRead, Serialize, Debug)]
#[xml(strict(unknown_attribute, unknown_element), tag = "glossword")]
struct GlossWord<'a> {
	#[xml(attr = "word")]
	word: Cow<'a, str>,
	#[xml(attr = "sense")]
	#[serde(skip_serializing_if = "Option::is_none")]
	sense: Option<Cow<'a, str>>,
}

#[derive(XmlRead, Serialize, Debug)]
#[xml(strict(unknown_attribute, unknown_element), tag = "user")]
struct User<'a> {
	#[xml(flatten_text = "username")]
	username: Cow<'a, str>,
	#[xml(flatten_text = "realname")]
	#[serde(skip_serializing_if = "Option::is_none")]
	realname: Option<Cow<'a, str>>,
}

#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WordType {
	BuLetteral,
	Cmavo,
	CmavoCompound,
	Cmevla,
	ExperimentalCmavo,
	ExperimentalGismu,
	Fuhivla,
	Gismu,
	Lujvo,
	ObsoleteCmavo,
	ObsoleteCmevla,
	ObsoleteFuhivla,
	ObsoleteZeiLujvo,
	ZeiLujvo,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid word type {0:?}")]
struct WordTypeFromStrError(Box<str>);

impl FromStr for WordType {
	type Err = WordTypeFromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"bu-letteral" => Self::BuLetteral,
			"cmavo" => Self::Cmavo,
			"cmavo-compound" => Self::CmavoCompound,
			"cmevla" => Self::Cmevla,
			"experimental cmavo" => Self::ExperimentalCmavo,
			"experimental gismu" => Self::ExperimentalGismu,
			"fu'ivla" => Self::Fuhivla,
			"gismu" => Self::Gismu,
			"lujvo" => Self::Lujvo,
			"obsolete cmavo" => Self::ObsoleteCmavo,
			"obsolete cmevla" => Self::ObsoleteCmevla,
			"obsolete fu'ivla" => Self::ObsoleteFuhivla,
			"obsolete zei-lujvo" => Self::ObsoleteZeiLujvo,
			"zei-lujvo" => Self::ZeiLujvo,
			_ => return Err(WordTypeFromStrError(s.into())),
		})
	}
}

fn main() {
	let username = std::env::var("JBOVLASTE_USERNAME").expect("missing JBOVLASTE_USERNAME env var");
	let password = std::env::var("JBOVLASTE_PASSWORD").expect("missing JBOVLASTE_PASSWORD env var");

	let agent = Agent::new();

	eprintln!("logging in");

	let login_status = agent
		.post("https://jbovlaste.lojban.org/login.html")
		.send_form(&[
			("backto", ""),
			("username", &username),
			("password", &password),
		])
		.unwrap()
		.status();
	assert!(
		(200..300).contains(&login_status),
		"unsuccessful login status code {login_status:?}"
	);

	eprintln!("logged in, starting export");

	let export_response = agent
		.get("https://jbovlaste.lojban.org/export/xml-export.html?lang=en")
		.call()
		.unwrap();
	let export_status = export_response.status();
	assert!(
		(200..300).contains(&export_status),
		"unsuccessful export status code {export_status:?}"
	);

	let raw = export_response.into_string().unwrap();

	eprintln!("export done, parsing and converting");

	let dictionary = Dictionary::from_str(&raw).unwrap();
	serde_json::to_writer(std::io::stdout().lock(), &dictionary).unwrap();
}
