# Cómo leer un dictamen

| Estado | Significado |
| --- | --- |
| Compatible en las comprobaciones realizadas | La evidencia disponible y tus confirmaciones satisfacen las comprobaciones ejecutadas. |
| Viable con condiciones | Hay una condición que resolver, como una actualización de BIOS o un límite de rendimiento. |
| Faltan datos para decidir | Una ficha, revisión, dato detectado o confirmación física sigue pendiente. |
| Incompatible con esta configuración | Una comprobación detectó un bloqueo para el montaje o la configuración evaluados. |

## Qué se cruza

- **CPU.** Socket, lista de soporte del modelo de placa, revisión de PCB y BIOS mínimo si las versiones se pueden comparar. Para algunas plataformas antiguas, una fila oficial puede documentar soporte entre sockets relacionados. La ausencia de una fila importada deja el soporte pendiente.
- **RAM.** Generación DDR, formato, capacidad y ocupación. El modo **Añadir al equipo** conserva los módulos detectados en el cálculo. La confirmación del SKU en la lista QVL y la velocidad efectiva son comprobaciones separadas.
- **GPU.** Ranura física, carriles eléctricos, modo según CPU y condiciones de ranura según la ficha. Si ambas fichas publican generación y ancho, la app calcula un techo documental. Las medidas, la potencia recomendada y el cableado dependen del SKU de la tarjeta. El enlace efectivo se negocia al instalarla.
- **Placa base.** CPU y RAM actuales frente a la placa candidata. La versión de BIOS de esa unidad, el gabinete y los conectores de la fuente requieren comprobación.
- **SSD.** Montaje NVMe M.2 2230, 2242 o 2280, o SATA de 2,5 pulgadas. El montaje directo de un NVMe se bloquea si la ficha confirma que la placa carece de M.2 integrado.

La potencia recomendada por el fabricante se compara como condición. Una dimensión publicada que excede el espacio disponible es un bloqueo para ese montaje.

[Documentación de compatibilidad entre generaciones PCIe](https://pcisig.com/are-pcie-50-products-compatible-products-built-prior-pcie-generations)
