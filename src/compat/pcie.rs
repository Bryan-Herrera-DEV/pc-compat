use super::{active_cpu, Level, Report};
use crate::catalog::{Board, PCI_SIG};
use crate::inventory::Inventory;

pub(super) struct CardLink {
    pub generation: Option<u8>,
    pub lanes: Option<u8>,
    pub connector_lanes: Option<u8>,
}

pub(super) fn evaluate(
    report: &mut Report,
    board: Option<&Board>,
    inventory: &Inventory,
    choice: Option<&str>,
    card: CardLink,
    product_source: &str,
) -> bool {
    let Some(board) = board.filter(|board| !board.pcie_slots.is_empty()) else {
        let detail = match card.generation {
            Some(generation) => format!("La tarjeta publica PCIe {generation}.0. Falta una ficha de ranuras para la placa exacta."),
            None => "Faltan los detalles del enlace de la tarjeta o de la placa exacta.".into(),
        };
        report.add(
            Level::Conditional,
            "Enlace PCIe",
            detail,
            Some(product_source),
        );
        return true;
    };

    let source = Some(board.pcie_source.as_deref().unwrap_or(&board.source));
    let slot = match choice {
        Some(name) => board.pcie_slots.iter().find(|slot| slot.name == name),
        None => board.pcie_slots.first(),
    };
    let Some(slot) = slot else {
        report.add(
            Level::Unknown,
            "Ranura PCIe",
            "Elige una ranura publicada para esta placa.",
            source,
        );
        return false;
    };
    let shape = match card.connector_lanes {
        Some(required) if required > slot.connector_lanes => {
            report.add(
                Level::Fail,
                "Conector PCIe",
                format!(
                    "{} es física ×{}; la tarjeta requiere ×{required}.",
                    slot.name, slot.connector_lanes
                ),
                source,
            );
            return false;
        }
        Some(required) => format!(
            "{} ofrece un conector físico ×{} para la tarjeta ×{required}.",
            slot.name, slot.connector_lanes
        ),
        None if slot.connector_lanes == 16 => {
            format!("{} ofrece un conector físico ×16.", slot.name)
        }
        None => {
            report.add(
                Level::Unknown,
                "Conector PCIe",
                format!(
                    "{} es física ×{}. La ficha de la tarjeta no indica el tamaño de su conector.",
                    slot.name, slot.connector_lanes
                ),
                source,
            );
            format!(
                "{} ofrece un conector físico ×{}.",
                slot.name, slot.connector_lanes
            )
        }
    };
    report.add(Level::Pass, "Ranura documentada", shape, source);

    let family = cpu_family(inventory);
    let mode = slot
        .modes
        .iter()
        .find(|mode| {
            family.is_some_and(|family| mode.cpu_families.iter().any(|entry| entry == family))
        })
        .or_else(|| slot.modes.iter().find(|mode| mode.cpu_families.is_empty()));
    let Some(mode) = mode else {
        report.add(Level::Unknown, "Modo PCIe según el procesador", "El modo de esta ranura depende del CPU; la familia detectada no coincide con los modos documentados.", source);
        return true;
    };

    let host_gen = mode
        .generation
        .map_or("generación por confirmar".into(), |gen| {
            format!("Gen{gen}")
        });
    let host_width = mode
        .lanes
        .map_or("ancho eléctrico por confirmar".into(), |lanes| {
            format!("×{lanes} eléctrico")
        });
    report.add(
        if mode.generation.is_some() && mode.lanes.is_some() {
            Level::Pass
        } else {
            Level::Conditional
        },
        "Límite de la ranura",
        format!(
            "{} publica {host_gen}, {host_width} en este modo.",
            slot.name
        ),
        source,
    );
    if let Some(note) = &slot.note {
        report.add(Level::Conditional, "Condiciones de la ranura", note, source);
    }
    let (gen, lanes) = (
        card.generation
            .zip(mode.generation)
            .map(|(card, host)| card.min(host)),
        card.lanes
            .zip(mode.lanes)
            .map(|(card, host)| card.min(host)),
    );
    if let Some(generation) = gen {
        let limit = card
            .generation
            .is_some_and(|card_gen| generation < card_gen)
            || lanes
                .zip(card.lanes)
                .is_some_and(|(actual, rated)| actual < rated);
        let width = lanes.map_or("ancho sin confirmar".into(), |lanes| {
            format!("hasta ×{lanes}")
        });
        let level = if limit || lanes.is_none() {
            Level::Conditional
        } else {
            Level::Pass
        };
        report.add(level, "Cruce tarjeta y placa",
            format!("Techo documentado del enlace: PCIe Gen{generation}, {width}. El enlace real se negocia al instalar la tarjeta."), Some(PCI_SIG));
    } else {
        report.add(Level::Conditional, "Cruce tarjeta y placa", "La generación común queda pendiente porque una ficha no la especifica para esta configuración.", Some(product_source));
    }
    true
}

pub(super) fn cpu_family(inventory: &Inventory) -> Option<&'static str> {
    let name = active_cpu(inventory)?.to_ascii_lowercase();
    if name.contains("intel") {
        return name.split_ascii_whitespace().find_map(|token| {
            let (class, model) = token.split_once('-')?;
            if !["i3", "i5", "i7", "i9"].contains(&class) {
                return None;
            }
            let digits = model.bytes().take_while(u8::is_ascii_digit).count();
            if digits == 5 {
                match model.get(..2)? {
                    "10" => Some("intel_10"),
                    "11" => Some("intel_11"),
                    "12" | "13" | "14" => Some("intel_12_14"),
                    _ => None,
                }
            } else if digits == 4 {
                match model.get(..1)? {
                    "2" => Some("intel_2"),
                    "3" if matches!(class, "i5" | "i7") => Some("intel_3"),
                    "8" | "9" => Some("intel_8_9"),
                    _ => None,
                }
            } else {
                None
            }
        });
    }
    if !name.contains("ryzen") {
        return None;
    }
    let model = name
        .split_ascii_whitespace()
        .find(|token| token.bytes().take_while(u8::is_ascii_digit).count() == 4)?;
    match model.as_bytes()[0] {
        b'7' | b'9' => Some("ryzen_7_9"),
        b'8' => Some(match model.get(..4)? {
            "8700" | "8600" | "8400" => "ryzen_8_x8",
            "8500" | "8300" => "ryzen_8_x4",
            _ => "ryzen_8",
        }),
        b'4' | b'5' if model.ends_with('g') || model.ends_with("ge") || model.ends_with("gt") => {
            Some("ryzen_am4_apu")
        }
        _ => None,
    }
}
