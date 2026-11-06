//! The Lore server's library. The `contract` feature holds the server's public types; the default
//! `server` feature adds the hosted server, which shows what Lore repositories hold through the Hub, as
//! the person who asked.

#[cfg(feature = "contract")]
pub mod contract;

#[cfg(feature = "server")]
pub mod covers;
#[cfg(feature = "server")]
pub mod hub;
#[cfg(feature = "server")]
pub mod mcp;
#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "contract")]
use veoveo_types::ServerSlug;

/// The component name, taken from the crate name.
pub fn component() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/// The server's slug, which names its roles and its URI scheme. Not `lore`, which is the Lore service's own name
/// in the database.
#[cfg(feature = "contract")]
pub fn server() -> ServerSlug {
    ServerSlug::parse("lore-api").expect("the server's slug")
}

#[cfg(test)]
mod tests {
    #[test]
    fn component_is_the_crate_name() {
        assert_eq!(super::component(), "veoveo-mcp-lore");
    }
}
