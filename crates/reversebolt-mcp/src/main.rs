use reversebolt_core::Endpoint;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};

struct Reversebolt {
    directory: PathBuf,
    client: reqwest::Client,
}

fn tools() -> Vec<Tool> {
    [
        ("status","Read the desktop's active session, observations, candidate evidence, edits and recent history.",json!({}),vec![]),
        ("open_practice","Start a fresh disposable practice process. Replaces the current practice. Labels and numeric observations are provided by its adapter; this is not game recognition.",json!({"seed":{"type":"integer","minimum":1,"description":"Optional reproducible random layout seed."}}),vec![]),
        ("observe","Advance the practice through 20 state changes and capture synchronized observations and process memory. Clears prior hypotheses.",json!({}),vec![]),
        ("discover","Infer numeric mappings from captured bytes and observations. Held-out matches are hypotheses, not established controls.",json!({}),vec![]),
        ("test_candidates","Probe practice matches with two interventions, check both observations, then restore the paused practice arena. This is restricted to the disposable practice environment.",json!({}),vec![]),
        ("set_value","Change one uniquely tested practice control and verify the observed effect. Use status for track indices; track 0 is the practice Health observation and 1 is Ammunition.",json!({"track":{"type":"integer","minimum":0,"maximum":1},"value":{"type":"number","minimum":-1000000,"maximum":1000000}}),vec!["track","value"]),
        ("undo","Restore the most recent edit in the current practice only if its bytes have not changed since the edit.",json!({}),vec![]),
        ("history","Read the shared session including recorded actions and patch history.",json!({}),vec![]),
        ("close_target","Close the disposable practice process and invalidate its controls. Keeps history.",json!({}),vec![]),
    ].into_iter().map(|(name,description,properties,required)| {
        let schema = json!({"type":"object","properties":properties,"required":required,"additionalProperties":false});
        let mut annotations = ToolAnnotations::new();
        annotations.read_only_hint = Some(matches!(name,"status"|"history"));
        annotations.open_world_hint = Some(false);
        Tool::new(format!("reversebolt_{name}"),description,schema.as_object().unwrap().clone())
            .with_annotations(annotations)
    }).collect()
}

impl ServerHandler for Reversebolt {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("reversebolt",env!("CARGO_PKG_VERSION")))
            .with_instructions("Reversebolt shares the desktop engine and history. Open the desktop app first. This preview supports a disposable numeric practice process, not arbitrary games. Treat candidate correlations as hypotheses. Observe, discover, test, inspect ambiguity, then edit.")
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tools().into_iter().find(|t| t.name == name)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let Some(action) = request
            .name
            .strip_prefix("reversebolt_")
            .filter(|_| self.get_tool(&request.name).is_some())
        else {
            return Err(ErrorData::invalid_params("Unknown Reversebolt tool.", None));
        };
        let result = self
            .command(action, Value::Object(request.arguments.unwrap_or_default()))
            .await;
        let response = match result {
            Ok(value) if value["ok"] == true => {
                CallToolResult::success(vec![ContentBlock::text(value.to_string())])
            }
            Ok(value) => CallToolResult::error(vec![ContentBlock::text(value.to_string())]),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error)]),
        };
        Ok(response.into())
    }
}

impl Reversebolt {
    async fn command(&self, action: &str, args: Value) -> Result<Value, String> {
        let bytes = std::fs::read(self.directory.join("endpoint.json")).map_err(|_| {
            "Open Reversebolt first. Its desktop engine must be running.".to_string()
        })?;
        let endpoint: Endpoint = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let url = reqwest::Url::parse(&endpoint.url).map_err(|e| e.to_string())?;
        if url.scheme() != "http" || url.host_str() != Some("127.0.0.1") {
            return Err("The engine endpoint must be on the local loopback interface.".into());
        }
        self.client
            .post(format!("{}/command", endpoint.url))
            .bearer_auth(endpoint.token)
            .json(&json!({"action":action,"args":args,"source":"agent"}))
            .send()
            .await
            .map_err(|e| format!("Could not reach the desktop engine: {e}"))?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let directory = args
        .iter()
        .position(|s| s == "--data-dir")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(reversebolt_core::data_dir);
    let server = Reversebolt {
        directory,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()?,
    };
    server
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
