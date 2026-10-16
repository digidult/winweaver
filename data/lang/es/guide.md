# Guía de WinWeaver

WinWeaver prepara instalaciones desatendidas de Windows 11 Pro para pequeños grupos de trabajo y mantiene
configurados los equipos instalados. Un perfil indica qué reglas del catálogo están activadas, con qué
parámetros y qué contienen los formularios de instalación; a partir de él, el programa genera `autounattend.xml`.

Esta página se abre en el primer inicio y, más adelante, desde el menú o con F1. Es un archivo Markdown;
las páginas de documentación se abren del mismo modo, cada una en su propia pestaña.

## Primeros pasos

1. Abre un perfil predefinido: **Menú, Nuevo, Perfil predefinido Office**. Office contiene los valores
   predeterminados del catálogo; Strict agrega restricciones, Laptop es adecuado para equipos portátiles y
   Home es una lista de permitidos para un equipo doméstico.
2. Recorre el árbol de la izquierda. Una casilla activa o desactiva una regla; la casilla de un grupo activa
   o desactiva todas sus reglas a la vez; una regla atenuada espera a otra regla que necesita. El panel
   derecho muestra los parámetros de la regla seleccionada y su descripción.
3. Rellena los formularios **Instalación**, **Cuentas** e **Idiomas** de la primera raíz.
4. Comprueba el perfil con F7. Los mensajes aparecen debajo de la fila de navegación; un clic en un mensaje
   selecciona la regla o el formulario al que se refiere.
5. Genera el archivo de respuesta con F9: `autounattend.xml` se guarda en la carpeta `output` junto al
   ejecutable, con los scripts que incluye al lado para que puedas revisarlos. Copia el archivo a la raíz
   de la unidad USB de instalación.
6. Guarda el perfil con Ctrl+S. Los perfiles son archivos `.wwprofile` de la carpeta `profiles` junto al
   ejecutable, o de cualquier otra ubicación que elijas con Guardar como.

## La ventana

- **Pestañas**: cada documento abierto es una pestaña: un perfil, una comparación, esta guía, los
  marcadores, el servidor MCP, la configuración. El botón de menú del extremo izquierdo de la fila de
  pestañas contiene Nuevo, Abrir, Guardar, Comparar, la guía, el servidor MCP, los marcadores,
  Configuración y Salir.
- **Barra de comandos**: Nuevo, Abrir, Guardar, Comprobar, Generar, Este equipo, Comparar. En una ventana
  estrecha solo muestra iconos, en la fila de navegación.
- **Fila de navegación**: Atrás y Adelante recorren los elementos que has seleccionado; la ruta de
  navegación muestra dónde estás y salta a cualquier nivel; Marcador marca la posición; Buscar filtra el
  árbol mientras escribes y recuerda lo que has buscado.
- **Árbol y panel de detalles**: el árbol a la izquierda y el elemento seleccionado a la derecha. La
  descripción se contrae con Ctrl+D. Las reglas de un grupo seleccionado muestran un conmutador al final de
  sus filas.
- **Barra de estado**: el estado del documento, la versión del catálogo, el servidor MCP y el último mensaje.

## Este equipo

**Este equipo**, en la barra de comandos, lee la configuración de este equipo a través de Windows, sin
cambiar nada:

- **Leer este equipo en un perfil nuevo**: cada regla que se puede evaluar en un Windows en ejecución queda
  activada (en vigor), desactivada (en vigor solo en parte o sin efecto) o mantiene su estado (no se puede
  leer, o está como en un Windows limpio); los formularios toman la edición, la zona horaria y los idiomas.
- **Leer la selección en este perfil** y **Comprobar la selección en este equipo** actúan sobre la regla o
  el grupo seleccionados.
- **Guardar un script de aplicación**, **un script de restablecimiento de los valores predeterminados** o
  **un script de auditoría** escribe un script de PowerShell para este equipo o para otro. Ejecuta los
  scripts de aplicación o de restablecimiento como administrador, primero en un equipo de prueba; guardan
  una copia de seguridad que restaura el script `Undo-Apply.ps1` que se encuentra a su lado. El informe de
  un script de auditoría se abre con **Abrir un informe de auditoría** y se convierte en un perfil.

No se leen las reglas que solo actúan durante la instalación o en el primer inicio de sesión de un usuario.

## Comparar perfiles

**Menú, Comparar con un archivo** o **Comparar con un perfil predefinido** abre una pestaña de comparación:
todas las diferencias entre el perfil activo y el otro, en tres columnas. "Aplicar el lado derecho"
establece esos valores en el perfil a partir del cual se hizo la comparación; "Guardar como archivo de
diferencias" conserva las diferencias en un archivo `.wwdiff` que se puede aplicar más adelante a
cualquier perfil.

## Abrir otros archivos

Abrir acepta perfiles (`.wwprofile`), comparaciones (`.wwdiff`), archivos de marcadores (`.wwmarks`),
páginas Markdown, archivos de respuesta generados por WinWeaver o WinKickOff (el perfil que incluyen),
perfiles de WinKickOff e informes de auditoría (`.json`).

## El servidor MCP

**Menú, Servidor MCP** abre la pestaña del servidor a través del cual los agentes de IA leen perfiles y,
cuando el modo lo permite, los cambian. El servidor se inicia en el modo lectura; edición le permite
cambiar perfiles en memoria y archivos le permite guardarlos y escribir el archivo de respuesta. Leer este
equipo requiere un permiso propio, que está desactivado en cada inicio.

## Teclado

| Teclas | Acción |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | Nuevo, Abrir, Guardar, Guardar como |
| F7, F9 | Comprobar el perfil, generar el archivo de respuesta |
| Ctrl+F, Esc | Buscar, borrar y cerrar la búsqueda |
| Alt+Left, Alt+Right | Atrás, Adelante |
| Ctrl+B | Agregar un marcador en la posición |
| Ctrl+D | Contraer o mostrar la descripción |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Cerrar la pestaña, pestaña siguiente, pestaña anterior |
| Esc | Cancelar la confirmación de un cambio de este equipo |
| Ctrl+, | Configuración |
| F1 | Esta guía |

## Archivos junto al ejecutable

`settings.json` (idioma, tema, archivos recientes, las pestañas de la última sesión), `profiles/` (tus
perfiles), `output/` (el archivo de respuesta y los scripts), `drafts/` (cambios sin guardar, que se
restauran en el siguiente inicio), `logs/` (lo que ha hecho el programa), `bookmarks.wwmarks`. No se
escribe nada en ningún otro sitio.

## Todavía no disponible

La aplicación de la configuración a este equipo directamente desde el programa (mientras tanto, usa el
script de aplicación) y la importación de plantillas de directivas (ADMX) llegarán en las próximas versiones.
