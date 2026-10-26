# Mantener el catálogo

La edición se hace en los insumos y la instantánea se regenera a partir de ellos.

| Archivo | Qué contiene |
| --- | --- |
| [boards.json](../catalog/boards.json) | Perfiles por fabricante, modelo y revisión, fuentes de soporte, topología y especificaciones curadas. |
| [seed-products.json](../catalog/seed-products.json) | Fichas curadas con un identificador estable y un enlace oficial. |
| [import_catalog.py](../scripts/import_catalog.py) | Validación e importación de listas ASUS, comparativa Intel, referencias GeForce y tarjetas ASUS. |
| [snapshot.json](../catalog/snapshot.json) | Instantánea versionada que se integra en el ejecutable. |

## Actualizar una ficha

1. Confirma fabricante, nombre exacto, SKU o revisión y conserva el enlace HTTPS a la ficha oficial.
2. Distingue conector físico y carriles eléctricos. Para modos según CPU o ranuras compartidas con M.2, registra la condición y su fuente.
3. Deja los campos desconocidos sin valor. Para modelos de referencia, conserva esa distinción aunque exista una variante de ensamblador con nombre parecido.
4. Regenera el catálogo y revisa diferencias de cobertura, identificadores y filas de BIOS.

## Regenerar la instantánea

Requiere Python 3.11 y `pdftotext` de Poppler en el PATH. Ejecuta estos comandos desde la raíz. Usa la fecha que quieras asignar a la nueva instantánea.

~~~powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r scripts/requirements.txt
$fechaCatalogo = Get-Date -Format yyyy-MM-dd
.\.venv\Scripts\python.exe scripts/import_catalog.py --date $fechaCatalogo
cargo test --locked
~~~

El importador verifica los dominios, los redireccionamientos, la identidad de los modelos y las tablas esperadas. La instantánea se sustituye al terminar una importación válida. Para fuentes de fabricante con acceso automatizado restringido, usa fichas curadas y conserva su procedencia.
