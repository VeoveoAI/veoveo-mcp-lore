//! The MCP surface through the SDK's in-process Hub: discovery, permission refusals, the App and the
//! skill. Reading a repository needs a Hub with Lore behind it, which the deployed probes cover.
use std::time::Duration;

use serde_json::json;
use veoveo_hub_sdk::hosting::{
    Hosted,
    testing::{self, TestHub},
};
use veoveo_mcp_lore::{
    contract::LorePermission::{self, FilesRead},
    hub::Hub,
    mcp::LoreMcp,
};

/// A Hub that grants `granted` in front of the server. Nothing behind it answers a Hub route.
fn hub(granted: &[LorePermission]) -> TestHub {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let http = reqwest::Client::builder().build().unwrap();
    let hub = Hub::new(&url::Url::parse("http://127.0.0.1:9/").unwrap(), http).unwrap();
    let domain = LoreMcp::new(hub).unwrap();
    TestHub::new(
        testing::for_domain::<LoreMcp>()
            .handler(move || Hosted::new(domain.clone()))
            .build(),
    )
    .granting(granted.iter().map(|permission| permission.name()))
}

#[tokio::test]
async fn tools_are_listed_with_their_permission() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let tools = hub(&[FilesRead]).rpc("tools/list", json!({})).await;
        let mut names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("{tools}"))
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(names, ["list_files", "thumbnail"]);

        let refused = hub(&[])
            .rpc("tools/call", json!({"name": "list_files", "arguments": {}}))
            .await;
        assert!(refused["error"].is_object(), "{refused}");
    })
    .await
    .expect("the test finished in time");
}

#[tokio::test]
async fn a_path_that_climbs_is_refused_before_any_read() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let reader = hub(&[FilesRead]);
        for (name, arguments) in [
            ("list_files", json!({"repository": "r", "path": "../other"})),
            (
                "thumbnail",
                json!({"repository": "r", "branch": "main", "path": "a/../../b"}),
            ),
        ] {
            let refused = reader
                .rpc("tools/call", json!({"name": name, "arguments": arguments}))
                .await;
            assert_eq!(refused["result"]["isError"], true, "{refused}");
        }
    })
    .await
    .expect("the test finished in time");
}

#[tokio::test]
async fn the_app_is_the_shared_gallery_with_this_servers_adapter() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let reader = hub(&[FilesRead]);
        let resources = reader.rpc("resources/list", json!({})).await;
        let app = resources["result"]["resources"]
            .as_array()
            .unwrap_or_else(|| panic!("{resources}"))
            .iter()
            .find(|resource| resource["uri"] == "ui://lore-api/repositories")
            .expect("the App is listed");
        assert_eq!(app["mimeType"], "text/html;profile=mcp-app");
        let document = reader
            .rpc(
                "resources/read",
                json!({"uri": "ui://lore-api/repositories"}),
            )
            .await;
        let html = document["result"]["contents"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{document}"));
        for needle in [
            "ui/initialize",
            "veoveoGallery",
            "list_files",
            "\"thumbnail\"",
            "ui/open-link",
            "\"view\", \"lore\"",
        ] {
            assert!(html.contains(needle), "{needle}");
        }
        let tools = reader.rpc("tools/list", json!({})).await;
        for name in ["list_files", "thumbnail"] {
            let tool = tools["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|tool| tool["name"] == name)
                .unwrap();
            assert_eq!(
                tool["_meta"]["ui"]["resourceUri"],
                "ui://lore-api/repositories"
            );
        }
    })
    .await
    .expect("the test finished in time");
}

#[tokio::test]
async fn the_skill_is_listed() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let listed = hub(&[FilesRead]).rpc("skills/list", json!({})).await;
        assert_eq!(
            listed["result"]["skills"][0]["uri"], "skill://lore-api/browse-repositories/SKILL.md",
            "{listed}"
        );
    })
    .await
    .expect("the test finished in time");
}
