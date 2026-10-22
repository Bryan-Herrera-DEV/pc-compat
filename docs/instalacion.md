# Instalar PC Compat en Windows

PC Compat se empaqueta como una aplicación de escritorio para Windows x64. El instalador NSIS instala para el usuario actual. La interfaz usa Microsoft Edge WebView2. Para comprobar los requisitos de la plataforma, consulta la documentación de Tauri. [Instaladores para Windows](https://v2.tauri.app/distribute/windows-installer/) · [Requisitos de Tauri](https://v2.tauri.app/start/prerequisites/)

## Desde un instalador

1. Obtén el instalador de un canal autorizado del proyecto y ejecútalo. Para la versión 0.4.0, el archivo se llama `PC Compat_0.4.0_x64-setup.exe`.
2. Completa el asistente y abre **PC Compat** desde Inicio.
3. Espera a que **Resumen** muestre **Inventario listo** y sigue la [guía de uso](uso.md).

El instalador local generado con `--no-sign` se produce sin firma. El nombre y la ubicación del paquete de compilación se documentan en [Desarrollo](desarrollo.md).

El inventario y el catálogo incluido funcionan localmente después de la instalación. Si abres una ficha oficial, la consulta se realiza en el navegador.

Para reinstalación o fallos de inicio, consulta [Solución de problemas](solucion-de-problemas.md).
