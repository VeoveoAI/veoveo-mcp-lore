//! The server's resource addresses under the `repo` scheme. The route declaration
//! owns each shape and its typed parameters; the shared URI machinery owns encoding and
//! parsing, so no address is built or split by hand.
use std::{borrow::Cow, error::Error, fmt};

use veoveo_types::{ResourceFieldCodec, ResourceRouteError, ResourceUriError, Vocabulary};

/// A resource address that failed admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoResourceError {
    Uri(ResourceUriError),
    UnknownResource,
}

impl fmt::Display for RepoResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uri(error) => error.fmt(f),
            Self::UnknownResource => f.write_str("unknown or noncanonical repo resource address"),
        }
    }
}

impl Error for RepoResourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Uri(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceUriError> for RepoResourceError {
    fn from(error: ResourceUriError) -> Self {
        Self::Uri(error)
    }
}

/// A document the server embeds and serves under `repo://docs/{doc_id}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Vocabulary)]
pub enum RepoDocument {
    /// The agent work manual, `AGENTS.md`.
    Agents,
    /// The design, `DESIGN.md`.
    Design,
}

/// A resource of the server.
#[veoveo_types::resource_address(routes(LoreAddresses), schema = derived, wire, parse = borrowed, display)]
pub enum RepoResource {
    #[resource(template = "repo://docs")]
    Docs,
    #[resource(template = "repo://docs/{doc_id}")]
    Document(#[resource(variable = "doc_id", codec = RepoFieldCodec)] RepoDocument),
    #[resource(template = "repo://contract")]
    Contract,
}

fn route_error(error: ResourceRouteError) -> RepoResourceError {
    match error {
        ResourceRouteError::Uri(error) => RepoResourceError::Uri(error),
        _ => RepoResourceError::UnknownResource,
    }
}

struct RepoFieldCodec;

impl ResourceFieldCodec<RepoDocument> for RepoFieldCodec {
    type Error = RepoResourceError;
    fn parse(value: &str) -> Result<RepoDocument, Self::Error> {
        value
            .parse()
            .map_err(|_| RepoResourceError::UnknownResource)
    }
    fn text(value: &RepoDocument) -> Cow<'_, str> {
        value.as_str().into()
    }
}

#[doc(hidden)]
pub struct LoreAddresses;
impl veoveo_types::ResourceProfile for LoreAddresses {
    type Error = RepoResourceError;
    const PROFILE: veoveo_types::ResourceProfileSpec<Self::Error> =
        veoveo_types::ResourceProfileSpec {
            route_error: |_, error| route_error(error),
        };
}

#[cfg(test)]
mod tests {
    use super::*;
    use veoveo_types::ResourceAddress;

    fn parse(uri: &str) -> Result<RepoResource, RepoResourceError> {
        RepoResource::parse(uri)
    }

    #[test]
    fn addresses_round_trip_through_their_routes() {
        for uri in ["repo://docs", "repo://docs/agents", "repo://contract"] {
            let address = parse(uri).unwrap_or_else(|error| panic!("{uri}: {error}"));
            assert_eq!(address.to_uri().unwrap().as_str(), uri);
        }
    }

    #[test]
    fn malformed_addresses_are_refused() {
        for uri in [
            "repo://docs/unknown",
            "repo://other",
            "repo://contract/extra",
            "media://contract",
        ] {
            assert!(parse(uri).is_err(), "{uri}");
        }
    }
}
