use super::{active_cpu, Inputs, Level, Report, GIB};
use crate::catalog::{Board, Catalog, Part, Product, WMI};
use crate::inventory::{number, Inventory};

pub(super) fn evaluate(
    report: &mut Report,
    catalog: &Catalog,
    inventory: &Inventory,
    candidate_index: usize,
    current: Option<&Board>,
    inputs: &Inputs,
) {
    let Some(board) = catalog.boards.get(candidate_index) else {
        report.add(
            Level::Unknown,
            "Perfil de la placa candidata",
            "La ficha no tiene un perfil de hardware asociado.",
            None,
        );
        return;
    };
    let cpu = installed_cpu(catalog, inventory);
    let rows: Vec<_> = cpu.map_or_else(Vec::new, |cpu| {
        board
            .cpu_support
            .iter()
            .filter(|entry| entry.product_id == cpu.id)
            .collect()
    });
    if let Some(cpu) = cpu.filter(|_| !rows.is_empty()) {
        let bios: Vec<_> = rows
            .iter()
            .map(|row| row.bios.as_str())
            .filter(|version| !version.is_empty() && *version != "all")
            .collect();
        let requirement = if bios.is_empty() {
            "Revisa la versión y revisión de PCB en la lista oficial.".into()
        } else {
            format!("Comprueba el BIOS mínimo publicado ({}) y la revisión de la unidad que vas a comprar.", bios.join(", "))
        };
        report.add(
            Level::Conditional,
            "Procesador y BIOS de la placa candidata",
            format!(
                "{} figura en la lista de {}. {requirement}",
                cpu.name, board.model
            ),
            Some(&board.cpu_source),
        );
    } else {
        let known_socket = cpu
            .and_then(|cpu| match &cpu.part {
                Part::Cpu { socket, .. } => Some((socket.as_str(), cpu.source.as_str())),
                _ => None,
            })
            .or_else(|| {
                current.map(|board| {
                    (
                        board.socket.as_str(),
                        board.socket_source.as_deref().unwrap_or(&board.source),
                    )
                })
            });
        match known_socket {
            Some((installed, _)) if installed != board.socket && !legacy_socket_pair(installed, &board.socket) => {
                report.add(Level::Fail, "Socket del procesador",
                    format!("La plataforma del procesador actual es {installed}. La placa candidata usa {}.", board.socket), Some(&board.source));
                if let Some((_, evidence)) = known_socket {
                    report.add(Level::Pass, "Plataforma de origen", format!("Ficha oficial de {installed}."), Some(evidence));
                }
            }
            _ => report.add(Level::Unknown, "Soporte del procesador",
                "Comprueba el modelo y stepping del procesador actual en la lista oficial de la placa candidata y la versión de BIOS de esa unidad.", Some(&board.cpu_source)),
        }
    }

    let modules = &inventory.memory;
    let generation = current.map(|board| board.ddr).or_else(|| {
        let types: Option<Vec<_>> = modules
            .iter()
            .map(|row| match number(row, "SMBIOSMemoryType") {
                Some(19) => Some(2),
                Some(24) => Some(3),
                Some(26) => Some(4),
                Some(34) => Some(5),
                _ => None,
            })
            .collect();
        let types = types?;
        let first = *types.first()?;
        types.iter().all(|ddr| *ddr == first).then_some(first)
    });
    if let Some(generation) = generation {
        let incompatible_form = board.form == "UDIMM"
            && modules
                .iter()
                .any(|row| number(row, "FormFactor") == Some(12));
        let occupied = modules.len();
        let total = modules
            .iter()
            .try_fold(0u64, |sum, row| sum.checked_add(number(row, "Capacity")?));
        let exceeds_capacity = total
            .zip(board.max_gib)
            .is_some_and(|(bytes, max)| bytes > u64::from(max) * GIB);
        let level = if generation != board.ddr
            || incompatible_form
            || occupied > usize::from(board.dimms)
            || exceeds_capacity
        {
            Level::Fail
        } else {
            Level::Conditional
        };
        report.add(level, "Reutilizar la RAM",
            format!("Windows detectó {occupied} módulos en una configuración DDR{generation}. La placa admite {} ranuras DDR{} {}{}. Revisa la lista de memoria para los SKU instalados.",
                board.dimms, board.ddr, board.form, board.max_gib.map_or(String::new(), |max| format!(" y hasta {max} GiB"))),
            Some(&board.source));
        if incompatible_form {
            report.add(
                Level::Fail,
                "Formato de la RAM",
                "Se detectó SO-DIMM y la placa candidata requiere UDIMM.",
                Some(WMI),
            );
        }
    } else {
        report.add(Level::Unknown, "RAM actual", format!("La placa candidata usa DDR{} {}. Falta identificar el tipo de memoria que quieres reutilizar.", board.ddr, board.form), Some(&board.memory_source));
    }

    let format = board
        .form_factor
        .as_deref()
        .map_or("el formato de la placa".into(), |value| {
            format!("una placa {value}")
        });
    report.confirm(inputs.board_fit, "Gabinete y alimentación de la placa",
        format!("Confirma que el gabinete admite {format} y que la fuente tiene los conectores ATX/EPS publicados."),
        Some(&board.source));
}

pub(super) fn legacy_socket_pair(left: &str, right: &str) -> bool {
    matches!(
        (left, right),
        ("AM3", "AM3+") | ("AM3+", "AM3") | ("FM2", "FM2+") | ("FM2+", "FM2")
    )
}

fn tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .filter(|token| !["r", "tm", "cpu", "processor"].contains(&token.as_str()))
        .collect()
}

fn installed_cpu<'a>(catalog: &'a Catalog, inventory: &Inventory) -> Option<&'a Product> {
    let label = tokens(active_cpu(inventory)?);
    let mut candidates: Vec<_> = catalog
        .products
        .iter()
        .filter(|product| matches!(product.part, Part::Cpu { .. }))
        .filter_map(|product| {
            let name = tokens(&product.name);
            let sku = tokens(&product.sku);
            let matches = |parts: &[String]| {
                !parts.is_empty() && label.windows(parts.len()).any(|window| window == parts)
            };
            if matches(&name) {
                Some((name.len(), product))
            } else if sku.len() >= 2 && matches(&sku) {
                Some((sku.len(), product))
            } else {
                None
            }
        })
        .collect();
    candidates.sort_unstable_by_key(|(score, _)| std::cmp::Reverse(*score));
    match candidates.as_slice() {
        [(score, _), (second, _), ..] if score == second => None,
        [(_, product), ..] => Some(product),
        [] => None,
    }
}
