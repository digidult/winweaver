# wview

wview dosyaları gösterir. Her biçim bir eklentidir: `wview.exe` yanındaki `plugins` klasöründe bir
DLL. Dosya yokken pencere eklentileri ve her birinin neyi açtığını listeler.

## Dosya açma

- **Aç** (Ctrl+O) ya da pencereye bırakılan bir dosya, dosyayı bir sekmede açar;
  `wview DOSYA...` dosyaları komut satırından açar.
- Hiçbir eklentinin bilmediği bir uzantıya sahip ya da uzantısız bir dosya içeriğine göre açılır:
  bir metin, bir resim, bir program.
- Hiçbir eklentinin açmadığı bir biçimdeki dosya kapalı kalır ve durum çubuğu bunu söyler.
- Ctrl+W bir sekmeyi kapatır, Ctrl+Tab sonrakine geçer.
- Menü, öndeki sekmenin öğelerini ve kendi öğeleri olan her eklentinin bir alt menüsünü içerir.

## Metin ve Markdown

Markdown sayfaları (`.md`) sayfa olarak, metin dosyaları (`.txt`, `.log`, `.ini`, `.cfg`, `.csv`)
oldukları gibi gösterilir. Ctrl+F metinde arar. **Ayarlar**, **Genel**: *Satırları görünüm
genişliğine göre kaydır* ve *Sayfa genişliği* bir metnin görünümünü değiştirir.

XML dosyaları (`.xml`, `.manifest`) da metin olarak görünür ve işaretli bir UTF-16 metni UTF-8 gibi
okunur.

Ayarlardaki **Tamamı okunan en büyük dosya** değerinden (başta 64 MB) büyük bir dosya parça parça
açılır: satırları okunurken sayılır, numaraları solda görünür; tekerlek, oklar, Page Up ve Page
Down, Home, End ve sağdaki çubuk metinde gezinir, Sol ve Sağ onu yana kaydırır. Arama tüm sözcükleri
içeren satırları listeler ve birine tıklamak oraya gider.

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

## İkili dosyalar

Windows'un programları ve kitaplıkları (`.exe`, `.dll`, `.sys` ve Portable Executable'ın diğer
dosyaları) yapılarının ağacı olarak açılır: başlıklar, bölümler, içe ve dışa aktarmalar, her alan
değeri, uzaklığı ve boyutuyla.

- Solda ağaç: bir tıklama veya oklar bir düğümü seçer, sağ ve sol onu açar ve kapatır. Sağda
  dosyanın baytları, ASCII'leriyle satır başına 16: seçilen düğümün baytları işaretlidir ve bir
  bayta tıklama onu tutan en küçük düğümü seçer.
- Çubuktaki **Metin** ağacı sekmenin aramasıyla (Ctrl+F) bir sayfa olarak gösterir; **Ağaç ve
  baytlar** geri döner.
- Dosya diskte kalır: görünüm yalnızca gösterdiği blokları okur. Dosyanın tamamını tutmadığı bir
  düğüm (kısa veya bozuk bir dosya) bir uyarı işareti taşır ve nedenini söyler.
- Başlıkların yanında: kaynaklar (türleri, adları ve dilleri, bir düzey açıldığında okunur), x64
  istisnaları, sertifikalar, yer değiştirmeler, PDB yoluyla hata ayıklama dizini, TLS, yükleme
  yapılandırması, gecikmeli içe aktarmalar ve meta verilerinin akışlarıyla .NET başlığı.
- **Yapıyı kaydet** ağacı varsayılan olarak dosyanın yanına bir `.wwbin` dosyası olarak yazar: her
  alan değeriyle, her ham bayt aralığı SHA-256'sıyla, dosya boyutu ve SHA-256'sıyla, baytlarından
  hiçbiri olmadan.
- **Bir yapıyla karşılaştır** dosyayı ondan kaydedilmiş bir `.wwbin` ile (örneğin dosyanın başka
  bir sürümününkiyle) karşılaştırır: değişikliklerin ağacı, her biri önceki ve sonraki değeriyle,
  düğümler adlarına ve anahtarlarına göre eşleşir, böylece yeni bir bölüm başkalarını kaydırmaz;
  **Tüm düğümler** veya **Yalnızca değişiklikler**. Seçilen bir düğüm bir yorum ve bir yer imi
  alır; **Karşılaştırmayı kaydet** bunları iki dosyayla birlikte bir `.wwbindiff` dosyasına yazar,
  o da karşılaştırmayı kendi sekmesinde açar. Bir `.wwbin` tek başına da ağacı olarak açılır.
- Seçili düğüm başka bir eklentinin gösterdiği türden bir dosya içerdiğinde (bir programın
  bildirimi, simgeleri, bit eşlemleri ve HTML sayfaları, verileri arasındaki bir resim veya metin,
  içindeki başka bir program) çubukta **İçerik**: sağ taraf onu o eklentiyle, üstünde kendi
  düğmeleriyle gösterir ve seçimi izler; **Ağaç ve baytlar** geri döner. Menüde:
  **İçeriği bir sekmede aç**, **Düğümü dosya olarak kaydet** (bir simge ICO dosyası olarak, değilse
  baytları) ve **Tüm düğümleri oku**.
- Bir simge grubu (`GROUP_ICON`) her boyutuyla tek bir simge olarak görünür, boyutlar sayfa olarak,
  ve tek bir ICO dosyası olarak kaydedilir.
- Başka biçimler açıklama olarak gelir, `plugins\formats` klasöründeki `.wwformat` dosyaları;
  Ayarlar, Eklentiler okunan biçimleri ve okunamayan dosyaları listeler.
- **İkili yapılar**, menüdeki **Açıklamalar klasörü** `plugins\formats` klasörünü Gezgin'de açar,
  yoksa oluşturur.

## Mandelbrot kümesi

Rust dışında bir dildeki eklentinin örneği: C, wview'un kararlı arayüzü üzerinden. Merkez, yakınlık,
yineleme, palet ve yumuşatılmış renkleri içeren birkaç satırlık bir `.mandel` dosyası kümenin
görüntüsü olarak açılır; arşivde `examples\seahorse.mandel` vardır.

- Görüntünün üstünde: **Iterations**, **Palette** (classic, fire, ice, grey), **Smooth colours**,
  **Zoom in**, **Zoom out**, **Reset** ve görünümü dosyasına geri yazan **Save** (eklentinin
  metinleri İngilizcedir).
- Tekerlek işaretçinin altında yakınlaştırır, sürükleme görüntüyü kaydırır, Shift ve sürükleme bir
  dikdörtgen çizer ve bırakınca ona yakınlaşır, çift tıklama ortalar ve yakınlaştırır, sağ düğme
  uzaklaştırır.
- Görüntüye bir tıklamadan sonra: oklar onu kaydırır, + ve - yakınlaştırır, Page Up ve Page Down
  yinelemeleri ikiye katlar veya yarıya indirir, P sonraki paleti alır, Home sıfırlar, Escape bir
  dikdörtgeni bırakır.
- Görüntü adım adım gelir, hemen kaba ve çizildikçe daha ince; durum çubuğu merkezi, yakınlığı,
  yinelemeleri ve son görüntünün süresini söyler.

## Ayarlar

**Ayarlar**, **Eklentiler** eklentileri ayarlarıyla listeler: tamamı okunan en büyük metin dosyası
(daha büyüğü parça parça açılır); %100'ün üzerinde yumuşatılmış pikseller, EXIF yönü, açıldığında
oynatılan bir animasyon ve tamamen çözülen en büyük görüntü (64, 256 veya 1024 megapiksel); satır
başına bayt, bir ikili dosyanın bloğu ve bloklarının önbelleği. Ayarlar programın yanındaki
`wview.settings.json` dosyasında tutulur.

## Komut satırı

- `wview DOSYA...` dosyaları sekmelerde açar.
- `wview --plugins` eklentileri ve biçimlerini listeler.
- `wview --probe DOSYA` bir dosyayı açar ve pencere olmadan çizer, sürelerle birlikte.
- `wview --help` seçenekleri listeler.
