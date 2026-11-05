# wview

wview muestra archivos. Cada formato es un complemento: una DLL en la carpeta `plugins` junto a
`wview.exe`. Sin archivo, la ventana enumera los complementos y lo que abre cada uno.

## Abrir archivos

- **Abrir** (Ctrl+O), o un archivo soltado en la ventana, abre el archivo en una pestaña;
  `wview ARCHIVO...` abre archivos desde la línea de comandos.
- Un archivo de un formato que ningún complemento abre queda cerrado, y la barra de estado lo dice.
- Ctrl+W cierra una pestaña, Ctrl+Tab pasa a la siguiente.

## Texto y Markdown

Las páginas Markdown (`.md`) se muestran como páginas, los archivos de texto (`.txt`, `.log`,
`.ini`, `.cfg`, `.csv`) tal como son. Ctrl+F busca en el texto. **Configuración**, **General**:
*Ajustar las líneas al ancho de la vista* y *Ancho de las páginas* cambian cómo se ve un texto.

## Imágenes

Todos los formatos que decodifica el Windows del equipo: BMP, PNG, JPEG, GIF, TIFF, ICO, y HEIF,
AVIF, WebP o el RAW de las cámaras donde sus extensiones están instaladas.

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

## Configuración

**Configuración**, **Complementos** enumera los complementos con su configuración: el archivo de
texto más grande que se abre; píxeles suavizados por encima del 100%, la orientación EXIF, una
animación reproducida al abrirla y la imagen más grande decodificada entera (64, 256 o 1024
megapíxeles). La configuración se guarda en `wview.settings.json` junto al programa.

## Línea de comandos

- `wview ARCHIVO...` abre los archivos en pestañas.
- `wview --plugins` enumera los complementos y sus formatos.
- `wview --probe ARCHIVO` abre un archivo y lo dibuja sin ventana, con los tiempos.
- `wview --help` enumera las opciones.
