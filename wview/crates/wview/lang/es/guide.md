# wview

wview muestra archivos. Cada formato es un complemento: una DLL en la carpeta `plugins` junto a
`wview.exe`. Sin archivo, la ventana enumera los complementos y lo que abre cada uno.

## Abrir archivos

- **Abrir** (Ctrl+O), o un archivo soltado en la ventana, abre el archivo en una pestaña;
  `wview ARCHIVO...` abre archivos desde la línea de comandos.
- Un archivo de una extensión que ningún complemento conoce, o sin extensión, se abre por su
  contenido: un texto, una imagen, un programa.
- Un archivo de un formato que ningún complemento abre queda cerrado, y la barra de estado lo dice.
- Ctrl+W cierra una pestaña, Ctrl+Tab pasa a la siguiente.
- El menú contiene los elementos de la pestaña activa y un submenú de cada complemento que tiene
  elementos propios.

## Texto y Markdown

Las páginas Markdown (`.md`) se muestran como páginas, los archivos de texto (`.txt`, `.log`,
`.ini`, `.cfg`, `.csv`) tal como son. Ctrl+F busca en el texto. **Configuración**, **General**:
*Ajustar las líneas al ancho de la vista* y *Ancho de las páginas* cambian cómo se ve un texto.

Los archivos XML (`.xml`, `.manifest`) también se muestran como textos, y un texto UTF-16 con su
marca se lee igual que uno UTF-8.

Un archivo mayor que **Archivo más grande leído entero** de la configuración (64 MB al principio) se
abre por partes: sus líneas se cuentan mientras se lee, con sus números a la izquierda; la rueda,
las flechas, Re Pág y Av Pág, Inicio, Fin y la barra de la derecha lo recorren, Izquierda y Derecha
lo mueven de lado. La búsqueda muestra las líneas con todas las palabras, y un clic en una lleva a
ella.

## Imágenes

Todos los formatos que decodifica el Windows del equipo: BMP, PNG, JPEG, GIF, TIFF, ICO, y HEIF,
AVIF, WebP o el RAW de las cámaras donde sus extensiones están instaladas.

Donde Windows no tiene decodificador, el complemento decodifica él mismo WebP, TGA, Netpbm (PBM,
PGM, PPM, PAM), QOI, Radiance HDR y farbfeld (de un WebP animado, el primer fotograma).

- La rueda hace zoom en el puntero, arrastrar mueve la imagen, un doble clic alterna entre la
  imagen entera y el 100%.
- La barra: **Ajustar**, **100%**, **Acercar**, **Alejar**, y **Anterior** y **Siguiente**, las
  imágenes de la misma carpeta; para una animación **Reproducir** (o **Pausa**), **Fotograma
  anterior** y **Fotograma siguiente**; para un archivo de varias páginas (TIFF, ICO) **Página
  anterior** y **Página siguiente**.
- Un clic en la imagen le da las teclas: las flechas mueven una imagen más grande que la ventana;
  si no, flecha izquierda y derecha toman la imagen anterior o siguiente de la carpeta, como
  siempre hacen Re Pág y Av Pág; Inicio y Fin la primera y la última; + y - hacen zoom por pasos,
  0 ajusta la imagen, de 1 a 4 muestran del 100% al 400%; la barra espaciadora reproduce o pausa
  una animación, la coma y el punto van al fotograma o la página anterior o siguiente. Tab o
  Escape devuelven las teclas a la ventana.
- Una foto se gira como se sostenía su cámara (la orientación EXIF). Una imagen grande se
  decodifica con su pestaña ya abierta y se dibuja por partes con el detalle que el zoom necesita;
  una imagen por encima del límite de la configuración se decodifica más pequeña, y la barra de
  estado lo dice.

## Archivos binarios

Los programas y bibliotecas de Windows (`.exe`, `.dll`, `.sys` y los demás archivos del Portable
Executable) se abren como el árbol de su estructura: los encabezados, las secciones, las
importaciones y las exportaciones, cada campo con su valor, su desplazamiento y su tamaño.

- A la izquierda el árbol: un clic o las flechas seleccionan un nodo, derecha e izquierda lo abren
  y lo cierran. A la derecha los bytes del archivo, 16 por fila con su ASCII: los bytes del nodo
  seleccionado están marcados, y un clic en un byte selecciona el nodo más pequeño que lo contiene.
- **Texto** en la barra muestra el árbol como una página, con la búsqueda de la pestaña (Ctrl+F);
  **Árbol y bytes** vuelve atrás.
- El archivo se queda en el disco: la vista lee los bloques que muestra. Un nodo que el archivo no
  contiene entero (un archivo corto o dañado) tiene un signo de aviso y dice por qué.
- Además de los encabezados: los recursos (sus tipos, nombres e idiomas, un nivel se lee al
  abrirlo), las excepciones de x64, los certificados, las reubicaciones, el directorio de
  depuración con la ruta del PDB, TLS, la configuración de carga, las importaciones diferidas y el
  encabezado de .NET con los flujos de sus metadatos.
- **Guardar la estructura** escribe el árbol como un archivo `.wwbin`, junto al archivo por
  defecto: cada campo con su valor, cada rango de bytes en bruto con su SHA-256, el archivo con su
  tamaño y su SHA-256, y ninguno de sus bytes.
- **Comparar con una estructura** compara el archivo con un `.wwbin` guardado de él (de otra
  versión del archivo, por ejemplo): el árbol de los cambios, cada uno con su valor antes y
  después, los nodos emparejados por sus nombres y claves, de modo que una sección nueva no mueve
  ninguna otra; **Todos los nodos** o **Solo los cambios**. Un nodo seleccionado admite un
  comentario y un marcador; **Guardar la comparación** los escribe con los dos archivos en un
  `.wwbindiff`, que abre la comparación en una pestaña propia. Un `.wwbin` se abre también solo,
  como su árbol.
- **Contenido** en la barra, cuando el nodo seleccionado contiene un archivo de un tipo que muestra
  otro complemento (el manifiesto de un programa, sus iconos, mapas de bits y páginas HTML, una
  imagen o un texto entre sus datos, otro programa dentro): la parte derecha lo muestra con ese
  complemento, con sus propios botones encima, y sigue la selección; **Árbol y bytes** vuelve. En el
  menú: **Abrir el contenido en una pestaña**, **Guardar el nodo como archivo** (un icono como
  archivo ICO, si no sus bytes) y **Leer todos los nodos**.
- Un grupo de iconos (`GROUP_ICON`) se muestra como un icono de todos los tamaños, los tamaños como
  páginas, y se guarda como un archivo ICO. Un grupo de cursores (`GROUP_CURSOR`) se muestra igual,
  sin sus puntos activos.
- Más formatos llegan como descripciones, archivos `.wwformat` en la carpeta `plugins\formats`;
  Configuración, Complementos enumera los formatos leídos y los archivos que no se leyeron.
- **Estructuras binarias**, **Carpeta de las descripciones** en el menú abre la carpeta
  `plugins\formats` en el Explorador y la crea si no existe.

## El conjunto de Mandelbrot

El ejemplo de un complemento en otro lenguaje que Rust: C, a través de la interfaz estable de
wview. Un archivo `.mandel`, unas líneas con el centro, el zoom, las iteraciones, la paleta y los
colores suavizados, se abre como la imagen del conjunto; el archivo comprimido trae
`examples\seahorse.mandel`.

- Sobre la imagen: **Iterations**, **Palette** (classic, fire, ice, grey), **Smooth colours**,
  **Zoom in**, **Zoom out**, **Reset** y **Save**, que escribe la vista de vuelta en su archivo
  (los textos del complemento están en inglés).
- La rueda hace zoom en el puntero, arrastrar mueve la imagen, Mayús y arrastrar dibuja un
  rectángulo al que soltar hace zoom, un doble clic centra y acerca, el botón derecho aleja.
- Tras un clic en la imagen: las flechas la mueven, + y - hacen zoom, Re Pág y Av Pág duplican o
  reducen a la mitad las iteraciones, P toma la paleta siguiente, Inicio restablece, Escape
  descarta un rectángulo.
- La imagen llega por pasos, gruesa al momento y más fina mientras se dibuja; la barra de estado
  nombra el centro, el zoom, las iteraciones y el tiempo de la última imagen.

## Configuración

**Configuración**, **Complementos** enumera los complementos con su configuración: el archivo de
texto más grande leído entero (uno mayor se abre por partes); píxeles suavizados por encima del
100%, la orientación EXIF, una animación reproducida al abrirla y la imagen más grande decodificada
entera (64, 256 o 1024 megapíxeles); los bytes por fila, el bloque de un archivo binario y la caché
de sus bloques. La configuración se guarda en `wview.settings.json` junto al programa.

## Línea de comandos

- `wview ARCHIVO...` abre los archivos en pestañas.
- `wview --plugins` enumera los complementos y sus formatos.
- `wview --probe ARCHIVO` abre un archivo y lo dibuja sin ventana, con los tiempos.
- `wview --help` enumera las opciones.
