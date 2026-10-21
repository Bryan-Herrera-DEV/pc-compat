use super::*;
use serde_json::json;
use std::collections::HashSet;

fn system(vendor: &str, model: &str, version: &str, cpu: &str) -> Inventory {
    Inventory {
        board: vec![json!({"Manufacturer": vendor, "Product": model, "Version": version})],
        cpu: vec![json!({"Name": cpu})],
        bios: vec![json!({"SMBIOSBIOSVersion": "2001"})],
        memory: vec![json!({"Capacity": 16 * GIB, "SMBIOSMemoryType": 26}); 2],
        memory_arrays: vec![json!({"Use": 3, "MaxCapacityEx": 64 * 1024 * 1024})],
        ..Default::default()
    }
}

fn b560() -> Inventory {
    system(
        "ASUSTeK COMPUTER INC.",
        "TUF GAMING B560M-PLUS WIFI",
        "Rev 1.xx",
        "Intel(R) Core(TM) i7-10700 CPU @ 2.90GHz",
    )
}

fn check(catalog: &Catalog, id: &str, inventory: &Inventory, inputs: &Inputs) -> Report {
    evaluate(
        catalog,
        inventory,
        catalog.find_product(id).expect("catalog fixture"),
        inputs,
    )
}

#[test]
fn bundled_catalog_ids_and_board_relations_are_consistent() {
    let catalog = Catalog::bundled().unwrap();
    let ids: HashSet<_> = catalog
        .products
        .iter()
        .map(|part| part.id.as_str())
        .collect();
    assert_eq!(ids.len(), catalog.products.len());
    assert!(ids.len() > 100);
    assert!(catalog.boards.len() >= 10);
    for (index, board) in catalog.boards.iter().enumerate() {
        assert_eq!(catalog.products.iter().filter(|product|
            matches!(product.part, Part::Motherboard { board_index } if board_index == index)
        ).count(), 1);
        let mut slot_names = HashSet::new();
        assert!(board.pcie_source.is_some() && !board.pcie_slots.is_empty());
        for slot in &board.pcie_slots {
            assert!(slot_names.insert(&slot.name));
            assert!(!slot.modes.is_empty());
            assert!(slot
                .modes
                .iter()
                .all(|mode| mode.lanes.is_none_or(|lanes| lanes <= slot.connector_lanes)));
        }
        assert!(!board.m2_known_absent || board.m2_slots.is_empty());
        for entry in &board.cpu_support {
            let product = catalog.find_product(&entry.product_id).unwrap();
            assert!(matches!(&product.part, Part::Cpu { socket, .. }
                if socket == &board.socket || motherboard::legacy_socket_pair(socket, &board.socket)));
        }
    }
}

#[test]
fn cpu_rules_are_specific_to_board_and_do_not_assume_missing_qvl_rows() {
    let catalog = Catalog::bundled().unwrap();
    let inputs = Inputs {
        cooler: Confirmation::Yes,
        ..Default::default()
    };
    let mut intel = b560();
    assert_eq!(
        check(&catalog, "i7-11700", &intel, &inputs).level,
        Level::Pass
    );
    intel.bios[0]["SMBIOSBIOSVersion"] = json!("0403");
    assert_eq!(
        check(&catalog, "i7-11700", &intel, &inputs).level,
        Level::Conditional
    );
    assert_eq!(
        check(&catalog, "ryzen-7-9700x", &intel, &inputs).level,
        Level::Fail
    );
    let amd = system(
        "ASUS",
        "TUF GAMING B650-PLUS WIFI",
        "Rev 1.xx",
        "AMD Ryzen 5 7600",
    );
    let report = check(&catalog, "ryzen-7-9700x", &amd, &inputs);
    assert_ne!(report.level, Level::Fail);
    assert!(report
        .checks
        .iter()
        .any(|check| check.title == "Validación de BIOS y PCB"
            || check.title == "CPU y BIOS"
            || check.title == "Actualizar BIOS"));
    let unlisted = system("Dell Inc.", "0A000", "A02", "Intel");
    assert_eq!(
        check(&catalog, "i7-11700", &unlisted, &inputs).level,
        Level::Unknown
    );
}

#[test]
fn revision_must_match_on_gigabyte_profile() {
    let catalog = Catalog::bundled().unwrap();
    let model = "B650 AORUS ELITE AX";
    assert!(catalog
        .find_board("Gigabyte Technology Co., Ltd.", model, Some("Rev 1.2"))
        .is_some());
    assert!(catalog.find_board("GIGABYTE", model, Some("1.1")).is_none());
    assert!(catalog.find_board("GIGABYTE", model, None).is_none());
}

#[test]
fn m2_topology_checks_slot_and_actual_cpu() {
    let catalog = Catalog::bundled().unwrap();
    let mut system = b560();
    let mut inputs = Inputs {
        m2_slot: Some("M.2_1".into()),
        storage_install: Confirmation::Yes,
        ..Default::default()
    };
    assert_eq!(
        check(&catalog, "mz-v9p2t0", &system, &inputs).level,
        Level::Fail
    );
    inputs.m2_slot = Some("M.2_2".into());
    assert_eq!(
        check(&catalog, "mz-v9p2t0", &system, &inputs).level,
        Level::Conditional
    );
    system.cpu[0]["Name"] = json!("11th Gen Intel(R) Core(TM) i7-11700");
    inputs.m2_slot = Some("M.2_1".into());
    assert_eq!(
        check(&catalog, "mz-v9p2t0", &system, &inputs).level,
        Level::Pass
    );
    system.cpu.clear();
    assert_eq!(
        check(&catalog, "mz-v9p2t0", &system, &inputs).level,
        Level::Unknown
    );
}

#[test]
fn compact_ssds_check_the_exact_mounting_length() {
    let catalog = Catalog::bundled().unwrap();
    let inventory = b560();
    let inputs = Inputs {
        m2_slot: Some("M.2_2".into()),
        storage_install: Confirmation::Yes,
        ..Default::default()
    };
    let compact = check(&catalog, "snv3sm3/1t0", &inventory, &inputs);
    assert_eq!(compact.level, Level::Fail);
    assert!(compact
        .checks
        .iter()
        .any(|check| check.detail.contains("M.2 2230")));
    let micro = check(&catalog, "cssd-f1000gbmp600mcr2", &inventory, &inputs);
    assert_eq!(micro.level, Level::Conditional);
    assert!(micro
        .checks
        .iter()
        .any(|check| check.detail.contains("NVMe 2242")));
}

#[test]
fn laptop_memory_and_partner_graphics_keep_their_own_requirements() {
    let catalog = Catalog::bundled().unwrap();
    let desktop = system(
        "ASUS",
        "TUF GAMING B650-PLUS WIFI",
        "Rev 1.xx",
        "AMD Ryzen 5 7600",
    );
    let ram = check(&catalog, "kf556s40ibk2-32", &desktop, &Inputs::default());
    assert_eq!(ram.level, Level::Fail);
    assert!(ram
        .checks
        .iter()
        .any(|check| check.detail.contains("SODIMM")));
    let inputs = Inputs {
        gpu_length: Some(235.0),
        ..Default::default()
    };
    let graphics = check(&catalog, "g5070-12v2c", &Inventory::default(), &inputs);
    assert_eq!(graphics.level, Level::Fail);
    assert!(graphics
        .checks
        .iter()
        .any(|check| check.detail.contains("236 mm")));
}

#[test]
fn memory_uses_exact_platform_and_detected_occupancy() {
    let catalog = Catalog::bundled().unwrap();
    let mut intel = b560();
    let inputs = Inputs::default();
    assert_eq!(
        check(&catalog, "cmk32gx5m2b5600c36", &intel, &inputs).level,
        Level::Fail
    );
    assert_eq!(
        check(&catalog, "cmk32gx4m2e3200c16", &intel, &inputs).level,
        Level::Conditional
    );
    intel.memory.push(json!({"Capacity": 16 * GIB}));
    assert_eq!(
        check(&catalog, "cmk32gx4m2e3200c16", &intel, &inputs).level,
        Level::Fail
    );
    let unknown = system("OEM", "UNKNOWN", "", "Intel");
    assert_eq!(
        check(&catalog, "cmk32gx5m2b5600c36", &unknown, &inputs).level,
        Level::Unknown
    );
}

#[test]
fn gpu_fit_checks_work_without_a_board_profile() {
    let catalog = Catalog::bundled().unwrap();
    let system = Inventory::default();
    let mut inputs = Inputs {
        gpu_length: Some(248.0),
        ..Default::default()
    };
    assert_eq!(
        check(&catalog, "dual-rtx5070-o12g", &system, &inputs).level,
        Level::Fail
    );
    inputs.gpu_length = Some(250.0);
    inputs.gpu_height = Some(130.0);
    inputs.gpu_thickness = Some(50.6);
    inputs.psu_w = Some(750);
    inputs.gpu_slot = Confirmation::Yes;
    inputs.gpu_cable = Confirmation::Yes;
    assert_eq!(
        check(&catalog, "dual-rtx5070-o12g", &system, &inputs).level,
        Level::Conditional
    );
    assert_eq!(
        check(&catalog, "arc-b580", &system, &inputs).level,
        Level::Unknown
    );
}

#[test]
fn pcie_checks_physical_connector_and_electrical_limit_independently() {
    let catalog = Catalog::bundled().unwrap();
    let mut inputs = Inputs {
        pcie_slot: Some("PCIEX16_2".into()),
        ..Default::default()
    };
    let id = "msi-gt-710-2gd3h-lp";
    let report = check(&catalog, id, &b560(), &inputs);
    assert!(report
        .checks
        .iter()
        .any(|row| row.title == "Cruce tarjeta y placa"
            && row.level == Level::Conditional
            && row.detail.contains("Gen2, hasta ×4")));

    inputs.pcie_slot = Some("PCIEX1".into());
    assert_eq!(check(&catalog, id, &b560(), &inputs).level, Level::Fail);
    assert_ne!(
        check(&catalog, "msi-gt-710-2gd3h-4hdmi", &b560(), &inputs).level,
        Level::Fail
    );

    inputs.pcie_slot = Some("PCIEX16_1".into());
    let mut intel = b560();
    let report = check(&catalog, "arc-a310", &intel, &inputs);
    assert!(report
        .checks
        .iter()
        .any(|row| row.title == "Cruce tarjeta y placa" && row.detail.contains("Gen3, hasta ×8")));
    intel.cpu[0]["Name"] = json!("Intel(R) Core(TM) i7-11700 CPU @ 2.50GHz");
    let report = check(
        &catalog,
        "asrock-rx-6400-challenger-itx-4gb",
        &intel,
        &inputs,
    );
    assert!(report
        .checks
        .iter()
        .any(|row| row.title == "Cruce tarjeta y placa" && row.detail.contains("Gen4, hasta ×4")));
    assert!(report
        .checks
        .iter()
        .any(|row| row.title == "Alimentación auxiliar" && row.level == Level::Pass));
}

#[test]
fn legacy_cpu_board_and_storage_follow_published_evidence() {
    let catalog = Catalog::bundled().unwrap();
    let mut old = system(
        "ASUSTeK COMPUTER INC.",
        "P8H61-M LX3 PLUS R2.0",
        "Rev X.0x",
        "Intel(R) Pentium(R) CPU G2020 @ 2.90GHz",
    );
    old.memory = vec![json!({"Capacity": 4 * GIB, "SMBIOSMemoryType": 24})];
    let inputs = Inputs {
        cooler: Confirmation::Yes,
        board_fit: Confirmation::Yes,
        ..Default::default()
    };
    let cpu = check(&catalog, "pentium-g2020", &old, &inputs);
    assert!(cpu
        .checks
        .iter()
        .any(|row| row.title == "CPU y BIOS" && row.level == Level::Pass));
    let same = check(&catalog, "board-asus-p8h61-m-lx3-plus-r2-0", &old, &inputs);
    assert!(same
        .checks
        .iter()
        .any(|row| row.title == "Procesador y BIOS de la placa candidata"
            && row.detail.contains("Pentium G2020")));
    assert_eq!(
        check(&catalog, "board-asus-prime-z790-p-wifi", &old, &inputs).level,
        Level::Fail
    );
    let storage = check(&catalog, "ct1000p310ssd2", &old, &inputs);
    assert!(storage
        .checks
        .iter()
        .any(|row| row.title == "Ranura M.2 integrada" && row.level == Level::Fail));

    old.cpu[0]["Name"] = json!("Intel(R) Core(TM) i3-3220 CPU @ 3.30GHz");
    let gen2 = check(&catalog, "arc-a310", &old, &inputs);
    assert!(gen2
        .checks
        .iter()
        .any(|row| row.title == "Cruce tarjeta y placa" && row.detail.contains("Gen2, hasta ×8")));
    old.cpu[0]["Name"] = json!("Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz");
    let gen3 = check(&catalog, "arc-a310", &old, &inputs);
    assert!(gen3
        .checks
        .iter()
        .any(|row| row.title == "Cruce tarjeta y placa" && row.detail.contains("Gen3, hasta ×8")));

    let amd = system(
        "ASUSTeK COMPUTER INC.",
        "M5A97 R2.0",
        "Rev X.0x",
        "AMD Athlon(tm) II X2 215 Processor",
    );
    let cross = check(&catalog, "athlon-ii-x2-215", &amd, &inputs);
    assert!(cross
        .checks
        .iter()
        .any(|row| row.title == "Soporte entre sockets" && row.detail.contains("AM3+")));
}

#[test]
#[ignore = "Requires Windows hardware providers"]
fn scan_windows() {
    let inventory = crate::inventory::scan().unwrap();
    assert!(!inventory.cpu.is_empty());
    assert!(!inventory.board.is_empty());
    assert!(!inventory.bios.is_empty());
    assert!(!inventory.adapters.is_empty() || !inventory.video.is_empty());
    assert!(!inventory.disks.is_empty() || !inventory.disk_fallback.is_empty());
    eprintln!("Provider warnings: {:?}", inventory.warnings);
}
