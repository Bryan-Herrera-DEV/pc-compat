# PC Compat

Aplicación de escritorio para Windows x64 construida con Tauri 2 y Rust. Detecta el hardware del equipo, busca candidatos en un catálogo de fabricantes y emite un dictamen por comprobación con su evidencia.

## Usar e instalar

El instalador NSIS se genera en `target/release/bundle/nsis/`. Instala para el usuario actual y usa Microsoft Edge WebView2. La aplicación y el catálogo funcionan localmente después de la instalación. Los enlaces a fichas se abren en el navegador cuando el usuario los solicita. [Instaladores Windows de Tauri](https://v2.tauri.app/distribute/windows-installer/)

1. En **Mi equipo**, revisa CPU, placa y BIOS, RAM por ranura, GPU, VRAM y discos. El inventario usa CIM/WMI y DXGI. Se puede copiar como JSON.
2. En el primer inicio, después del escaneo de un equipo de escritorio, puedes introducir la potencia nominal de la fuente y las medidas útiles para una tarjeta gráfica. Todos los campos son opcionales. Puedes corregirlos en **Datos de instalación**. Se guardan en la WebView del equipo y se reutilizan en las evaluaciones de GPU.
3. En **Catálogo**, abre RAM, gráficas, almacenamiento, placas base o procesadores desde el resumen o desde las pestañas. Busca por nombre, marca o SKU. El filtro de CPU muestra filas oficiales de la placa y el de gráficas distingue fichas con medidas y requisitos eléctricos. Completa los datos de instalación y revisa las comprobaciones enlazadas.
4. En **Cobertura**, consulta la fecha del catálogo, los perfiles y sus listas oficiales.

## Cómo se llega a un dictamen

| Componente | Evidencia |
| --- | --- |
| CPU | Socket de la placa, fila del CPU exacto en la lista del fabricante, revisión de PCB y BIOS cuando se pueden comparar de forma segura. |
| RAM | Generación DDR, formato, ranuras ocupadas, capacidad de placa y SMBIOS cuando están disponibles. La confirmación de QVL y la velocidad efectiva se evalúan por separado. |
| GPU | Forma física y ancho eléctrico de la ranura, modo según CPU, recursos compartidos y generación común con la tarjeta. La ficha del SKU determina medidas, potencia recomendada y cableado. El enlace negociado y la ocupación física se verifican en el equipo. |
| Placa base | Cruce de CPU y RAM instalados con la ficha candidata. El BIOS y la revisión de la placa candidata se comprueban en esa unidad. El gabinete y los conectores ATX/EPS requieren inspección. |
| SSD | Formatos NVMe M.2 2230, 2242 y 2280 o SATA 2,5″, topología publicada cuando está disponible y confirmación de espacio y ocupación física. |

En equipos sin perfil exacto, Windows sigue aportando inventario y el motor aplica las comprobaciones que tienen evidencia suficiente. La ausencia de un modelo en una lista importada da **Faltan datos**, porque algunas fuentes publican la tabla paginada y el importador registra las filas accesibles. La recomendación de vatios se mantiene como condición. La falta de espacio frente a las medidas de un SKU exacto sí constituye un bloqueo.

Las revisiones con topologías distintas requieren perfiles independientes. La app incluye perfiles de ASUS, ASRock, GIGABYTE y MSI. Los cruces automáticos de BIOS provienen de ASUS. Para ASRock, GIGABYTE y MSI se incluyen especificaciones y enlaces oficiales de soporte. Aún faltan perfiles específicos para la mayoría de OEM y portátiles. El catálogo incluye placas LGA775, LGA1155, LGA1150, AM2, AM3+ y FM2+, procesadores Pentium, Celeron, Athlon, Phenom y FX, y gráficas desde ATI Radeon HD 5450 y GeForce GT 710 hasta Radeon, GeForce y Intel Arc recientes. El catálogo registra modelos históricos y actuales sin información de precios o stock regional.

## Estructura

| Ruta | Responsabilidad |
| --- | --- |
| `ui/` | Interfaz Tauri en HTML, CSS y JavaScript. Solicita inventario y evaluaciones a Rust. |
| `src/main.rs` | IPC de Tauri, estado de inventario local y apertura de URLs presentes en el catálogo. |
| `src/inventory.rs`, `src/inventory.ps1` | Lectura de CIM/WMI y DXGI. |
| `src/compat.rs` | Motor de evaluación. Distingue evidencia, condiciones, datos pendientes y bloqueos. |
| `catalog/boards.json`, `catalog/seed-products.json` | Perfiles y fichas curadas de fabricantes. |
| `scripts/import_catalog.py` | Importación de tablas CPU de ASUS, comparativa Intel de escritorio, catálogo GeForce de NVIDIA y fichas de tarjetas ASUS. |
| `catalog/snapshot.json` | Catálogo versionado que se incluye en el ejecutable. |

La WebView usa contenido local y una política de contenido que limita los orígenes. El comando para abrir fuentes exige HTTPS y coincidencia con un enlace incorporado. El motor recibe el SKU y los datos manuales de instalación, y usa el inventario leído por Rust.

## Actualizar el catálogo

Con Python 3.11 y `pdftotext` de Poppler disponible en PATH, desde la raíz del proyecto.

```powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r scripts/requirements.txt
.\.venv\Scripts\python.exe scripts/import_catalog.py --date 2026-11-06
cargo test --locked
```

Cambia la fecha al publicar otra instantánea. El importador usa una lista de dominios oficiales, comprueba la identidad del modelo y la estructura de las tablas, y sustituye la instantánea solo al finalizar una importación válida. Los fabricantes con acceso automatizado restringido se mantienen en las fichas curadas. Revisa el diff de cobertura y las variantes ambiguas antes de empaquetar. Los usuarios reciben la instantánea incluida en la versión que instalen.

Fuentes de importación iniciales. [ASUS CPU Support](https://www.asus.com/us/motherboards-components/motherboards/tuf-gaming/tuf-gaming-b560m-plus-wifi/helpdesk_cpu?model2Name=TUF-GAMING-B560M-PLUS-WIFI) · [Comparativa Intel](https://cdrdv2-public.intel.com/841923/Intel-Core-Desktop-Boxed-Processors-Comparison-Chart.pdf) · [Comparativa NVIDIA](https://www.nvidia.com/en-us/geforce/graphics-cards/compare/) · [Kingston ValueRAM](https://www.kingston.com/en/memory) · [Fichas SSD Samsung](https://semiconductor.samsung.com/consumer-storage/support/documents/)

## Desarrollar y empaquetar

En Windows, instala el compilador MSVC y el Windows SDK, Rust mediante rustup y Node.js. El proyecto fija la versión de Rust. [Requisitos oficiales de Tauri](https://v2.tauri.app/start/prerequisites/)

```powershell
npm ci
npm run dev
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked scan_windows -- --ignored --nocapture
npm run build -- --ci --no-sign
```

El último comando genera un instalador local sin firma. Para una publicación pública, valida una instalación limpia en Windows 10/11 y prepara la firma del ejecutable y del instalador.

