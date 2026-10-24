# Inventario de hardware

El backend consulta proveedores CIM/WMI de Windows y enumera adaptadores con DXGI. El firmware y los controladores determinan el detalle que esos proveedores devuelven. Una propiedad vacía queda pendiente. [Clases de hardware CIM/WMI](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/computer-system-hardware-classes) · [Descripción de adaptadores DXGI](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/ns-dxgi-dxgi_adapter_desc1)

| Grupo | Qué lee la app |
| --- | --- |
| CPU | Modelo, fabricante, núcleos, hilos, frecuencia máxima y caché cuando el proveedor los informa. |
| Placa y BIOS | Fabricante, producto y revisión de la placa. Fabricante y versión del firmware. Ranuras declaradas por Windows. |
| RAM | Módulos instalados, tipo SMBIOS, capacidad, ubicación, número de parte, frecuencia y voltaje configurado si están disponibles. |
| GPU | Adaptadores DXGI, memoria dedicada y datos del controlador que devuelve WMI. |
| Discos | Modelo, firmware, interfaz y capacidad. La app usa un proveedor alternativo cuando el proveedor principal no enumera discos. |
| Sistema | Modelo y tipo de equipo, edición, compilación y arquitectura de Windows. |

La revisión detectada se usa para identificar perfiles que la requieren. Si faltan datos de una placa, puedes seguir viendo el inventario y buscar candidatos. Las comprobaciones sin evidencia suficiente quedan pendientes.

La potencia de la fuente, el espacio útil del gabinete y la ocupación física para el montaje se confirman en el equipo. Consulta [Privacidad e inventario local](privacidad.md) antes de compartir una exportación.
