# WinWeaver kılavuzu

WinWeaver, küçük çalışma grupları için Windows 11 Pro'nun gözetimsiz yüklemelerini hazırlar ve yüklü
bilgisayarların yapılandırmasını korur. Profil, kataloğun hangi kurallarının hangi parametrelerle açık
olduğunu ve yükleme formlarının neleri içerdiğini belirtir; program bu profilden `autounattend.xml`
dosyasını oluşturur.

Bu sayfa ilk başlangıçta, daha sonra da menüden veya F1 ile açılır. Bir Markdown dosyasıdır; belge
sayfaları da aynı şekilde, her biri kendi sekmesinde açılır.

## İlk adımlar

1. Bir hazır ayar açın: **Menü, Yeni, Office hazır ayarından profil**. Office katalog varsayılanlarını
   içerir; Strict kısıtlamalar ekler, Laptop taşınabilir bilgisayarlara uygundur, Home ise ev bilgisayarı
   için bir izin listesidir.
2. Soldaki ağaçta gezinin. Bir onay kutusu bir kuralı açar veya kapatır; bir grubun onay kutusu grubun
   kurallarını tek seferde açar veya kapatır; soluk görünen bir kural, ihtiyaç duyduğu başka bir kuralı
   bekler. Sağdaki panel seçili kuralın parametrelerini ve açıklamasını gösterir.
3. İlk kökün altındaki **Yükleme**, **Hesaplar** ve **Diller** formlarını doldurun.
4. Profili F7 ile denetleyin. İletiler gezinti satırının altında görünür; bir iletiye tıklamak, ilgili
   kuralı veya formu seçer.
5. Yanıt dosyasını F9 ile oluşturun: `autounattend.xml`, yürütülebilir dosyanın yanındaki `output`
   klasörüne yazılır; içerdiği betikler de incelemeniz için yanına konur. Dosyayı yükleme USB belleğinin
   köküne kopyalayın.
6. Profili Ctrl+S ile kaydedin. Profiller, yürütülebilir dosyanın yanındaki `profiles` klasöründe veya
   Farklı kaydet ile seçtiğiniz herhangi bir yerde bulunan `.wwprofile` dosyalarıdır.

## Pencere

- **Sekmeler**: her açık belge bir sekmedir: profil, karşılaştırma, bu kılavuz, yer işaretleri, MCP
  sunucusu, ayarlar. Sekme satırının sol ucundaki menü düğmesi Yeni, Aç, Kaydet, Karşılaştır, kılavuz,
  MCP sunucusu, yer işaretleri, Ayarlar ve Çıkış komutlarını içerir.
- Menüdeki **Hakkında ve Lisans**: derlemenin commit'iyle birlikte sürüm ve programa gömülü lisans (MIT);
  böylece yalnızca `winweaver.exe` taşınsa da lisans onunla kalır.
- **Komut çubuğu**: Yeni, Aç, Kaydet, Denetle, Oluştur, Bu bilgisayar, Karşılaştır. Dar bir pencerede
  yalnızca simgeleri gösterir ve bunlar gezinti satırına taşınır.
- **Gezinti satırı**: Geri ve İleri, seçtiğiniz öğeler arasında gezinir; içerik haritası nerede
  olduğunuzu gösterir ve herhangi bir düzeye atlar; Yer işareti konumu işaretler; Arama, siz yazdıkça
  ağacı filtreler ve aradıklarınızı hatırlar.
- **Ağaç ve ayrıntı paneli**: solda ağaç, sağda seçili öğe. Açıklama Ctrl+D ile daraltılır. Seçili bir
  grubun kuralları, satırlarının sonunda bir anahtar gösterir.
- **Durum çubuğu**: belgenin durumu, katalog sürümü, MCP sunucusu ve son ileti.

## Bu bilgisayar

Komut çubuğundaki **Bu bilgisayar**, bu bilgisayarın ayarlarını hiçbir şeyi değiştirmeden Windows
üzerinden okur:

- **Bu bilgisayarı yeni bir profile oku**: çalışan bir Windows'ta değerlendirilebilen her kural açık
  (geçerli) veya kapalı (kısmen geçerli ya da geçerli değil) olur ya da durumunu korur (okunamıyorsa
  veya temiz bir Windows'taki gibiyse); formlar sürümü, saat dilimini ve dilleri alır.
- **Seçimi bu profile oku** ve **Seçimi bu bilgisayarda denetle**, seçili kural veya grup üzerinde
  çalışır.
- **Uygulama betiğini**, **varsayılanlara dönüş betiğini** veya **denetim betiğini kaydet**, bu ya da
  başka bir bilgisayar için bir PowerShell betiği yazar. Uygulama veya dönüş betiğini yönetici olarak,
  önce bir test bilgisayarında çalıştırın; betik, yanındaki `Undo-Apply.ps1` dosyasının geri yüklediği
  bir yedek kaydeder. Bir denetim betiğinin raporu **Denetim raporu aç** ile açılır ve bir profile
  dönüşür.

Yalnızca yükleme sırasında veya bir kullanıcının ilk oturum açmasında etkili olan kurallar okunmaz.

## Profilleri karşılaştırma

**Menü, Bir dosyayla karşılaştır** veya **Bir hazır ayarla karşılaştır**, bir karşılaştırma sekmesi
açar: etkin profil ile diğeri arasındaki her fark üç sütunda gösterilir. "Sağ tarafı uygula", bu
değerleri karşılaştırmanın yapıldığı profilde ayarlar; "Fark dosyası olarak kaydet", farkları daha sonra
herhangi bir profile uygulanabilen bir `.wwdiff` dosyası olarak saklar.

## Diğer dosyaları açma

Aç komutu; profilleri (`.wwprofile`), karşılaştırmaları (`.wwdiff`), yer işareti dosyalarını
(`.wwmarks`), Markdown sayfalarını, WinWeaver veya WinKickOff tarafından oluşturulan yanıt dosyalarını
(içerdikleri profil), WinKickOff profillerini ve denetim raporlarını (`.json`) kabul eder.

## İlke şablonları

**Ayarlar, İlke şablonları** bu Windows'un, bir klasörün veya bir katalog dosyasının ADMX şablonlarını
içeri aktarır; ilkeleri Standart ilkeler altında kurallara dönüşür ve içeri aktarımlar `imports/` içinde
saklanır. Yalnızca işaretli içeri aktarımlar okunur ve yüklenir; diğerleri bellek kullanmaz. Bir profil,
yüklenmemiş bir içeri aktarımın seçeneklerini içerebilir: bunlar profilde değişmeden kalır, ağacın
sonunda **Bilinmeyen seçenekler** altında listelenir ve oluşturmaya alınmaz; profil açılırken ve yanıt
dosyası oluşturulurken bir uyarı verilir. Bu gruptaki **Katalogları yükle**, onları içeren içeri
aktarımları bulur ve yalnızca bunları yükler. Bir ayar (Genel, Pencere) grubu gizler.

Chrome, Edge ve Firefox'un yayımladığı biçimdeki bir şablon zip arşivi veya bir katalog dosyası arşivi
**Arşiv içeri aktar...** ile içeri aktarılır ve açılmadan okunur.

İki içeri aktarımda bulunan bir ilke her iki ağaçta da görünür. Bir katalog sekmesinin çubuğundaki
**Karşılaştır**, onu yüklenmiş başka bir içeri aktarımla veya açık bir katalog dosyasıyla yan yana koyar:
yalnızca birinde olan ilkeler ve başlığı, açıklaması, değerleri veya parametreleri farklı olanlar. Bir içeri
aktarımı kaldırmak, her kaldırma gibi, önce onay ister.

## MCP sunucusu

**Menü, MCP sunucusu**, yapay zeka aracılarının profilleri okuduğu ve mod izin verdiğinde değiştirdiği
sunucunun sekmesini açar. Sunucu okuma modunda başlar; düzenleme modu profilleri bellekte değiştirmesine,
dosyalar modu ise profilleri kaydetmesine ve yanıt dosyasını yazmasına izin verir. Bu bilgisayarı okumak,
her başlangıçta kapalı olan ayrı bir izin gerektirir.

## Oluşturucu

Pencere OpenGL veya DirectX 12 ile çizilir. **Otomatik** (Ayarlar, Genel, Pencere) önce daha az bellek
kullanan OpenGL'i dener, OpenGL başlamazsa DirectX 12'ye geçer: 3D hızlandırması olmayan bir sanal
makinede, ekran kartı sürücüsü olmayan yeni bir kurulumda, bazı uzak oturumlarda. DirectX 12 orada da
Windows'un yazılım rasterleştiricisiyle çizer. Pencere OpenGL ile hatalı çiziliyorsa (eski bir sürücü),
DirectX 12'yi seçin. Bir değişiklik sonraki başlatmada geçerli olur; `winweaver.exe --renderer directx`
(veya `opengl`, `auto`) yalnızca bir başlatma için seçer. `logs/` içindeki günlük, pencerenin neyle
çizildiğini belirtir.

## Komut satırı

`winweaver.exe` parametresiz pencereyi açar. Bir değer, parametresinden sonra bir boşluk veya `=` ile
yazılır: `--renderer=directx`.

| Parametreler | Ne yaparlar |
|---|---|
| `--renderer NAME` | Pencereyi açar ve yalnızca bu başlatma için önce NAME oluşturucusunu dener: `auto`, `opengl` veya `directx` |
| `--mcp stdio` | MCP'yi pencere olmadan standart giriş ve çıkış üzerinden sunar; bir MCP istemcisi onu böyle başlatır |
| `--mcp http --token TOKEN` | MCP'yi pencere olmadan `http://127.0.0.1:8765/mcp` adresinde sunar; belirteç en az 32 karakterdir |
| `--port N` | `--mcp http` ile: başka bir bağlantı noktası |
| `--mode NAME` | `--mcp` ile: `read` (varsayılan) yalnızca okur, `edit` profilleri bellekte değiştirir, `files` ayrıca kaydeder ve yanıt dosyasını yazar |
| `--read-pc` | `--mcp` ile: bu bilgisayarın ayarlarını okumaya izin verir |
| `--profile NAME` | `--mcp` ile: bir hazır ayar veya `profiles/` içindeki bir profil geçerli profil olur |
| `--version` | Sürümü yazdırır |
| `--check-catalog` | Yerleşik kataloğu çevirileriyle yükler ve sorunlarını yazdırır |
| `--help` | Parametre listesini yazdırır |
| `--smoke-window` | Küçük bir pencere açar, iki kare çizer ve bunları neyin çizdiğini yazdırır (`--renderer` ile de): grafik denetimi |
| `--elevated-apply PLAN DIGEST` | Pencere, onaylanmış bir planı yönetici olarak uygulamak için kendini böyle başlatır; elle başlatmak için değildir |

Mod ve bu bilgisayarı okuma izni her başlatmada verilir ve hiç kaydedilmez. İletiler programın başlatıldığı
konsola ve `logs/` klasörüne gider.

## Klavye

| Tuşlar | Eylem |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | Yeni, Aç, Kaydet, Farklı kaydet |
| F7, F9 | Profili denetle, yanıt dosyasını oluştur |
| Ctrl+F, Esc | Ara, aramayı temizle ve kapat |
| Alt+Left, Alt+Right | Geri, İleri |
| Ctrl+B | Konuma yer işareti koy |
| Ctrl+D | Açıklamayı daralt veya göster |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Sekmeyi kapat, sonraki sekme, önceki sekme |
| Esc | Bu bilgisayardaki bir değişikliğin onayını iptal et |
| Ctrl+, | Ayarlar |
| F1 | Bu kılavuz |

## Yürütülebilir dosyanın yanındaki dosyalar

`settings.json` (dil, tema, son kullanılan dosyalar, son oturumun sekmeleri), `profiles/`
(profilleriniz), `output/` (yanıt dosyası ve betikler), `drafts/` (bir sonraki başlangıçta geri yüklenen
kaydedilmemiş değişiklikler), `logs/` (programın yaptıkları), `bookmarks.wwmarks`. Başka hiçbir yere bir
şey yazılmaz.

## Henüz yok

Windows özelliklerini ve yeteneklerini doğrudan programdan okuma: o zamana kadar denetim betiği okur.
