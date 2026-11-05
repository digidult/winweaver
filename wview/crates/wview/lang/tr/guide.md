# wview

wview dosyaları gösterir. Her biçim bir eklentidir: `wview.exe` yanındaki `plugins` klasöründe bir
DLL. Dosya yokken pencere eklentileri ve her birinin neyi açtığını listeler.

## Dosya açma

- **Aç** (Ctrl+O) ya da pencereye bırakılan bir dosya, dosyayı bir sekmede açar;
  `wview DOSYA...` dosyaları komut satırından açar.
- Hiçbir eklentinin açmadığı bir biçimdeki dosya kapalı kalır ve durum çubuğu bunu söyler.
- Ctrl+W bir sekmeyi kapatır, Ctrl+Tab sonrakine geçer.

## Metin ve Markdown

Markdown sayfaları (`.md`) sayfa olarak, metin dosyaları (`.txt`, `.log`, `.ini`, `.cfg`, `.csv`)
oldukları gibi gösterilir. Ctrl+F metinde arar. **Ayarlar**, **Genel**: *Satırları görünüm
genişliğine göre kaydır* ve *Sayfa genişliği* bir metnin görünümünü değiştirir.

## Görüntüler

Bilgisayardaki Windows'un çözdüğü her biçim: BMP, PNG, JPEG, GIF, TIFF, ICO ve uzantıları
kuruluysa HEIF, AVIF, WebP veya fotoğraf makinelerinin RAW dosyaları.

- Tekerlek işaretçinin altında yakınlaştırır, sürükleme görüntüyü kaydırır, çift tıklama tüm
  görüntü ile %100 arasında geçiş yapar.
- Çubuk: **Sığdır**, **%100**, **Yakınlaştır**, **Uzaklaştır** ve aynı klasörün görüntüleri için
  **Önceki** ve **Sonraki**; bir animasyon için **Oynat** (veya **Duraklat**), **Önceki kare** ve
  **Sonraki kare**; birkaç sayfalık bir dosya (TIFF, ICO) için **Önceki sayfa** ve **Sonraki
  sayfa**.
- Görüntüye bir tıklama tuşları ona verir: oklar pencereden büyük bir görüntüyü kaydırır, aksi
  halde sol ve sağ ok, Page Up ve Page Down'ın her zaman yaptığı gibi klasörün önceki veya sonraki
  görüntüsünü alır; Home ve End ilkini ve sonuncusunu; + ve - adım adım yakınlaştırır, 0 görüntüyü
  sığdırır, 1 ile 4 arası %100 ile %400 arasını gösterir; Boşluk bir animasyonu oynatır veya
  duraklatır, virgül ve nokta önceki veya sonraki kareye ya da sayfaya gider. Tab veya Escape
  tuşları pencereye geri verir.
- Bir fotoğraf, makinesinin tutulduğu gibi döndürülür (EXIF yönü). Büyük bir görüntü sekmesi zaten
  açıkken çözülür ve yakınlaştırmanın gerektirdiği ayrıntıyla parça parça çizilir; ayarlardaki
  sınırın üzerindeki bir görüntü küçültülerek çözülür ve durum çubuğu bunu söyler.

## Ayarlar

**Ayarlar**, **Eklentiler** eklentileri ayarlarıyla listeler: açılacak en büyük metin dosyası;
%100'ün üzerinde yumuşatılmış pikseller, EXIF yönü, açıldığında oynatılan bir animasyon ve
tamamen çözülen en büyük görüntü (64, 256 veya 1024 megapiksel). Ayarlar programın yanındaki
`wview.settings.json` dosyasında tutulur.

## Komut satırı

- `wview DOSYA...` dosyaları sekmelerde açar.
- `wview --plugins` eklentileri ve biçimlerini listeler.
- `wview --probe DOSYA` bir dosyayı açar ve pencere olmadan çizer, sürelerle birlikte.
- `wview --help` seçenekleri listeler.
