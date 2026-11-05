//! Registration uses the ready route table directly.
use super::setup::LoreContract;
use std::sync::LazyLock;
use veoveo_hub_sdk::{registration::AuthorizationDeclaration, server_contract::McpServerContract};
pub static AUTHORIZATION: LazyLock<AuthorizationDeclaration> = LazyLock::new(|| {
    let declaration = super::ROUTES.authorization();
    declaration
        .check(&crate::server(), &LoreContract::scheme())
        .expect("Files route authorization");
    declaration
});
