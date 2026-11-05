//! The server's resource addresses under the `lore-api` scheme. The route declaration
//! owns each shape and its typed parameters; the shared URI machinery owns encoding and
//! parsing, so no address is built or split by hand.
use std::{borrow::Cow, error::Error, fmt};

use veoveo_types::{ResourceFieldCodec, ResourceRouteError, ResourceUriError, Vocabulary};

/// A resource address that failed admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoreResourceError {
    Uri(ResourceUriError),
    UnknownResource,
}

impl fmt::Display for LoreResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uri(error) => error.fmt(f),
            Self::UnknownResource => {
                f.write_str("unknown or noncanonical lore-api resource address")
            }
        }
    }
}

impl Error for LoreResourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Uri(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceUriError> for LoreResourceError {
    fn from(error: ResourceUriError) -> Self {
        Self::Uri(error)
    }
}

/// A document the server embeds and serves under `lore-api://docs/{doc_id}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Vocabulary)]
pub enum LoreDocument {
    /// The agent work manual, `AGENTS.md`.
    Agents,
    /// The design, `DESIGN.md`.
    Design,
}

/// A resource of the server.
#[veoveo_types::resource_address(routes(LoreAddresses), schema = derived, wire, parse = borrowed, display)]
pub enum LoreResource {
    #[resource(template = "lore-api://docs")]
    Docs,
    #[resource(template = "lore-api://docs/{doc_id}")]
    Document(#[resource(variable = "doc_id", codec = LoreFieldCodec)] LoreDocument),
    #[resource(template = "lore-api://contract")]
    Contract,
}

fn route_error(error: ResourceRouteError) -> LoreResourceError {
    match error {
        ResourceRouteError::Uri(error) => LoreResourceError::Uri(error),
        _ => LoreResourceError::UnknownResource,
    }
}

struct LoreFieldCodec;

impl ResourceFieldCodec<LoreDocument> for LoreFieldCodec {
    type Error = LoreResourceError;
    fn parse(value: &str) -> Result<LoreDocument, Self::Error> {
        value
            .parse()
            .map_err(|_| LoreResourceError::UnknownResource)
    }
    fn text(value: &LoreDocument) -> Cow<'_, str> {
        value.as_str().into()
    }
}

#[doc(hidden)]
pub struct LoreAddresses;
impl veoveo_types::ResourceProfile for LoreAddresses {
    type Error = LoreResourceError;
    const PROFILE: veoveo_types::ResourceProfileSpec<Self::Error> =
        veoveo_types::ResourceProfileSpec {
            route_error: |_, error| route_error(error),
        };
}

#[cfg(test)]
mod tests {
    use super::*;
    use veoveo_types::ResourceAddress;

    fn parse(uri: &str) -> Result<LoreResource, LoreResourceError> {
        LoreResource::parse(uri)
    }

    #[test]
    fn addresses_round_trip_through_their_routes() {
        for uri in [
            "lore-api://docs",
            "lore-api://docs/agents",
            "lore-api://contract",
        ] {
            let address = parse(uri).unwrap_or_else(|error| panic!("{uri}: {error}"));
            assert_eq!(address.to_uri().unwrap().as_str(), uri);
        }
    }

    #[test]
    fn malformed_addresses_are_refused() {
        for uri in [
            "lore-api://docs/unknown",
            "lore-api://other",
            "lore-api://contract/extra",
            "media://contract",
        ] {
            assert!(parse(uri).is_err(), "{uri}");
        }
    }
}
