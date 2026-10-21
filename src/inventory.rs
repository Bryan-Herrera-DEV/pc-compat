use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::Command;
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ERROR_NOT_FOUND,
};

pub type Row = Value;

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Inventory {
    pub system: Vec<Row>,
    pub os: Vec<Row>,
    pub board: Vec<Row>,
    pub bios: Vec<Row>,
    pub cpu: Vec<Row>,
    pub memory: Vec<Row>,
    pub memory_arrays: Vec<Row>,
    pub video: Vec<Row>,
    pub disks: Vec<Row>,
    pub disk_fallback: Vec<Row>,
    pub slots: Vec<Row>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub adapters: Vec<Adapter>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Adapter {
    pub name: String,
    pub pci_ids: String,
    pub dedicated_bytes: u64,
    pub shared_bytes: u64,
}

pub fn scan() -> Result<Inventory, String> {
    let powershell = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .ok_or("Windows no informó la ruta SystemRoot.")?
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let output = Command::new(powershell)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
        .arg(include_str!("inventory.ps1"))
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|error| format!("No se pudo iniciar Windows PowerShell: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Falló la consulta de Windows ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let mut inventory: Inventory = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Windows devolvió un inventario ilegible: {error}"))?;
    match adapters() {
        Ok(adapters) => inventory.adapters = adapters,
        Err(error) => inventory.warnings.push(format!("DXGI: {error}")),
    }
    Ok(inventory)
}

fn adapters() -> windows::core::Result<Vec<Adapter>> {
    // The DXGI factory and adapters own their COM references. No pointers escape this call.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        let mut result = Vec::new();
        for index in 0.. {
            let adapter = match factory.EnumAdapters1(index) {
                Ok(adapter) => adapter,
                Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(error) => return Err(error),
            };
            let desc = adapter.GetDesc1()?;
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let end = desc
                .Description
                .iter()
                .position(|&unit| unit == 0)
                .unwrap_or(desc.Description.len());
            result.push(Adapter {
                name: String::from_utf16_lossy(&desc.Description[..end]),
                pci_ids: format!(
                    "VEN_{:04X} DEV_{:04X} SUBSYS_{:08X} REV_{:02X}",
                    desc.VendorId, desc.DeviceId, desc.SubSysId, desc.Revision
                ),
                dedicated_bytes: desc.DedicatedVideoMemory as u64,
                shared_bytes: desc.SharedSystemMemory as u64,
            });
        }
        Ok(result)
    }
}

pub fn field<'a>(row: &'a Row, key: &str) -> Option<&'a str> {
    row.get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn number(row: &Row, key: &str) -> Option<u64> {
    row.get(key)?.as_u64()
}
