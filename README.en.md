# PC Compat

[Leer en español](README.md) · [Documentation in Spanish](docs/README.md)

**PC Compat is a Windows PC hardware compatibility checker and local hardware inventory tool.** Built with Rust and Tauri, it helps you evaluate a PC build or upgrade against official manufacturer specifications.

- Scan the installed CPU, motherboard, BIOS, RAM, graphics adapters and storage.
- Search the offline parts catalog by model, brand or SKU. It includes legacy Pentium, Athlon and DDR2 hardware alongside newer processors, GPUs and NVMe SSDs.
- Check motherboard CPU support and BIOS requirements, DDR memory type and capacity, and PCIe slot size, link width and generation.
- Review each result with its supporting source. Unknown firmware, undocumented board revisions and physical measurements remain pending.

## Get started

Use a Windows x64 installer from an authorized project channel. Open **Resumen** to scan the machine, then **Catálogo** to select a candidate part. On desktop systems, you can save optional power-supply and case-clearance measurements. The application UI is in Spanish. [Installation guide](docs/instalacion.md) · [Usage guide](docs/uso.md)

For authorized development, open PowerShell in the project root and run:

~~~powershell
npm ci
npm run dev
~~~

[Development](docs/desarrollo.md) · [Compatibility rules](docs/compatibilidad.md) · [Catalog and sources](docs/catalogo.md) · [Contributing](CONTRIBUTING.md)

## License

PC Compat is proprietary software. The license permits installing and running binaries from authorized channels and reserves copying and redistribution rights subject to its stated terms. [License](LICENSE.md)
