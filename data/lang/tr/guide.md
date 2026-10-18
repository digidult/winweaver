# WinWeaver kılavuzu

WinWeaver, Windows 10 ve 11'in gözetimsiz kurulumlarını hazırlar ve kurulu bilgisayarların yapılandırmasını
korur. Bir profil, kataloğun hangi kurallarının hangi parametrelerle açık olduğunu ve kurulum formlarında ne
bulunduğunu söyler; program ondan betikleriyle birlikte `autounattend.xml` yanıt dosyasını oluşturur,
profilleri karşılaştırır, çalışan bir bilgisayarın ayarlarını okur ve ona uygular. Katalog, etki alanı
olmayan küçük ofisler için yazılmıştır: önce güvenlik ve güncellemeler.

Bu sayfa ilk başlangıçta, daha sonra da menüden veya F1 ile açılır. Bir Markdown dosyasıdır; belge
sayfaları da aynı şekilde, her biri kendi sekmesinde açılır.

## Program nasıl düzenlenmiştir

- **Profiller** (`.wwprofile`) seçimlerinizi tutar; Office, Strict, Laptop ve Home hazır ayarları başlangıç
  noktalarıdır.
- **Katalog**, üç bölümdeki yerleşik kurallardan (Kurulum, Standart ilkeler, Diğer) ve içeri aktardığınız,
  onlar da kurala dönüşen ilke şablonlarından (ADMX) oluşur.
- **Sekmeler**: her belge kendi sekmesinde açılır: profiller, karşılaştırmalar, kataloglar, yer imleri, MCP
  sunucusu, bunun gibi sayfalar.
- **Bu bilgisayar** çalışan Windows'u okur, kuralları onda denetler, onları bir yedekle uygular ve başka
  bilgisayarlar için betikler oluşturur.
- **Programın klasörü** her şeyi `winweaver.exe` dosyasının yanında tutar (bu sayfanın sonuna bakın); hiçbir
  şey kurulmaz.

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
7. Yanıt dosyasını herhangi bir gerçek bilgisayardan önce bir sanal makinede deneyin: kurun, sonra o
   bilgisayarı **Bu bilgisayar** ile okuyup profille karşılaştırın.
8. Kurulu bilgisayarları düzende tutun: **Bu bilgisayar** seçili kuralları yönetici olarak bir yedekle
   uygular veya başka bilgisayarlarda uygulayan, geri alan, denetleyen ve geri döndüren bağımsız betikler
   kaydeder.

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

Windows özellikleri ve yetenekleri, WinWeaver yönetici olarak çalıştığında DISM üzerinden okunur; bu,
okumaya birkaç saniye ekler. Bu haklar olmadan okunamazlar.
Okuma arka planda yürür: pencere çalışmaya devam eder ve durum çubuğundaki bir gösterge bunu belirtir.

Yönetici olarak uygulandığında, Windows uygulamalarını veya yeteneklerini kaldıran bir kural onları
pencereden de kaldırır: yetenekleri DISM üzerinden, uygulamaları tüm kullanıcılar ve sonradan oluşturulan
hesaplar için. Yedek onları listeler, ancak Geri al onları geri getirmez.

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

Bir seçeneğin ayrıntı paneli, onu içeren katalog sekmelerine ve diğer içeri aktarımların ağaçlarındaki aynı
ilkeye bağlantı verir; seçeneğin yer imi de öyle. Ayarlar, Genel'de açıklamanın hangi bölümlerinin baştan
açık olacağı seçilir.

Bir katalog sekmesinin çubuğundaki **Düzenle** kataloğu bir düzenleyicide açar, şablon menüsündeki **Yeni katalog**
de size ait boş bir katalog başlatır. Bir ilke başlığını, açıklamasını, bilgisayarı veya varsayılan kullanıcı
profilini ve yazdığı ya da kaldırdığı kayıt defteri değerlerini alır; **Uygula** tüm dosyayı denetler ve denetim
başarısız olursa hiçbir şey değişmez. Kaydet (Ctrl+S) dosyayı yazar ve yeniden yükler.

## Kayıt defteri

**Menü, Kayıt defteri** bu bilgisayarın kayıt defterini bir sekmede açar: anahtarlar solda, seçili anahtarın
değerleri türleriyle (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD, REG_BINARY) sağda, yol üstteki
satırda. Çift tıklama bir değeri düzenler; Yeni anahtar, Yeni değer, Düzenle ve Sil çubuktadır ve silme önce
onay ister. Bilgisayarın anahtarları (HKEY_LOCAL_MACHINE) yalnızca WinWeaver yönetici olarak çalıştığında
değişir. **Ne yazar** içindeki bir anahtarın yanındaki düğme onu burada açar. **Yer imi** anahtarı ve değeri bir
yorumla saklar, yer imi de onu yazan kuralları listeler. Çubuktaki **Bul** seçili anahtarın altında regedit'in
araması gibi arar: anahtar adlarında, değer adlarında ve verilerde, istenirse yalnızca dizenin tamamında.
Arama arka planda yürür, **Durdur** onu bitirir ve bulunan bir yere tıklamak onu gösterir. Anahtarların
hakları daha sonra gelecek.

## MCP sunucusu

**Menü, MCP sunucusu**, yapay zekâ aracılarının profilleri okuduğu ve mod izin verdiğinde değiştirdiği sunucunun sekmesini açar. Sekme, programın yanındaki `mcp/default.wwmcp` yapılandırma dosyasını düzenler ve her değişiklikte kaydeder: bağlantı noktası ve belirteç, mod (read yalnızca okur, edit profilleri bellekte değiştirir, files ayrıca onları kaydeder ve yanıt dosyasını yazar), bu bilgisayarı okuma, kayıt defterini okuma veya değiştirme ve yer imlerini ile katalogları değiştirme izinleri, sunucunun başladığı profil ve ayrıntılı günlük. **Aç** başka bir `.wwmcp` dosyası alır, **Farklı kaydet** bir kopya oluşturur; `winweaver --mcp FILE` pencere olmadan dosyanın söylediği gibi hizmet verir. Çalışan sunucunun günlüğü her çağrıyı listeler; **Ayrıntıları göster** sağında seçili çağrının argümanlarını ve yanıtını, andığı kurallara, profillere, kataloglara ve kayıt defteri anahtarlarına giden düğmelerle gösteren bir panel açar. Sunucunun günlüğü `logs/mcp-<tarih>.jsonl` dosyasıdır, satır başına bir JSON nesnesi; varsayılan olarak kapalı ayrıntılı günlük, her isteği ve yanıtı parolalar, anahtarlar ve belirteçler olmadan eksiksiz tutar.

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
| `--registry NAME` | `--mcp` ile: `off` (varsayılan), `read` araçların kayıt defterini okumasına, `write` değiştirmesine de izin verir |
| `--edit-library` | `--mcp` ile: araçlar yer imlerini ve katalog dosyalarını değiştirebilir; okumak her zaman serbesttir |
| `--profile NAME` | `--mcp` ile: bir hazır ayar veya `profiles/` içindeki bir profil geçerli profil olur |
| `--mcp FILE` | MCP'yi pencere olmadan bir yapılandırma dosyasının söylediği gibi sunar: bir `.wwmcp` dosyasının yolu veya programın yanındaki `mcp` klasöründen bir ad |
| `--config FILE` | `--mcp stdio` veya `--mcp http` ile: bir yapılandırma dosyasının ayarları, belirteç de; diğer seçenekler onları bu başlatma için değiştirir |
| `--detailed-log` | `--mcp` ile: günlük her isteği ve yanıtı parolalar, anahtarlar ve belirteçler olmadan eksiksiz tutar |
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
kaydedilmemiş değişiklikler), `mcp/` (MCP sunucusunun yapılandırmaları), `logs/` (programın yaptıkları), `bookmarks.wwmarks`. Başka hiçbir yere bir
şey yazılmaz.

## Henüz yok

Bazı kuralların betik adımları ve diğer kullanıcıların profilleri, pencereden uygulandığında: o zamana
kadar bunu uygulama betiği yapar.
