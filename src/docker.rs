use std::collections::HashMap;

use isahc::{HttpClient, ReadResponseExt};
use isahc::config::{Configurable, Dialer};
use serde::Deserialize;

pub struct Docker {
	client: HttpClient,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Container {
	pub config: Config,
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
	pub fn connect() -> Result<Self, isahc::Error> {
		let client = isahc::HttpClientBuilder::new()
			.max_connections(1)
			.dial(Dialer::unix_socket("/var/run/docker.sock"))
			.default_header("Content-Type", "application/json")
			.default_header("Accept", "application/json")
			.build()?;
		Ok(Self { client })
	}

	pub fn get_containers(&self) -> std::io::Result<Vec<SmallContainer>> {
		Ok(self.client.get("http://localhost/containers/json?all=false")?.json()?)
	}

	pub fn get_container(&self, id: &str) -> std::io::Result<Container> {
		Ok(self.client.get(format!("http://localhost/containers/{}/json", id))?.json()?)
	}
}
