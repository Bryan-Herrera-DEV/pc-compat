use serde::{Deserialize, Serialize};

pub const PCI_SIG: &str =
    "https://pcisig.com/are-pcie-50-products-compatible-products-built-prior-pcie-generations";
pub const WMI: &str =
    "https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/computer-system-hardware-classes";
pub const DXGI: &str =
    "https://learn.microsoft.com/en-us/windows/win32/api/dxgi/ns-dxgi-dxgi_adapter_desc1";

#[derive(Deserialize, Serialize)]
pub struct Catalog {
    pub schema: u32,
    pub published: String,
    pub products: Vec<Product>,
    pub boards: Vec<Board>,
}

#[derive(Deserialize, Serialize)]
pub struct Product {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub summary: String,
    pub source: String,
    #[serde(flatten)]
    pub part: Part,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum Part {
    Cpu {
        socket: String,
        base_w: Option<u16>,
    },
    Memory {
        ddr: u8,
        modules: u8,
        gib_each: u16,
        rated_mts: u16,
        form: String,
    },
    Gpu {
        pcie_gen: Option<u8>,
        pcie_lanes: Option<u8>,
        pcie_connector_lanes: Option<u8>,
        psu_w: Option<u16>,
        length_mm: Option<f32>,
        height_mm: Option<f32>,
        thickness_mm: Option<f32>,
        connector: Option<String>,
        #[serde(default)]
        slot_power_only: bool,
        #[serde(default)]
        reference: bool,
    },
    Storage {
        interface: Interface,
        pcie_gen: Option<u8>,
    },
    Motherboard {
        board_index: usize,
    },
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Interface {
    #[serde(rename = "nvme_2230")]
    Nvme2230,
    #[serde(rename = "nvme_2242")]
    Nvme2242,
    #[serde(rename = "nvme_2280")]
    Nvme2280,
    #[serde(rename = "sata_25")]
    Sata25,
}

impl Interface {
    pub fn m2_size(self) -> Option<u16> {
        match self {
            Self::Nvme2230 => Some(2230),
            Self::Nvme2242 => Some(2242),
            Self::Nvme2280 => Some(2280),
            Self::Sata25 => None,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Board {
    pub manufacturer: String,
    pub model: String,
    pub revision: Option<String>,
    pub socket: String,
    pub ddr: u8,
    pub dimms: u8,
    pub max_gib: Option<u16>,
    pub form: String,
    pub form_factor: Option<String>,
    pub source: String,
    pub max_source: Option<String>,
    pub socket_source: Option<String>,
    pub cpu_source: String,
    pub memory_source: String,
    pub memory_note: Option<String>,
    pub topology_source: Option<String>,
    pub pcie_source: Option<String>,
    #[serde(default)]
    pub pcie_slots: Vec<PcieSlot>,
    #[serde(default)]
    pub m2_slots: Vec<Slot>,
    #[serde(default)]
    pub m2_known_absent: bool,
    pub sata: Option<bool>,
    pub sata_note: Option<String>,
    #[serde(default)]
    pub cpu_support: Vec<CpuSupport>,
}

#[derive(Deserialize, Serialize)]
pub struct PcieSlot {
    pub name: String,
    pub connector_lanes: u8,
    pub modes: Vec<PcieMode>,
    pub note: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct PcieMode {
    pub generation: Option<u8>,
    pub lanes: Option<u8>,
    #[serde(default)]
    pub cpu_families: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Slot {
    pub name: String,
    pub pcie_gen: u8,
    pub lengths: Vec<u16>,
    pub cpu_family: Option<String>,
    pub note: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct CpuSupport {
    pub product_id: String,
    pub bios: String,
    pub pcb: String,
    pub detail: Option<String>,
}

impl Catalog {
    pub fn bundled() -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(include_str!("../catalog/snapshot.json"))
            .map_err(|error| format!("Catálogo ilegible: {error}"))?;
        if catalog.schema != 1 || catalog.products.is_empty() || catalog.boards.is_empty() {
            return Err("Versión o contenido de catálogo no admitido.".into());
        }
        Ok(catalog)
    }

    pub fn find_product(&self, id: &str) -> Option<&Product> {
        self.products.iter().find(|product| product.id == id)
    }

    pub fn find_board(
        &self,
        manufacturer: &str,
        model: &str,
        revision: Option<&str>,
    ) -> Option<&Board> {
        let vendor = manufacturer.trim().trim_end_matches('.');
        let vendor = if vendor.eq_ignore_ascii_case("ASUSTeK COMPUTER INC") {
            "ASUS"
        } else if vendor.eq_ignore_ascii_case("Gigabyte Technology Co., Ltd") {
            "GIGABYTE"
        } else if vendor.eq_ignore_ascii_case("Micro-Star International Co., Ltd") {
            "MSI"
        } else {
            vendor
        };
        self.boards.iter().find(|board| {
            board.manufacturer.eq_ignore_ascii_case(vendor)
                && board.model.eq_ignore_ascii_case(model.trim())
                && board.revision.as_deref().is_none_or(|required| {
                    revision.is_some_and(|value| {
                        let value = value.trim().to_ascii_lowercase();
                        value
                            .strip_prefix("rev")
                            .unwrap_or(&value)
                            .trim_start_matches(['.', ' '])
                            == required
                    })
                })
        })
    }

    pub fn is_source(&self, url: &str) -> bool {
        [PCI_SIG, WMI, DXGI].contains(&url)
            || self.products.iter().any(|item| item.source == url)
            || self.boards.iter().any(|board| {
                [
                    Some(board.source.as_str()),
                    Some(board.cpu_source.as_str()),
                    Some(board.memory_source.as_str()),
                    board.max_source.as_deref(),
                    board.socket_source.as_deref(),
                    board.topology_source.as_deref(),
                    board.pcie_source.as_deref(),
                ]
                .contains(&Some(url))
            })
    }
}
