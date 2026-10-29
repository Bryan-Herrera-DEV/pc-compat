# Validar un cambio

Ejecuta los siguientes comandos desde la raíz del proyecto.

~~~powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
node --check ui/app.js
~~~

La suite de Rust comprueba la integridad del catálogo, los cruces de BIOS, RAM, PCIe y M.2, y el paso de datos por los comandos Tauri. Ejecuta por separado la prueba de inventario real en Windows.

~~~powershell
cargo test --locked scan_windows -- --ignored
~~~

## Revisión manual

- Abre la app y comprueba que el inventario termina aunque algún proveedor devuelva una advertencia.
- Guarda una medida en el inicio, reinicia y verifica que la ficha de GPU la recupera. Prueba también omitirla en una sesión nueva.
- Busca un SKU exacto y prueba una ranura con el mismo conector físico y menos carriles eléctricos.
- Verifica que un perfil de placa desconocido deja pendientes los cruces que dependen de él.
- Abre una fuente desde el dictamen y comprueba que lleva a la página del modelo correcto.
- Si hay cambios de interfaz, revisa navegación por teclado, texto del dictamen, estados vacíos y distribución en ventana estrecha.

[Revisión de interfaz 0.3.0](../design-qa.md)
