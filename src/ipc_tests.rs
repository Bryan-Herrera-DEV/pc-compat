use super::*;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;

fn request(command: &str, body: Value) -> InvokeRequest {
    InvokeRequest {
        cmd: command.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.into(),
    }
}

#[test]
fn catalog_and_verdict_cross_the_tauri_command_boundary() {
    let app = mock_builder()
        .manage(AppState {
            catalog: Catalog::bundled().unwrap(),
            inventory: Arc::new(Mutex::new(Some(Inventory::default()))),
        })
        .invoke_handler(tauri::generate_handler![get_catalog, evaluate_upgrade])
        .build(mock_context(noop_assets()))
        .unwrap();
    let view = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(&view, request("get_catalog", json!({}))).unwrap();
    let InvokeResponseBody::Raw(bytes) = response else {
        panic!("catalog must be a binary IPC response");
    };
    let catalog: Catalog = serde_json::from_slice(&bytes).unwrap();
    assert!(catalog.find_product("dual-rtx5070-o12g").is_some());

    let report = get_ipc_response(
        &view,
        request(
            "evaluate_upgrade",
            json!({"id": "dual-rtx5070-o12g", "inputs": {"gpu_length": 248}}),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert_eq!(report["level"], "fail");
    assert_eq!(report["checks"][0]["title"], "Longitud libre");
}
