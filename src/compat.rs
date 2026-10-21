use crate::catalog::{Board, Catalog, Part, Product, PCI_SIG, WMI};
use crate::inventory::{field, number, Inventory};
use serde::{Deserialize, Serialize};

mod motherboard;
mod pcie;
#[cfg(test)]
mod tests;

const GIB: u64 = 1 << 30;

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confirmation {
    #[default]
    Unknown,
    Yes,
    No,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Inputs {
    pub replace_memory: bool,
    pub memory_qvl: Confirmation,
    pub cooler: Confirmation,
    pub gpu_slot: Confirmation,
    pub pcie_slot: Option<String>,
    pub gpu_cable: Confirmation,
    pub psu_w: Option<u16>,
    pub gpu_length: Option<f32>,
    pub gpu_height: Option<f32>,
    pub gpu_thickness: Option<f32>,
    pub m2_slot: Option<String>,
    pub storage_install: Confirmation,
    pub board_fit: Confirmation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Pass,
    Conditional,
    Unknown,
    Fail,
}

#[derive(Serialize)]
pub struct Check {
    pub level: Level,
    pub title: &'static str,
    pub detail: String,
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct Report {
    pub level: Level,
    pub checks: Vec<Check>,
}

impl Report {
    fn new() -> Self {
        Self {
            level: Level::Pass,
            checks: Vec::new(),
        }
    }

    fn add(
        &mut self,
        level: Level,
        title: &'static str,
        detail: impl Into<String>,
        source: Option<&str>,
    ) {
        self.level = self.level.max(level);
        self.checks.push(Check {
            level,
            title,
            detail: detail.into(),
            source: source.map(str::to_owned),
        });
    }

    fn confirm(
        &mut self,
        value: Confirmation,
        title: &'static str,
        detail: impl Into<String>,
        source: Option<&str>,
    ) {
        let (level, prefix) = match value {
            Confirmation::Unknown => (Level::Unknown, "Pendiente de inspección. "),
            Confirmation::Yes => (Level::Pass, "Confirmado por ti. "),
            Confirmation::No => (Level::Fail, "El equipo no cumple según tu inspección. "),
        };
        self.add(level, title, format!("{prefix}{}", detail.into()), source);
    }
}

pub fn detected_board<'a>(inventory: &Inventory, catalog: &'a Catalog) -> Option<&'a Board> {
    let [board] = inventory.board.as_slice() else {
        return None;
    };
    catalog.find_board(
        field(board, "Manufacturer")?,
        field(board, "Product")?,
        field(board, "Version"),
    )
}

fn active_cpu(inventory: &Inventory) -> Option<&str> {
    let [cpu] = inventory.cpu.as_slice() else {
        return None;
    };
    field(cpu, "Name")
}

pub fn evaluate(
    catalog: &Catalog,
    inventory: &Inventory,
    product: &Product,
    inputs: &Inputs,
) -> Report {
    let mut report = Report::new();
    let board = detected_board(inventory, catalog);
    if let Some(board) = board {
        report.add(
            Level::Pass,
            "Placa base",
            format!("{} {}", board.manufacturer, board.model),
            Some(&board.source),
        );
    }

    match &product.part {
        Part::Cpu { socket, base_w } => {
            let Some(board) = board else {
                report.add(Level::Unknown, "Soporte de CPU y BIOS",
                    "Falta la ficha exacta del equipo, incluida la revisión cuando el fabricante la distingue. Consulta la lista de procesadores admitidos del fabricante de la placa o del equipo OEM.", None);
                return report;
            };
            let socket_source = Some(board.socket_source.as_deref().unwrap_or(&board.source));
            let matches: Vec<_> = board
                .cpu_support
                .iter()
                .filter(|entry| entry.product_id == product.id)
                .collect();
            if socket == &board.socket {
                report.add(
                    Level::Pass,
                    "Socket",
                    format!("Coincide con {socket}."),
                    socket_source,
                );
            } else if motherboard::legacy_socket_pair(socket, &board.socket) {
                if matches.is_empty() {
                    report.add(Level::Unknown, "Socket y modelo",
                        format!("La placa usa {} y la ficha del CPU indica {socket}. Busca una fila para el modelo exacto.", board.socket), Some(&board.cpu_source));
                } else {
                    report.add(
                        Level::Pass,
                        "Soporte entre sockets",
                        format!(
                            "La lista oficial de la placa {} documenta este procesador {socket}.",
                            board.socket
                        ),
                        Some(&board.cpu_source),
                    );
                }
            } else {
                report.add(
                    Level::Fail,
                    "Socket",
                    format!(
                        "La placa usa {} y la ficha del CPU indica {socket}.",
                        board.socket
                    ),
                    socket_source,
                );
                return report;
            }
            if matches.is_empty() {
                report.add(Level::Unknown, "Modelo exacto",
                    "Este CPU no aparece en las filas importadas de esta placa. Busca el modelo y stepping en la lista oficial.", Some(&board.cpu_source));
            } else {
                let any_bios = matches.iter().all(|row| {
                    row.bios.eq_ignore_ascii_case("all") && row.pcb.eq_ignore_ascii_case("all")
                });
                let minimum = matches
                    .iter()
                    .filter_map(|entry| asus_bios(&entry.bios))
                    .max();
                let minimum_known = board.manufacturer == "ASUS"
                    && minimum.is_some()
                    && matches.iter().all(|entry| {
                        asus_bios(&entry.bios).is_some() && entry.pcb.eq_ignore_ascii_case("all")
                    });
                let installed = inventory
                    .bios
                    .first()
                    .and_then(|row| field(row, "SMBIOSBIOSVersion"))
                    .and_then(asus_bios);
                match (any_bios, minimum_known, minimum, installed) {
                    (true, _, _, _) => report.add(Level::Pass, "CPU y BIOS", "El fabricante valida este modelo para todas las revisiones de PCB y BIOS listadas.", Some(&board.cpu_source)),
                    (false, true, Some(minimum), Some(actual)) if actual >= minimum => report.add(Level::Pass,
                        "CPU y BIOS", format!("Modelo listado para todas las revisiones de PCB. BIOS {actual:04} cumple el mínimo {minimum:04}."), Some(&board.cpu_source)),
                    (false, true, Some(minimum), Some(actual)) => report.add(Level::Conditional,
                        "Actualizar BIOS", format!("La versión {actual:04} es anterior a la mínima {minimum:04}. Revisa el procedimiento oficial antes de reemplazar el CPU."), Some(&board.cpu_source)),
                    _ => {
                        let entries = matches.iter().map(|row| {
                            let bios = if row.bios.is_empty() { "sin versión" } else { &row.bios };
                            let pcb = if row.pcb.is_empty() { "PCB sin dato" } else { &row.pcb };
                            row.detail.as_ref().map_or_else(
                                || format!("{bios} / {pcb}"),
                                |detail| format!("{bios} / {pcb} · {detail}"),
                            )
                        }).collect::<Vec<_>>().join(", ");
                        report.add(Level::Unknown, "Validación de BIOS y PCB",
                            format!("El fabricante lista el modelo ({entries}). Verifica la versión y revisión en la tabla y el historial de BIOS."), Some(&board.cpu_source));
                    }
                }
            }
            let power = base_w.map_or(String::new(), |w| {
                format!(" La ficha indica {w} W; confirma los límites de potencia del CPU.")
            });
            report.confirm(
                inputs.cooler,
                "Disipación y alimentación",
                format!("Verifica montaje del disipador y alimentación de CPU.{power}"),
                Some(&product.source),
            );
        }
        Part::Memory {
            ddr,
            modules,
            gib_each,
            rated_mts,
            form,
        } => {
            let kit_bytes = u64::from(*modules) * u64::from(*gib_each) * GIB;
            let installed = if inputs.replace_memory {
                &[][..]
            } else {
                &inventory.memory[..]
            };
            if let Some(board) = board {
                if *ddr != board.ddr || form != &board.form {
                    report.add(
                        Level::Fail,
                        "Tipo de memoria",
                        format!(
                            "La placa requiere DDR{} {} y el kit es DDR{ddr} {form}.",
                            board.ddr, board.form
                        ),
                        Some(&board.source),
                    );
                    return report;
                }
                report.add(
                    Level::Pass,
                    "Tipo de memoria",
                    format!("DDR{ddr} {form} coincide con la ficha de la placa."),
                    Some(&board.source),
                );
                if let Some(note) = &board.memory_note {
                    report.add(
                        Level::Conditional,
                        "Organización de la memoria",
                        note,
                        Some(&board.source),
                    );
                }
                let occupied = installed.len() + usize::from(*modules);
                if !inputs.replace_memory && installed.is_empty() {
                    report.add(
                        Level::Unknown,
                        "Ocupación",
                        "Windows no devolvió módulos. Verifica la ocupación antes de añadir RAM.",
                        Some(WMI),
                    );
                } else {
                    report.add(
                        if occupied > usize::from(board.dimms) {
                            Level::Fail
                        } else {
                            Level::Pass
                        },
                        "Ranuras DIMM",
                        format!(
                            "La instalación ocuparía {occupied} de {} ranuras.",
                            board.dimms
                        ),
                        Some(&board.source),
                    );
                }
                let current_bytes = installed.iter().try_fold(0u64, |total, row| {
                    total.checked_add(number(row, "Capacity").filter(|n| *n > 0)?)
                });
                if let (Some(max), Some(total)) = (
                    board.max_gib,
                    current_bytes.and_then(|n| n.checked_add(kit_bytes)),
                ) {
                    let firmware_limit = inventory
                        .memory_arrays
                        .iter()
                        .filter(|row| number(row, "Use") == Some(3))
                        .filter_map(|row| {
                            number(row, "MaxCapacityEx")
                                .filter(|n| *n > 0)?
                                .checked_mul(1024)
                        })
                        .min();
                    let level = if total > u64::from(max) * GIB {
                        Level::Fail
                    } else if firmware_limit.is_some_and(|limit| total > limit) {
                        Level::Unknown
                    } else {
                        Level::Pass
                    };
                    let firmware = firmware_limit.map_or(String::new(), |limit| {
                        format!(" SMBIOS declara {} GiB.", limit / GIB)
                    });
                    report.add(
                        level,
                        "Capacidad",
                        format!(
                            "Total previsto: {} GiB. Máximo de la placa: {max} GiB.{firmware}",
                            total / GIB
                        ),
                        Some(board.max_source.as_deref().unwrap_or(&board.source)),
                    );
                } else {
                    report.add(
                        Level::Unknown,
                        "Capacidad",
                        "Verifica el límite de la placa, del CPU y el límite por módulo.",
                        Some(&board.source),
                    );
                }
                if inputs.memory_qvl == Confirmation::Yes {
                    report.add(Level::Pass, "QVL de memoria", "Confirmaste el SKU, la revisión y el número de módulos en la lista oficial.", Some(&board.memory_source));
                } else {
                    report.add(
                        Level::Conditional,
                        "QVL de memoria",
                        "Verifica el SKU, revisión y población de ranuras en la lista oficial.",
                        Some(&board.memory_source),
                    );
                }
            } else {
                let observed: Vec<_> = inventory
                    .memory
                    .iter()
                    .filter_map(|row| match number(row, "SMBIOSMemoryType") {
                        Some(19) => Some(2),
                        Some(24) => Some(3),
                        Some(26) => Some(4),
                        Some(34) => Some(5),
                        _ => None,
                    })
                    .collect();
                let observation = if observed.is_empty() {
                    "Windows no informó una generación DDR utilizable.".into()
                } else if observed.iter().all(|value| *value == observed[0]) {
                    format!(
                        "Windows detectó DDR{} instalada. El kit es DDR{ddr} {form}.",
                        observed[0]
                    )
                } else {
                    "Windows reportó tipos de memoria distintos.".into()
                };
                report.add(Level::Unknown, "Perfil de memoria", format!("{observation} Confirma tipo físico, soporte y capacidad máxima en el manual de la placa o del equipo."), Some(WMI));
            }
            if !installed.is_empty() {
                report.add(Level::Conditional, "Mezcla de kits", "Si conservas la RAM, comprueba estabilidad y perfil efectivo con todos los módulos.", Some(&product.source));
            }
            report.add(Level::Conditional, "Velocidad del kit",
                format!("El fabricante publica {rated_mts} MT/s. La velocidad aplicable depende del controlador del CPU, el perfil de memoria y cuántos DIMM están instalados."), Some(&product.source));
        }
        Part::Gpu {
            pcie_gen,
            pcie_lanes,
            pcie_connector_lanes,
            psu_w,
            length_mm,
            height_mm,
            thickness_mm,
            connector,
            slot_power_only,
            reference,
        } => {
            if *reference {
                report.add(Level::Unknown, "Variante de la tarjeta",
                    "Esta ficha identifica una GPU de referencia. Para certificar espacio y alimentación hace falta el SKU exacto del ensamblador.", Some(&product.source));
            }
            let link = pcie::CardLink {
                generation: *pcie_gen,
                lanes: *pcie_lanes,
                connector_lanes: *pcie_connector_lanes,
            };
            if !pcie::evaluate(
                &mut report,
                board,
                inventory,
                inputs.pcie_slot.as_deref(),
                link,
                &product.source,
            ) {
                return report;
            }
            report.confirm(
                inputs.gpu_slot,
                "Ranura de expansión",
                "Confirma que la ranura elegida queda libre para el montaje y que hay espacio en las ranuras adyacentes.",
                board.map(|board| board.source.as_str()),
            );
            if *slot_power_only {
                report.add(
                    Level::Pass,
                    "Alimentación auxiliar",
                    "La ficha publica alimentación desde la ranura PCIe.",
                    Some(&product.source),
                );
            } else if let Some(connector) = connector {
                report.confirm(
                    inputs.gpu_cable,
                    "Cableado de alimentación",
                    format!("La tarjeta requiere {connector}."),
                    Some(&product.source),
                );
            } else if !reference {
                report.add(
                    Level::Unknown,
                    "Alimentación auxiliar",
                    "La ficha disponible no identifica un conector auxiliar para este SKU.",
                    Some(&product.source),
                );
            }
            if let Some(watts) = psu_w {
                let (level, detail) = match inputs.psu_w {
                    Some(actual) if actual >= *watts => (Level::Pass, format!("{actual} W declarados frente a {watts} W recomendados para el sistema.")),
                    Some(actual) if actual > 0 => (Level::Conditional, format!("{actual} W declarados por debajo de los {watts} W recomendados por el fabricante.")),
                    _ => (Level::Unknown, format!("Introduce la potencia nominal de la fuente. El fabricante recomienda {watts} W para el sistema.")),
                };
                report.add(
                    level,
                    "Potencia de la fuente",
                    detail,
                    Some(&product.source),
                );
            }
            for (title, available, required) in [
                ("Longitud libre", inputs.gpu_length, length_mm),
                ("Altura libre", inputs.gpu_height, height_mm),
                ("Grosor libre", inputs.gpu_thickness, thickness_mm),
            ] {
                if let Some(required) = required {
                    match available.filter(|n| n.is_finite() && *n > 0.0) {
                        Some(actual) => report.add(if actual >= *required { Level::Pass } else { Level::Fail },
                            title, format!("{actual} mm libres frente a {required} mm requeridos."), Some(&product.source)),
                        None => report.add(Level::Unknown, title, format!("La tarjeta requiere {required} mm. Mide el espacio útil con el cableado instalado."), Some(&product.source)),
                    }
                }
            }
        }
        Part::Storage {
            interface,
            pcie_gen,
        } => {
            let Some(m2_size) = interface.m2_size() else {
                if let Some(board) = board.filter(|board| board.sata == Some(true)) {
                    report.add(
                        Level::Pass,
                        "Interfaz SATA",
                        board
                            .sata_note
                            .as_deref()
                            .unwrap_or("La placa publica puertos SATA."),
                        Some(board.topology_source.as_deref().unwrap_or(&board.source)),
                    );
                } else {
                    report.add(Level::Conditional, "Interfaz SATA", "Consulta en el manual qué puertos siguen habilitados al ocupar las ranuras M.2.", board.map(|board| board.source.as_str()));
                }
                report.confirm(inputs.storage_install, "Montaje y cableado",
                    "Confirma bahía de 2,5 pulgadas, puerto habilitado, cable de datos y alimentación SATA.", Some(&product.source));
                return report;
            };
            if let Some(board) = board.filter(|board| board.m2_known_absent) {
                report.add(Level::Fail, "Ranura M.2 integrada",
                    "La ficha de la placa no incluye una ranura M.2 para almacenamiento. Este dictamen evalúa el montaje directo en la placa.",
                    Some(board.topology_source.as_deref().unwrap_or(&board.source)));
                return report;
            }
            if let Some(board) = board.filter(|board| !board.m2_slots.is_empty()) {
                let source = Some(board.topology_source.as_deref().unwrap_or(&board.source));
                let Some(slot) = board
                    .m2_slots
                    .iter()
                    .find(|slot| Some(slot.name.as_str()) == inputs.m2_slot.as_deref())
                else {
                    report.add(
                        Level::Unknown,
                        "Ranura de destino",
                        "Selecciona la ranura M.2 que vas a utilizar.",
                        source,
                    );
                    return report;
                };
                if !slot.lengths.contains(&m2_size) {
                    report.add(
                        Level::Fail,
                        "Formato M.2",
                        format!("{} no admite el formato M.2 {m2_size}.", slot.name),
                        source,
                    );
                    return report;
                }
                if let Some(family) = &slot.cpu_family {
                    match pcie::cpu_family(inventory) {
                        Some(current) if current == family => {}
                        Some(_) => {
                            report.add(
                                Level::Fail,
                                "Ranura deshabilitada",
                                format!("{} no se habilita con el CPU detectado.", slot.name),
                                source,
                            );
                            return report;
                        }
                        None => {
                            report.add(
                                Level::Unknown,
                                "Dependencia del CPU",
                                format!("Verifica si el CPU instalado habilita {}.", slot.name),
                                source,
                            );
                            return report;
                        }
                    }
                }
                report.add(
                    Level::Pass,
                    "Interfaz y formato",
                    format!(
                        "{} admite NVMe {m2_size} con un enlace de hasta PCIe {}.0.",
                        slot.name, slot.pcie_gen
                    ),
                    source,
                );
                if let Some(note) = &slot.note {
                    report.add(Level::Conditional, "Condiciones de la ranura", note, source);
                }
                if pcie_gen.is_some_and(|gen| gen > slot.pcie_gen) {
                    report.add(Level::Conditional, "Ancho de banda", format!("La ranura limita el enlace a PCIe {}.0. La velocidad publicada del SSD puede quedar limitada.", slot.pcie_gen), Some(PCI_SIG));
                }
            } else {
                report.add(Level::Conditional, "Topología M.2", format!("Comprueba en el manual el protocolo NVMe, formato M.2 {m2_size}, límites PCIe y recursos compartidos de la ranura. Windows no confirma la ocupación física."), board.map(|board| board.source.as_str()));
            }
            report.confirm(
                inputs.storage_install,
                "Montaje y ranura",
                format!("Confirma una ranura NVMe {m2_size} disponible, tornillo y espacio para el disipador."),
                Some(&product.source),
            );
        }
        Part::Motherboard { board_index } => {
            motherboard::evaluate(&mut report, catalog, inventory, *board_index, board, inputs);
        }
    }
    report
        .checks
        .sort_by_key(|check| std::cmp::Reverse(check.level));
    report
}

fn asus_bios(value: &str) -> Option<u16> {
    if value.len() == 4 && value.bytes().all(|byte| byte.is_ascii_digit()) {
        value.parse().ok().filter(|version| *version > 0)
    } else {
        None
    }
}
