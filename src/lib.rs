#![warn(
clippy::nursery,
clippy::pedantic,
)]
#![warn(
clippy::dbg_macro,
clippy::float_cmp_const,
clippy::multiple_inherent_impl,
clippy::todo,
clippy::unimplemented,
clippy::use_debug,
)]
#![allow(
clippy::default_trait_access,
clippy::explicit_deref_methods,
clippy::fn_params_excessive_bools,
clippy::future_not_send,
clippy::multiple_crate_versions,
clippy::non_ascii_literal,
clippy::redundant_else,
clippy::semicolon_if_nothing_returned,
clippy::too_many_arguments,
clippy::too_many_lines,
clippy::wildcard_imports,
)]

#[macro_use]
extern crate lazy_static;

mod docker;

use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;

use libnss::host::{Addresses, AddressFamily, Host, HostHooks};
use libnss::interop::Response;
use libnss::libnss_host_hooks;
use once_cell::sync::Lazy;

use docker::Docker;

use crate::docker::{Container, Network, SmallContainer};

struct DockerHost;
libnss_host_hooks!(docker, DockerHost);

static DOCKER: Lazy<Result<Docker, isahc::Error>> = Lazy::new(Docker::connect);

fn docker<E>() -> Result<&'static Docker, Response<E>> {
	DOCKER.as_ref().map_err(|_| Response::Unavail)
}

impl HostHooks for DockerHost {
	fn get_all_entries() -> Response<Vec<Host>> {
		get_containers().and_then(
			|cs| cs.iter().map(|c| query_container(c).and_then(container_into_host)).collect::<Result<Vec<Host>, _>>()
		).into_response()
	}

	fn get_host_by_name(name: &str, family: AddressFamily) -> Response<Host> {
		match family {
			AddressFamily::IPv4 | AddressFamily::Unspecified => match get_containers() {
				Ok(cs) => cs.into_iter()
					.filter_map(|c| query_container::<()>(&c).ok())
					.find(|h| h.config.hostname == name)
					.map_or(Response::NotFound, |c| container_into_host(c).into_response()),
				Err(e) => e,
			},
			AddressFamily::IPv6 => Response::NotFound,
		}
	}

	fn get_host_by_addr(addr: IpAddr) -> Response<Host> {
		match addr {
			IpAddr::V4(addr) => match get_containers() {
				Ok(cs) => cs.into_iter().find(|c| match networks_to_addresses::<()>(&c.network_settings.networks) {
					Ok(Addresses::V4(addrs)) => addrs.contains(&addr),
					_ => false,
				}).map_or(Response::NotFound, |c|
					match query_container(&c) {
						Ok(c) => container_into_host(c).into_response(),
						Err(e) => e,
					},
				),
				Err(e) => e,
			}
			IpAddr::V6(_) => Response::NotFound,
		}
	}
}

fn get_containers<E>() -> Result<Vec<SmallContainer>, Response<E>> {
	docker()?.get_containers().map_err(|_| Response::Unavail)
}

fn container_into_host<E>(c: Container) -> Result<Host, Response<E>> {
	Ok(Host {
		name: c.config.hostname,
		aliases: Vec::new(),
		addresses: networks_to_addresses(&c.network_settings.networks)?,
	})
}

fn query_container<E>(c: &SmallContainer) -> Result<Container, Response<E>> {
	docker()?.get_container(&c.id).map_err(|_| Response::NotFound)
}

fn networks_to_addresses<E>(networks: &HashMap<String, Network>) -> Result<Addresses, Response<E>> {
	networks.iter()
		.map(|(_name, n)| FromStr::from_str(&n.ip_address))
		.collect::<Result<_, _>>()
		.map(Addresses::V4).map_err(|_| Response::NotFound)
}

trait ResponseResultExt {
	type Data;
	fn into_response(self) -> Response<Self::Data>;
}

impl<T> ResponseResultExt for Result<T, Response<T>> {
	type Data = T;
	fn into_response(self) -> Response<Self::Data> {
		match self {
			Ok(x) => Response::Success(x),
			Err(e) => e,
		}
	}
}
