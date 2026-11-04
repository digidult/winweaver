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
  **Önceki** ve **Sonraki**.
- Görüntüye bir tıklama tuşları ona verir: sol ve sağ ok ya da Page Up ve Page Down klasörün önceki
  veya sonraki görüntüsünü, Home ve End ilkini ve sonuncusunu alır, + ve - yakınlaştırır,
  0 görüntüyü sığdırır, 1 %100 gösterir. Tab veya Escape tuşları pencereye geri verir.

## Ayarlar

**Ayarlar**, **Eklentiler** eklentileri ayarlarıyla listeler: açılacak en büyük metin dosyası;
%100'ün üzerinde yumuşatılmış pikseller ve çözülen görüntünün en uzun kenarı. Ayarlar programın
yanındaki `wview.settings.json` dosyasında tutulur.

## Komut satırı

- `wview DOSYA...` dosyaları sekmelerde açar.
- `wview --plugins` eklentileri ve biçimlerini listeler.
- `wview --probe DOSYA` bir dosyayı açar ve pencere olmadan çizer, sürelerle birlikte.
- `wview --help` seçenekleri listeler.
