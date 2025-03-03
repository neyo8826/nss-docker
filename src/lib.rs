mod docker;

use docker::Docker;
use libnss::host::{AddressFamily, Addresses, Host, HostHooks};
use libnss::interop::Response;
use libnss::libnss_host_hooks;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;

use crate::docker::{Container, Network, SmallContainer};

struct DockerHost;
libnss_host_hooks!(docker, DockerHost);

static DOCKER: Lazy<ResponseResult<Docker>> = Lazy::new(Docker::connect);

fn docker() -> ResponseResult<&'static Docker> {
	DOCKER.as_ref().map_err(|&e| e)
}

/*
STATUS:
+ hostname
- hostname.network
+ name
- name.network
- id
- id.network
- *.docker
*/

impl HostHooks for DockerHost {
	fn get_all_entries() -> Response<Vec<Host>> {
		get_containers().map(
			|cs| cs.iter().filter_map(|c| query_container(c).and_then(container_into_host).ok()).collect()
		).into_response()
	}

	fn get_host_by_name(name: &str, family: AddressFamily) -> Response<Host> {
		match family {
			AddressFamily::IPv4 | AddressFamily::Unspecified => match get_containers() {
				Ok(cs) => cs.into_iter()
					.filter_map(|c| query_container(&c).ok())
					.find(|h| h.config.hostname == name || h.name == name)
					.map_or(Response::NotFound, |c| container_into_host(c).into_response()),
				Err(e) => e.into(),
			},
			AddressFamily::IPv6 => Response::NotFound,
		}
	}

	fn get_host_by_addr(addr: IpAddr) -> Response<Host> {
		match addr {
			IpAddr::V4(addr) => match get_containers() {
				Ok(cs) => cs.into_iter().find(|c| match networks_to_addresses(&c.network_settings.networks) {
					Ok(Addresses::V4(addrs)) => addrs.contains(&addr),
					_ => false,
				}).map_or(Response::NotFound, |c|
					match query_container(&c) {
						Ok(c) => container_into_host(c).into_response(),
						Err(e) => e.into(),
					},
				),
				Err(e) => e.into(),
			}
			IpAddr::V6(_) => Response::NotFound,
		}
	}
}

fn get_containers() -> ResponseResult<Vec<SmallContainer>> {
	docker()?.get_containers()
}

fn container_into_host(c: Container) -> ResponseResult<Host> {
	Ok(Host {
		name: c.config.hostname,
		aliases: vec![c.name],
		addresses: networks_to_addresses(&c.network_settings.networks)?,
	})
}

fn query_container(c: &SmallContainer) -> ResponseResult<Container> {
	docker()?.get_container(&c.id)
}

fn networks_to_addresses(networks: &HashMap<String, Network>) -> ResponseResult<Addresses> {
	let addrs = networks.iter().filter_map(|(_name, n)| FromStr::from_str(&n.ip_address).ok()).collect::<Vec<_>>();
	if addrs.is_empty() {
		Err(ResponseError::NotFound)
	} else {
		Ok(Addresses::V4(addrs))
	}
}

#[derive(Copy, Clone)]
pub enum ResponseError {
	Unavail,
	NotFound,
}

pub type ResponseResult<T> = Result<T, ResponseError>;

impl<T> From<ResponseError> for Response<T> {
	fn from(e: ResponseError) -> Self {
		match e {
			ResponseError::Unavail => Self::Unavail,
			ResponseError::NotFound => Self::NotFound,
		}
	}
}

trait ResponseResultExt {
	type Item;
	fn into_response(self) -> Response<Self::Item>;
}

impl<T> ResponseResultExt for Result<T, ResponseError> {
	type Item = T;
	fn into_response(self) -> Response<Self::Item> {
		match self {
			Ok(x) => Response::Success(x),
			Err(e) => e.into(),
		}
	}
}
