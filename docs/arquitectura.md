# Arquitectura

PC Compat es una aplicación de escritorio Tauri 2 con backend en Rust y una interfaz local en HTML, CSS y JavaScript.

~~~mermaid
flowchart LR
    A[CIM / WMI y DXGI] --> B[Inventario en Rust]
    C[Fichas de fabricantes] --> D[Importador de catálogo]
    D --> E[Instantánea incluida]
    B --> F[Motor de compatibilidad]
    E --> F
    G[WebView Tauri] -->|ID de la pieza y datos físicos| F
    F -->|Dictamen y fuentes| G
~~~

| Módulo | Responsabilidad |
| --- | --- |
| [Interfaz](../ui/app.js) | Inventario, búsqueda, navegación y datos manuales de instalación. |
| [IPC](../src/main.rs) | Carga del catálogo, escaneo, evaluación y apertura controlada de fuentes oficiales. |
| [Inventario](../src/inventory.rs) | Lectura de Windows y DXGI. El escaneo se ejecuta en una tarea bloqueante fuera del hilo de interfaz. |
| [Modelo del catálogo](../src/catalog.rs) | Tipos de datos y resolución de perfiles según fabricante, modelo y revisión. |
| [Compatibilidad](../src/compat.rs) | Reglas y niveles por comprobación. Incluye módulos para [PCIe](../src/compat/pcie.rs) y [placas base](../src/compat/motherboard.rs). |
| [Configuración Tauri](../tauri.conf.json) | WebView, política de contenido y empaquetado de Windows. |

El backend conserva el último inventario válido en memoria durante la sesión y evalúa con ese inventario. La interfaz envía el identificador de la pieza y los datos físicos que proporciona la persona usuaria. El comando para abrir una fuente exige HTTPS y coincidencia con una URL reconocida por el catálogo.
