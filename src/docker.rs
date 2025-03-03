use std::collections::HashMap;

use isahc::{HttpClient, ReadResponseExt};
use isahc::config::{Configurable, Dialer};
use serde::{Deserialize, Deserializer};
use serde::de::DeserializeOwned;

use crate::{ResponseError, ResponseResult};

pub struct Docker {
	client: HttpClient,
}

#[allow(clippy::option_if_let_else)]
fn deserialize_container_name<'de, D>(d: D) -> Result<String, D::Error> where D: Deserializer<'de> {
	let name: String = Deserialize::deserialize(d)?;
	Ok(if let Some(name) = name.strip_prefix('/') { name.to_owned() } else { name })
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Container {
	pub config: Config,
	#[serde(deserialize_with = "deserialize_container_name")]
	pub name: String,
	pub network_settings: NetworkSettings,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SmallContainer {
	pub id: String,
	pub network_settings: NetworkSettings,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
	pub hostname: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkSettings {
	pub networks: HashMap<String, Network>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Network {
	#[serde(rename = "IPAddress")]
	pub ip_address: String,
}

impl Docker {
	pub fn connect() -> ResponseResult<Self> {
		let client = isahc::HttpClientBuilder::new()
			.max_connections(1)
			.dial(Dialer::unix_socket("/var/run/docker.sock"))
			.default_header("Content-Type", "application/json")
			.default_header("Accept", "application/json")
			.build().map_err(|_| ResponseError::Unavail)?;
		Ok(Self { client })
	}

	pub fn get_containers(&self) -> ResponseResult<Vec<SmallContainer>> {
		self.get_json("http://localhost/containers/json?all=false")
	}

	pub fn get_container(&self, id: &str) -> ResponseResult<Container> {
		self.get_json(&format!("http://localhost/containers/{id}/json"))
	}

	fn get_json<T: DeserializeOwned>(&self, uri: &str) -> ResponseResult<T> {
		self.client
			.get(uri).map_err(|_| ResponseError::NotFound)?
			.json().map_err(|_| ResponseError::NotFound)
	}
}
