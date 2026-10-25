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
- **El catálogo**: las reglas integradas en tres partes y las plantillas de directivas (ADMX) que importe,
  que también se convierten en reglas. **Instalación** reúne lo que hacen el archivo de respuesta y sus
  scripts; **Directivas**, todas las reglas que solo escriben en el Registro y todas las importaciones;
  **Otras**, el resto: servicios, características, aplicaciones, programas y scripts en un Windows en
  ejecución.
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

## El nombre del equipo

En el formulario **Instalación** el nombre del equipo es aleatorio (como hace Windows de forma
predeterminada), un nombre fijo o una plantilla que el archivo de respuesta expande en cada PC durante la
instalación. Una plantilla contiene letras latinas, dígitos y guiones alrededor de estos marcadores:

| Marcador | Qué da |
|---|---|
| `{serial}`, `{serial:N}` | El número de serie del equipo según su firmware, sus letras y dígitos en mayúsculas; `:N` conserva sus últimos N caracteres, sin él todos (hasta 15) |
| `{mac}`, `{mac:N}` | La dirección MAC del primer adaptador de red físico, 12 dígitos hexadecimales en mayúsculas; `:N` conserva sus últimos N dígitos, sin él los 12 |
| `{random}`, `{random:N}` | Mayúsculas y dígitos aleatorios sin los parecidos I, O, 0 y 1; `:N` da N de ellos, sin él 6 |

N va de 1 a 15. Un número de serie o una dirección que no se pueden leer (no hay, o el fabricante dejó un
marcador como `Default string`) se convierten en caracteres aleatorios de la misma longitud; uno más
corto se queda entero. El nombre con la longitud completa de cada marcador cabe en 15 caracteres, así que
`PC-{serial}` es demasiado largo y `PC-{serial:12}` cabe; deje alguna letra, porque Windows rechaza un
nombre solo de dígitos. `PC-{random}` da `PC-` y seis caracteres aleatorios, como `PC-K7Q2MX`; con el
número de serie `5CG1234XYZ` y la dirección `00155D3E5A01`, `WS-{serial:6}` da `WS-234XYZ` y
`{mac:4}-LAB` da `5A01-LAB`. Debajo del campo el formulario muestra qué da la plantilla.

## Idiomas de entrada

El formulario **Idiomas** muestra los idiomas de entrada en su orden; el primero es el predeterminado.
Cada idioma muestra su nombre y una lista de distribuciones de teclado: primero la que Windows le da de
forma predeterminada y después todas las distribuciones de Windows por su nombre. **Agregar** busca un
idioma por una parte de su nombre o etiqueta entre todos los idiomas que conoce Windows y lo agrega con
la distribución elegida; el mismo idioma puede aparecer dos veces con dos distribuciones, como Ucraniano
(extendido) y Ucraniano. Un idioma que Windows numera solo en el primer inicio de sesión, como Ruso
(Ucrania), recibe la distribución que Windows le da entonces, y hasta ese inicio de sesión el archivo de
respuesta nombra el idioma al que pertenece. Los idiomas y distribuciones son los del Windows en el que
se ejecuta WinWeaver, con sus nombres en su idioma.

## Archivos de usuarios

Un archivo de usuarios (`.wwusers`) enumera las cuentas locales que hay que crear en un PC con sus
contraseñas, cifradas con una contraseña maestra en el mismo archivo. **Menú, Nuevo, Archivo de
usuarios** empieza uno vacío; **Archivo de usuarios con las cuentas de este PC**, uno con todas las
cuentas normales de este PC (grupo, estado, caducidad de la contraseña, carpeta del perfil; sin
contraseñas). La pestaña muestra las cuentas en una lista y la seleccionada en un formulario: nombre,
nombre completo, descripción, grupo, carpeta del perfil y las órdenes para una cuenta que ya existe en un
PC (establecer la contraseña del archivo, habilitarla o deshabilitarla, exactamente este grupo); sin
ellas una cuenta existente se deja como está. **Desbloquear** pide la contraseña maestra o la establece
cuando el archivo no tiene; **Contraseña** establece la de la cuenta seleccionada; **Bloquear** las
olvida. Sin la contraseña maestra las contraseñas no se pueden volver a leer. **Comprobar** enumera los
problemas, **Leer este PC** agrega las cuentas de este PC que faltan en el archivo. **Crear en este PC**
y **Aplicar contraseñas** cambian este PC tras una pregunta y necesitan WinWeaver ejecutado como
administrador; el resultado queda bajo el formulario, una línea por cuenta. **Script** guarda
`Create-<nombre>.ps1` junto al archivo de usuarios: pide la contraseña maestra por sí mismo y hace lo
mismo en otro PC desde un símbolo del sistema abierto como administrador (`-PasswordsOnly` solo establece
las contraseñas). En el formulario **Cuentas** de un perfil, **De un archivo de usuarios** carga sus
cuentas, con sus contraseñas si da la contraseña maestra.

Mientras el archivo está desbloqueado, el formulario muestra la contraseña de la cuenta seleccionada como
puntos, con **Mostrar** y **Copiar** (el historial del portapapeles de Windows conserva lo copiado). En
el formulario **Cuentas** de un perfil, **A un archivo de usuarios** crea un archivo de usuarios con sus
cuentas; sus contraseñas esperan una contraseña maestra antes del primer guardado.

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
- **Enlaces**: cada regla, grupo, catálogo, archivo de perfil y clave del registro que nombra una vista
  lleva a él: las reglas que necesita una regla, las que la necesitan y aquellas con las que entra en
  conflicto; las reglas nombradas en los mensajes; los grupos por encima y dentro de un grupo; la misma
  directiva en otros catálogos; las reglas que escriben una clave del registro, en la pestaña del
  registro y en un archivo de registro. Un enlace en un perfil abre su árbol hasta el elemento; desde
  otra pestaña muestra la regla en la primera pestaña de perfil.
- **Barra de estado**: el estado del documento, la versión del catálogo, el servidor MCP y el último mensaje.
- **Abrir la carpeta**: tras una compilación o al guardar un script, un archivo de registro o un perfil,
  un botón al final de la barra de estado muestra el archivo en su carpeta en el Explorador;
  Configuración, Archivos abre las carpetas del programa.

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
- **Guardar un archivo de registro** o **un archivo de registro de restablecimiento de los valores
  predeterminados** escribe los valores del registro de la selección en un archivo `.reg` para regedit o
  `reg import`; lo que un archivo de registro no puede contener (pasos de scripts, aplicaciones,
  funcionalidades) queda en él como comentarios. Los valores del perfil de usuario predeterminado van a
  `HKEY_USERS\UnattendDefault`, y el comentario al principio del archivo explica cómo cargar allí su
  subárbol.

No se leen las reglas que solo actúan durante la instalación o en el primer inicio de sesión de un usuario.

Las características y funcionalidades de Windows se leen mediante DISM cuando WinWeaver se ejecuta como
administrador, lo que añade unos segundos a la lectura; sin esos permisos no se pueden leer.
La lectura se hace en segundo plano: la ventana sigue funcionando y un indicador en la barra de estado la muestra.

Al aplicar como administrador, una regla que quita aplicaciones o funcionalidades de Windows también las
quita desde la ventana: las funcionalidades mediante DISM, las aplicaciones para todos los usuarios y para
las cuentas creadas después. La copia de seguridad las nombra, pero Deshacer no las devuelve.

Los pasos de script de las reglas también se aplican de forma nativa, sin PowerShell: .NET Framework 3.5
y los controladores del medio de instalación, los tipos de inicio de los servicios, NetBIOS, las unidades
extraíbles en el panel de navegación, «la contraseña nunca caduca» para las cuentas del perfil, la
desactivación de las cuentas integradas Administrador e Invitado (nunca la cuenta en uso) y la
eliminación de las copias del archivo de respuesta. Deshacer no revierte los programas ejecutados, los
cambios de cuentas ni los archivos eliminados; la confirmación dice cuántos pasos de este tipo tiene un
plan.

## Comparar perfiles

**Menú, Comparar con un archivo** o **Comparar con un perfil predefinido** abre una pestaña de comparación:
todas las diferencias entre el perfil activo y el otro, en tres columnas. "Aplicar el lado derecho"
establece esos valores en el perfil a partir del cual se hizo la comparación; "Guardar como archivo de
diferencias" conserva las diferencias en un archivo `.wwdiff` que se puede aplicar más adelante a
cualquier perfil.

**Comparar, Cambios sin guardar** abre una pestaña de diferencias del perfil con su archivo, o con el
perfil predefinido del que procede: a la izquierda el perfil tal como está ahora, a la derecha tal como
se guardó.

## Abrir otros archivos

Abrir acepta perfiles (`.wwprofile`), comparaciones (`.wwdiff`), archivos de marcadores (`.wwmarks`),
páginas Markdown, archivos de respuesta con un perfil incluido (el perfil que incluyen), perfiles
antiguos en JSON e informes de auditoría (`.json`).

Un archivo de registro (`.reg`) se abre en una pestaña propia, como describe la sección sobre el
registro.

## Plantillas de directivas

**Configuración, Plantillas de directivas** importa las plantillas ADMX de este Windows, de una carpeta o
de un archivo de catálogo; sus directivas se convierten en reglas dentro de Directivas, y las
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

**Editar** en la barra de una pestaña de catálogo lo abre en un editor, y **Nuevo catálogo** en el menú de las
plantillas empieza uno vacío propio. Una directiva recibe su título, su descripción, el equipo o el perfil
de usuario predeterminado y los valores del registro que escribe o quita; **Aplicar** comprueba todo el
archivo, y nada cambia si la comprobación falla. Guardar (Ctrl+S) escribe el archivo y lo vuelve a cargar.

## El registro

**Menú, Registro** abre el registro de este equipo en una pestaña: las claves a la izquierda, los valores
de la clave elegida a la derecha con sus tipos (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD,
REG_QWORD, REG_BINARY), la ruta en la fila de arriba. Un doble clic edita un valor; Nueva clave, Nuevo
valor, Editar y Eliminar están en la barra, y una eliminación pide confirmación antes. Las claves del
equipo (HKEY_LOCAL_MACHINE) solo cambian cuando WinWeaver se ejecuta como administrador. El botón junto a
una clave en **Qué escribe** la abre aquí. **Marcador** guarda la clave y el valor con un comentario, y
el marcador muestra las reglas que la escriben. **Buscar** en la barra busca bajo la clave elegida como
la búsqueda de regedit: en los nombres de las claves, los nombres de los valores y los datos, solo la
cadena completa si se pide. La búsqueda se hace en segundo plano, **Detener** la termina y un clic en un
lugar encontrado lo muestra.

**Permisos** en la barra muestra el propietario y la lista de acceso de la clave seleccionada: las
entradas heredadas se muestran, las entradas propias de la clave cambian (permitir o denegar, control
total o lectura, a qué se aplican), una cuenta o un grupo de este equipo recibe una entrada nueva y la
herencia de la clave superior se puede desactivar. Aplicar pregunta antes; una clave del equipo solo
cambia con WinWeaver ejecutado como administrador.

Un archivo de registro (`.reg`) se abre en una pestaña propia, como una estructura y no como texto: a la
izquierda las claves y los comentarios del archivo en su orden, a la derecha la clave seleccionada con
sus valores en los tipos de regedit y los comentarios entre ellos, un enlace a la pestaña del registro y
las reglas de los catálogos cargados que escriben la clave o el valor seleccionado. **Nueva clave**,
**Nuevo valor**, **Editar** y **Eliminar** están en la barra, y una eliminación pregunta antes.
**Guardar** vuelve a escribir cada línea: los comentarios y las líneas que el programa no lee se quedan
donde estaban. Un archivo guardado desde **Este equipo** nombra sus reglas en comentarios, y la flecha de
ese comentario muestra la regla en el perfil.

## El servidor MCP

**Menú, Servidor MCP** abre la pestaña del servidor a través del cual los agentes de IA leen perfiles y, si el modo lo permite, los cambian. La pestaña edita un archivo de configuración, `mcp/default.wwmcp` junto al programa, y lo guarda con cada cambio: el puerto y el token, el modo (read solo lee, edit cambia perfiles en memoria, files además los guarda y escribe el archivo de respuestas), los permisos de leer este equipo, leer o cambiar el registro y cambiar los marcadores y catálogos, el perfil con el que empieza el servidor y el registro detallado. **Abrir** toma otro archivo `.wwmcp` y **Guardar como** hace una copia; `winweaver --mcp FILE` sirve sin la ventana como dice el archivo. El registro del servidor en marcha lista cada llamada; **Mostrar los detalles** abre a su derecha un panel con los argumentos y la respuesta de la llamada elegida y botones a las reglas, perfiles, catálogos y claves del registro que nombra. El registro del servidor es `logs/mcp-<fecha>.jsonl`, un objeto JSON por línea; el registro detallado, desactivado por defecto, guarda cada solicitud y respuesta completas, sin contraseñas, claves ni tokens.

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
| `--edit-library` | Con `--mcp`: las herramientas pueden cambiar los marcadores y los archivos de catálogo; leerlos siempre está permitido |
| `--profile NAME` | Con `--mcp`: un perfil predefinido o un perfil de `profiles/` pasa a ser el actual |
| `--mcp FILE` | Sirve MCP sin la ventana como dice un archivo de configuración: una ruta a un archivo `.wwmcp` o un nombre de la carpeta `mcp` junto al programa |
| `--config FILE` | Con `--mcp stdio` o `--mcp http`: la configuración de un archivo, también el token; las otras opciones la cambian para este inicio |
| `--detailed-log` | Con `--mcp`: el registro guarda cada solicitud y respuesta completas, sin contraseñas, claves ni tokens |
| `--version` | Muestra la versión |
| `--check-catalog` | Carga el catálogo integrado con sus traducciones y muestra sus problemas |
| `--build PROFILE` | Comprueba un perfil y genera su archivo de respuesta sin la ventana: `autounattend.xml` en `output`, con los scripts que incrusta junto a él; PROFILE es un archivo `.wwprofile`, un perfil predefinido (`Office`, `Strict`, `Laptop`, `Home`) o el nombre de un perfil de `profiles/`; un error de la comprobación detiene la generación |
| `--import-admx SOURCE` | Importa plantillas de directivas como lo hace Configuración, Plantillas de directivas: `system` (las plantillas de este Windows), una carpeta de archivos ADMX, un archivo `.admx` con los ADML de las carpetas de sus idiomas, un archivo `.zip` o un archivo `.wwcatalog`; el archivo de catálogo va a `imports/`, y la ventana y el servidor MCP lo cargan en su próximo inicio (ejecútelo con la ventana cerrada) |
| `--export-catalog SOURCE` | Escribe un archivo de catálogo (`.wwcatalog`) como lo hace Exportar en Configuración, Plantillas de directivas: de una importación por su identificador (el nombre de su archivo en `imports/`), o de un origen de `--import-admx` leído sin importarlo; `<id>.wwcatalog` en `output` |
| `--read-pc` | Sin `--mcp`: lee este equipo en un perfil nuevo con el nombre del equipo, como Este equipo, Leer este PC, y lo guarda en `profiles/`; con un archivo `.xml` en `--out` escribe en su lugar el archivo de respuesta de ese perfil; las características y funcionalidades de Windows solo se leen cuando el programa se ejecuta como administrador |
| `--base PROFILE` | Con `--read-pc`: el perfil del que parte la lectura, indicado como para `--build`; por defecto el perfil predefinido Office |
| `--diff A B` | Escribe las diferencias del perfil A al perfil B como archivo de diferencias, `A-vs-B.wwdiff` en `profiles/`; A y B se indican como para `--build` |
| `--script KIND FILE` | Escribe sin la ventana el script independiente de un perfil (`.wwprofile`) o de una diferencia (`.wwdiff`): `apply`, `revert` (vuelta a los valores predeterminados de Windows) o `audit`; junto a un script de aplicación o de vuelta queda `Undo-Apply.ps1` |
| `--reg KIND FILE` | Escribe el archivo de registro de la aplicación (`apply`) o de la vuelta a los valores predeterminados (`revert`) de un perfil o de una diferencia |
| `--only IDS` | Con `--script` o `--reg`: solo estas reglas o grupos, sus identificadores separados por comas |
| `--out FILE` | Con `--build`, `--export-catalog`, `--read-pc`, `--diff`, `--script` o `--reg`: el archivo que se escribe, cuando el valor termina con la extensión que escribe el comando (`.xml`, `.wwcatalog`, `.wwprofile`, `.wwdiff`, `.ps1`, `.reg`); cualquier otro valor es una carpeta, creada si falta, para el archivo con su nombre habitual; `Undo-Apply.ps1` y los scripts del archivo de respuesta van junto al archivo; sin `--out` los archivos van a `output` junto al programa (`profiles/` para `--read-pc` y `--diff`) |
| `--help` | Muestra la lista de parámetros |
| `--smoke-window` | Abre una ventana pequeña, dibuja dos fotogramas y muestra con qué se dibujaron (también con `--renderer`): una comprobación de los gráficos |
| `--elevated-apply PLAN DIGEST` | Así se inicia la propia ventana para aplicar un plan confirmado como administrador; no es para iniciarlo a mano |

Un comando que escribe archivos nombra cada archivo que escribe y termina con el código 0; un uso incorrecto
de los parámetros termina con el código 2, un fallo con 1, y se muestra el motivo.

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

`winweaver.settings.json` (idioma, tema, archivos recientes, las pestañas de la última sesión), `profiles/` (tus
perfiles), `output/` (el archivo de respuesta y los scripts), `drafts/` (cambios sin guardar, que se
restauran en el siguiente inicio), `mcp/` (las configuraciones del servidor MCP), `logs/` (lo que ha hecho el programa), `bookmarks.wwmarks`. No se
escribe nada en ningún otro sitio.

La configuración, los marcadores, los borradores y los archivos de diferencias guardan la ruta de un
archivo de esta carpeta de forma relativa a ella, así que la carpeta entera puede moverse o copiarse a
otro PC; un archivo en otro lugar conserva su ruta completa.

## Todavía no disponible

Por ahora no falta nada del plan: lo que el programa necesite después lo mostrará la práctica.
