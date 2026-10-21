#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod catalog;
mod compat;
mod inventory;

#[cfg(test)]
mod ipc_tests;

use catalog::Catalog;
use compat::{Inputs, Report};
use inventory::Inventory;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

struct AppState {
    catalog: Catalog,
    inventory: Arc<Mutex<Option<Inventory>>>,
}

#[derive(Serialize)]
struct Scan {
    inventory: Inventory,
    profile_index: Option<usize>,
}

#[tauri::command]
fn get_catalog() -> tauri::ipc::Response {
    tauri::ipc::Response::new(include_bytes!("../catalog/snapshot.json").to_vec())
}

#[tauri::command]
async fn scan_hardware(state: State<'_, AppState>) -> Result<Scan, String> {
    let saved = Arc::clone(&state.inventory);
    let inventory = tauri::async_runtime::spawn_blocking(move || {
        let mut saved = saved
            .lock()
            .map_err(|_| "El inventario local quedó inaccesible.")?;
        *saved = None;
        let inventory = inventory::scan()?;
        *saved = Some(inventory.clone());
        Ok::<_, String>(inventory)
    })
    .await
    .map_err(|error| format!("El análisis se interrumpió: {error}"))??;

    let profile_index = compat::detected_board(&inventory, &state.catalog).and_then(|selected| {
        state
            .catalog
            .boards
            .iter()
            .position(|board| std::ptr::eq(board, selected))
    });
    Ok(Scan {
        inventory,
        profile_index,
    })
}

#[tauri::command]
fn evaluate_upgrade(
    id: &str,
    inputs: Inputs,
    state: State<'_, AppState>,
) -> Result<Report, String> {
    let product = state
        .catalog
        .find_product(id)
        .ok_or("La referencia no está en el catálogo.")?;
    let saved = state
        .inventory
        .try_lock()
        .map_err(|_| "El inventario está ocupado. Espera a que termine la detección.")?;
    let inventory = saved
        .as_ref()
        .ok_or("Ejecuta primero la detección de hardware.")?;
    Ok(compat::evaluate(
        &state.catalog,
        inventory,
        product,
        &inputs,
    ))
}

#[tauri::command]
fn open_source(url: &str, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !url.starts_with("https://") || !state.catalog.is_source(url) {
        return Err("El enlace no pertenece a las fuentes del catálogo.".into());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string())
}

fn main() {
    let catalog = Catalog::bundled().expect("valid bundled catalog");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            catalog,
            inventory: Arc::default(),
        })
        .invoke_handler(tauri::generate_handler![
            get_catalog,
            scan_hardware,
            evaluate_upgrade,
            open_source
        ])
        .run(tauri::generate_context!())
        .expect("start PC Compat");
}
