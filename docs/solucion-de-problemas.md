# Solución de problemas

| Síntoma | Qué revisar |
| --- | --- |
| La ventana no inicia | Confirma que Windows es x64 y que WebView2 está disponible. Para diagnosticar una compilación local, inicia desde PowerShell con `npm run dev` y lee el error de la consola. |
| La detección se queda pendiente o muestra un error | Pulsa **Volver a detectar**. Abre **Sistema** y revisa las advertencias. Si trabajas desde el código, ejecuta la prueba de escaneo real de la [guía de validación](validacion.md). |
| No aparece un perfil para la placa | Revisa fabricante, modelo y revisión en **Placa y BIOS**. Contrástalos con **Cobertura**. En equipos OEM, busca la ficha del modelo completo del equipo. |
| El resultado de CPU pide revisar BIOS | Compara la versión instalada con el mínimo publicado para la revisión y el modelo exactos. Si evalúas comprar otra placa, consulta la versión de la unidad candidata. |
| Un resultado de GPU sigue pendiente | Busca el SKU del ensamblador y abre su ficha. Comprueba la ranura, el cableado y las medidas con el gabinete montado. |
| Falta una pieza en la búsqueda | Quita filtros, prueba el SKU y busca en su categoría. Si sigue ausente, usa la guía para [proponer una ficha oficial](../CONTRIBUTING.md). |

[Microsoft Edge WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
