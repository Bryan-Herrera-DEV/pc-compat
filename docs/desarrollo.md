# Desarrollar PC Compat

Usa Windows x64, las herramientas de compilación de Visual Studio con MSVC y Windows SDK, Microsoft Edge WebView2, Rust mediante rustup y Node.js. El repositorio fija Rust en [rust-toolchain.toml](../rust-toolchain.toml). [Requisitos oficiales de Tauri](https://v2.tauri.app/start/prerequisites/)

Desde la raíz del proyecto, en PowerShell:

~~~powershell
npm ci
npm run dev
~~~

La ventana se abre mediante Tauri. Los comandos de inventario y dictamen requieren el backend. Abrir `ui/index.html` directamente en un navegador sirve para revisar estilos, con funciones de escritorio indisponibles.

## Generar un paquete local

~~~powershell
npm run build -- --ci --no-sign
~~~

El instalador NSIS se escribe en `target/release/bundle/nsis/`. Este comando produce un paquete local sin firma. Antes de distribuir una versión, verifica la instalación desde una cuenta de Windows de prueba y revisa los requisitos de firma del canal de distribución. [Instaladores para Windows](https://v2.tauri.app/distribute/windows-installer/)

Para revisar reglas y el escaneo, consulta [Validación](validacion.md). Para regenerar datos, consulta [Mantenimiento del catálogo](mantenimiento-catalogo.md).
