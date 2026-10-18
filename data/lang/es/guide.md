# Guía de WinWeaver

WinWeaver prepara instalaciones desatendidas de Windows 10 y 11 y mantiene configurados los equipos
instalados. Un perfil indica qué reglas del catálogo están activadas, con qué parámetros y qué contienen
los formularios de instalación; a partir de él el programa genera el archivo de respuesta
`autounattend.xml` con sus scripts, compara perfiles, lee la configuración de un equipo en marcha y la
aplica. El catálogo está pensado para oficinas pequeñas sin dominio: primero la seguridad y las
actualizaciones.

Esta página se abre en el primer inicio y, más adelante, desde el menú o con F1. Es un archivo Markdown;
las páginas de documentación se abren del mismo modo, cada una en su propia pestaña.

## Cómo está organizado

- **Perfiles** (`.wwprofile`): guardan sus elecciones; los perfiles predefinidos Office, Strict, Laptop y
  Home son puntos de partida.
- **El catálogo**: las reglas integradas en tres partes (Instalación, Directivas estándar, Otros) y las
  plantillas de directivas (ADMX) que importe, que también se convierten en reglas.
- **Pestañas**: cada documento se abre en su propia pestaña: perfiles, comparaciones, catálogos,
  marcadores, el servidor MCP, páginas como esta.
- **Este equipo** lee el Windows en marcha, comprueba reglas en él, las aplica con una copia de seguridad
  y crea scripts para otros equipos.
- **La carpeta del programa** guarda todo junto a `winweaver.exe` (vea el final de esta página); no se
  instala nada.

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
7. Pruebe el archivo de respuesta en una máquina virtual antes de cualquier equipo real: instale, después
   lea ese equipo con **Este equipo** y compárelo con el perfil.
8. Mantenga al día los equipos instalados: **Este equipo** aplica las reglas seleccionadas como
   administrador con una copia de seguridad, o guarda scripts independientes que aplican, revierten,
   auditan y deshacen en otros equipos.

## La ventana

- **Pestañas**: cada documento abierto es una pestaña: un perfil, una comparación, esta guía, los
  marcadores, el servidor MCP, la configuración. El botón de menú del extremo izquierdo de la fila de
  pestañas contiene Nuevo, Abrir, Guardar, Comparar, la guía, el servidor MCP, los marcadores,
  Configuración y Salir.
- **Acerca de y Licencia** en el menú: la versión con el commit de la compilación y la licencia (MIT),
  integrada en el programa, así que se queda con `winweaver.exe` aunque solo se mueva ese archivo.
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

## Plantillas de directivas

**Configuración, Plantillas de directivas** importa las plantillas ADMX de este Windows, de una carpeta o
de un archivo de catálogo; sus directivas se convierten en reglas dentro de Directivas estándar, y las
importaciones se guardan en `imports/`. Solo se leen y cargan las importaciones marcadas; las demás no
ocupan memoria. Un perfil puede tener opciones de una importación que no está cargada: se conservan sin
cambios en el perfil, aparecen en **Opciones desconocidas** al final del árbol y quedan fuera de la
generación, con un aviso al abrir el perfil y al generar el archivo de respuesta. **Cargar catálogos** en
ese grupo encuentra las importaciones que las contienen y carga solo esas. Un ajuste (General, Ventana)
oculta el grupo.

Un archivo zip de plantillas, como las publican Chrome, Edge y Firefox, o de un archivo de catálogo se
importa con **Importar un archivo comprimido...** y se lee sin descomprimirlo.

Una directiva que tienen dos importaciones aparece en ambos árboles. **Comparar** en la barra de una
pestaña de catálogo lo pone junto a otra importación cargada o un archivo de catálogo abierto: las
directivas que solo están en uno de ellos y aquellas cuyo título, descripción, valores o parámetros
difieren. Quitar una importación, como cualquier eliminación, pide confirmación antes.

El panel de detalles de una opción enlaza las pestañas de los catálogos que la tienen y la misma
directiva en los árboles de otras importaciones; un marcador de la opción también. En Configuración,
General se elige qué partes de la descripción se abren al principio.

## El registro

**Menú, Registro** abre el registro de este equipo en una pestaña: las claves a la izquierda, los valores de
la clave elegida a la derecha con sus tipos (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD,
REG_BINARY), la ruta en la fila de arriba. Un doble clic edita un valor; Nueva clave, Nuevo valor, Editar y
Eliminar están en la barra, y una eliminación pide confirmación antes. Las claves del equipo
(HKEY_LOCAL_MACHINE) solo cambian cuando WinWeaver se ejecuta como administrador. El botón junto a una clave
en **Qué escribe** la abre aquí. **Marcador** guarda la clave y el valor con un comentario, y el marcador
muestra las reglas que la escriben. Los permisos de las claves y la búsqueda en el registro llegarán más
tarde.

## El servidor MCP

**Menú, Servidor MCP** abre la pestaña del servidor a través del cual los agentes de IA leen perfiles y,
cuando el modo lo permite, los cambian. El servidor se inicia en el modo lectura; edición le permite
cambiar perfiles en memoria y archivos le permite guardarlos y escribir el archivo de respuesta. Leer este
equipo requiere un permiso propio, que está desactivado en cada inicio.

## El renderizador

La ventana dibuja con OpenGL o con DirectX 12. **Automático** (Configuración, General, Ventana) prueba
primero OpenGL, que ocupa menos memoria, y pasa a DirectX 12 cuando OpenGL no se inicia: en una máquina
virtual sin aceleración 3D, en una instalación nueva sin el controlador de la tarjeta gráfica, en algunas
sesiones remotas. DirectX 12 también dibuja allí, con el rasterizador por software de Windows. Si la
ventana se dibuja mal con OpenGL (un controlador antiguo), elija DirectX 12. Un cambio se aplica en el
siguiente inicio; `winweaver.exe --renderer directx` (o `opengl`, `auto`) elige solo para un inicio. El
registro de `logs/` indica con qué dibuja la ventana.

## Línea de comandos

`winweaver.exe` sin parámetros abre la ventana. Un valor sigue a su parámetro tras un espacio o `=`:
`--renderer=directx`.

| Parámetros | Qué hacen |
|---|---|
| `--renderer NAME` | Abre la ventana y prueba primero el renderizador NAME, solo en este inicio: `auto`, `opengl` o `directx` |
| `--mcp stdio` | Sirve MCP por la entrada y salida estándar, sin la ventana; así lo inicia un cliente MCP |
| `--mcp http --token TOKEN` | Sirve MCP en `http://127.0.0.1:8765/mcp` sin la ventana; el token tiene al menos 32 caracteres |
| `--port N` | Con `--mcp http`: otro puerto |
| `--mode NAME` | Con `--mcp`: `read` (por defecto) solo lee, `edit` cambia los perfiles en memoria, `files` además los guarda y escribe el archivo de respuesta |
| `--read-pc` | Con `--mcp`: permite leer la configuración de este equipo |
| `--registry NAME` | Con `--mcp`: `off` (por defecto), `read` deja que las herramientas lean el registro, `write` que también lo cambien |
| `--profile NAME` | Con `--mcp`: un perfil predefinido o un perfil de `profiles/` pasa a ser el actual |
| `--version` | Muestra la versión |
| `--check-catalog` | Carga el catálogo integrado con sus traducciones y muestra sus problemas |
| `--help` | Muestra la lista de parámetros |
| `--smoke-window` | Abre una ventana pequeña, dibuja dos fotogramas y muestra con qué se dibujaron (también con `--renderer`): una comprobación de los gráficos |
| `--elevated-apply PLAN DIGEST` | Así se inicia la propia ventana para aplicar un plan confirmado como administrador; no es para iniciarlo a mano |

El modo y el permiso para leer este equipo se indican en cada inicio y nunca se guardan. Los mensajes van a
la consola desde la que se inició el programa y a `logs/`.

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

Leer las características y funcionalidades de Windows directamente desde el programa: mientras tanto las
lee el script de auditoría.
