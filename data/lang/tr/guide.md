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
- **Katalog**, üç bölümdeki yerleşik kurallardan ve içeri aktardığınız, onlar da kurala dönüşen ilke
  şablonlarından (ADMX) oluşur. **Yükleme**, yanıt dosyasının ve betiklerinin yaptıklarını; **İlkeler**,
  yalnızca kayıt defterine yazan her kuralı ve her içeri aktarımı; **Diğer** ise geri kalanı, yani
  çalışan bir Windows'taki hizmetleri, özellikleri, uygulamaları, programları ve betikleri içerir.
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

## Bilgisayarın adı

**Yükleme** formunda bilgisayar adı rastgele (Windows'un varsayılanı gibi), sabit ya da yanıt dosyasının
Kurulum sırasında her bilgisayarda genişlettiği bir şablondur. Şablon, şu yer tutucuların çevresinde
Latin harfleri, rakamlar ve kısa çizgiler içerir:

| Yer tutucu | Ne verir |
|---|---|
| `{serial}`, `{serial:N}` | Bilgisayarın donanım yazılımındaki seri numarası, harfleri ve rakamları büyük harfle; `:N` son N karakteri tutar, `:N` olmadan tümünü (en çok 15) |
| `{mac}`, `{mac:N}` | İlk fiziksel ağ bağdaştırıcısının MAC adresi, büyük harfle 12 onaltılık rakam; `:N` son N rakamı tutar, `:N` olmadan 12'sini de |
| `{random}`, `{random:N}` | I, O, 0 ve 1 gibi benzerleri olmadan rastgele büyük harfler ve rakamlar; `:N` bunlardan N tane verir, `:N` olmadan 6 |

N 1 ile 15 arasındadır. Okunamayan bir seri numarası veya adres (yoksa ya da üretici `Default string`
gibi bir yer tutucu bıraktıysa) aynı uzunlukta rastgele karakterlere dönüşür; daha kısa olan olduğu gibi
kalır. Her yer tutucunun tam uzunluğuyla ad 15 karaktere sığar; bu yüzden `PC-{serial}` çok uzun,
`PC-{serial:12}` sığar. Adda bir harf bırakın, çünkü Windows yalnızca rakamlardan oluşan bir adı
reddeder. `PC-{random}`, `PC-` ve altı rastgele karakter verir, örneğin `PC-K7Q2MX`; seri numarası
`5CG1234XYZ` ve adres `00155D3E5A01` ile `WS-{serial:6}` şablonu `WS-234XYZ`, `{mac:4}-LAB` şablonu
`5A01-LAB` verir. Form, alanın altında şablonun ne verdiğini gösterir.

## Giriş dilleri

**Diller** formu giriş dillerini sırasıyla listeler; ilki varsayılandır. Her dil adını ve bir klavye
düzeni listesini gösterir: önce Windows'un dile varsayılan olarak verdiği düzen, sonra Windows'un tüm
düzenleri adlarına göre. **Ekle**, Windows'un tanıdığı tüm diller arasında adının veya etiketinin bir
parçasıyla bir dil bulur ve seçilen düzenle ekler; aynı dil iki düzenle iki kez yer alabilir, örneğin
Ukraynaca (Genişletilmiş) ve Ukraynaca. Windows'un yalnızca ilk oturum açmada numaralandırdığı bir dil,
örneğin Rusça (Ukrayna), Windows'un o zaman verdiği düzeni alır; o oturum açmaya kadar yanıt dosyası
dilin ait olduğu dili adlandırır. Diller ve düzenler, WinWeaver'ın çalıştığı Windows'tan, onun dilindeki
adlarıyla gelir.

## Kullanıcı dosyaları

Bir kullanıcı dosyası (`.wwusers`), bir bilgisayarda oluşturulacak yerel hesapları, aynı dosyada bir ana
parolayla şifrelenmiş parolalarıyla birlikte listeler. **Menü, Yeni, Kullanıcı dosyası** boş bir dosya
başlatır; **Bu bilgisayarın hesaplarından kullanıcı dosyası** ise bu bilgisayarın tüm normal hesaplarıyla
(grup, durum, parola süresi, profil klasörü; parola yok) bir dosya başlatır. Sekme hesapları bir listede,
seçileni ise bir formda gösterir: ad, tam ad, açıklama, grup, profil klasörü ve bilgisayarda zaten var
olan bir hesap için talimatlar (dosyadaki parolayı ayarla, etkinleştir veya devre dışı bırak, tam olarak
bu grup); bunlar olmadan var olan hesap olduğu gibi bırakılır. **Kilidi aç** ana parolayı sorar ya da
dosyada yoksa belirler; **Parola** seçili hesabın parolasını ayarlar; **Kilitle** onları unutur. Ana
parola olmadan parolalar bir daha okunamaz. **Denetle** sorunları listeler, **Bu bilgisayarı oku**
dosyada eksik olan bu bilgisayarın hesaplarını ekler. **Bu bilgisayarda oluştur** ve **Parolaları
uygula** bir sorudan sonra bu bilgisayarı değiştirir ve WinWeaver'ın yönetici olarak çalıştırılmasını
gerektirir; sonuç formun altında, hesap başına bir satır olarak durur. **Betik**, kullanıcı dosyasının
yanına `Create-<ad>.ps1` kaydeder: ana parolayı kendisi sorar ve yönetici olarak açılmış bir komut
isteminden başka bir bilgisayarda aynısını yapar (`-PasswordsOnly` yalnızca parolaları ayarlar). Bir
profilin **Hesaplar** formunda **Kullanıcı dosyasından**, ana parolayı verirseniz parolalarıyla birlikte
hesaplarını yükler.

Dosyanın kilidi açıkken form, seçili hesabın parolasını nokta olarak gösterir; **Göster** ve **Kopyala**
düğmeleri vardır (Windows pano geçmişi kopyalananı saklar). Bir profilin **Hesaplar** formunda
**Kullanıcı dosyasına**, hesaplarından bir kullanıcı dosyası oluşturur; parolaları ilk kayıttan önce bir
ana parola bekler.

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
- **Bağlantılar**: bir görünümün adını verdiği her kural, grup, katalog, profil dosyası ve kayıt defteri
  anahtarı kendisine götürür: bir kuralın ihtiyaç duyduğu kurallar, ona ihtiyaç duyanlar ve onunla
  çakışanlar; iletilerde adı geçen kurallar; bir grubun üstündeki ve içindeki gruplar; diğer
  kataloglardaki aynı ilke; bir kayıt defteri anahtarını yazan kurallar, kayıt defteri sekmesinde ve
  kayıt dosyasında. Profildeki bir bağlantı ağacını öğeye kadar açar; başka bir sekmeden kuralı ilk
  profil sekmesinde gösterir.
- **Durum çubuğu**: belgenin durumu, katalog sürümü, MCP sunucusu ve son ileti.
- **Klasörü aç**: bir derlemeden ya da bir betik, kayıt dosyası veya profil kaydedildikten sonra durum
  çubuğunun sonundaki düğme dosyayı Gezgin'de klasöründe gösterir; Ayarlar, Dosyalar programın
  klasörlerini açar.

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
- **Kayıt dosyasını kaydet** veya **varsayılanlara dönüş kayıt dosyasını kaydet**, seçimin kayıt defteri
  değerlerini regedit veya `reg import` için bir `.reg` dosyası olarak yazar; bir kayıt dosyasının
  tutamadıkları (betik adımları, uygulamalar, özellikler) içinde yorum olarak kalır. Varsayılan kullanıcı
  profilinin değerleri `HKEY_USERS\UnattendDefault` altına gider ve dosyanın başındaki yorum, kovanının
  oraya nasıl yükleneceğini anlatır.

Yalnızca yükleme sırasında veya bir kullanıcının ilk oturum açmasında etkili olan kurallar okunmaz.

Windows özellikleri ve yetenekleri, WinWeaver yönetici olarak çalıştığında DISM üzerinden okunur; bu,
okumaya birkaç saniye ekler. Bu haklar olmadan okunamazlar.
Okuma arka planda yürür: pencere çalışmaya devam eder ve durum çubuğundaki bir gösterge bunu belirtir.

Yönetici olarak uygulandığında, Windows uygulamalarını veya yeteneklerini kaldıran bir kural onları
pencereden de kaldırır: yetenekleri DISM üzerinden, uygulamaları tüm kullanıcılar ve sonradan oluşturulan
hesaplar için. Yedek onları listeler, ancak Geri al onları geri getirmez.

Kuralların betik adımları da PowerShell olmadan yerel olarak uygulanır: yükleme ortamından .NET Framework
3.5 ve sürücüler, hizmetlerin başlangıç türleri, NetBIOS, gezinti bölmesindeki çıkarılabilir sürücüler,
profil hesapları için "parolanın süresi hiç dolmaz", yerleşik Administrator ve Guest hesaplarının devre
dışı bırakılması (kullanılan hesap asla) ve yanıt dosyası kopyalarının silinmesi. Geri al; çalışan
programları, değişen hesapları ve silinen dosyaları geri almaz; onay, bir planda bu türden kaç adım
olduğunu söyler.

## Profilleri karşılaştırma

**Menü, Bir dosyayla karşılaştır** veya **Bir hazır ayarla karşılaştır**, bir karşılaştırma sekmesi
açar: etkin profil ile diğeri arasındaki her fark üç sütunda gösterilir. "Sağ tarafı uygula", bu
değerleri karşılaştırmanın yapıldığı profilde ayarlar; "Fark dosyası olarak kaydet", farkları daha sonra
herhangi bir profile uygulanabilen bir `.wwdiff` dosyası olarak saklar.

**Karşılaştır, Kaydedilmemiş değişiklikler**, profilin dosyasıyla ya da geldiği hazır ayarla farklarını
bir sekmede açar: solda profilin şimdiki hali, sağda kaydedildiği hali.

## Diğer dosyaları açma

Aç komutu; profilleri (`.wwprofile`), karşılaştırmaları (`.wwdiff`), yer işareti dosyalarını
(`.wwmarks`), Markdown sayfalarını, profil içeren yanıt dosyalarını (içerdikleri profil), JSON
biçimindeki eski profilleri ve denetim raporlarını (`.json`) kabul eder.

Bir kayıt dosyası (`.reg`), kayıt defteri bölümünde anlatıldığı gibi kendi sekmesinde açılır.

## İlke şablonları

**Ayarlar, İlke şablonları** bu Windows'un, bir klasörün veya bir katalog dosyasının ADMX şablonlarını
içeri aktarır; ilkeleri İlkeler altında kurallara dönüşür ve içeri aktarımlar `imports/` içinde
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

**Menü, Kayıt defteri** bu bilgisayarın kayıt defterini bir sekmede açar: anahtarlar solda, seçili
anahtarın değerleri türleriyle (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD, REG_BINARY)
sağda, yol üstteki satırda. Çift tıklama bir değeri düzenler; Yeni anahtar, Yeni değer, Düzenle ve Sil
çubuktadır ve silme önce onay ister. Bilgisayarın anahtarları (HKEY_LOCAL_MACHINE) yalnızca WinWeaver
yönetici olarak çalıştığında değişir. **Ne yazar** içindeki bir anahtarın yanındaki düğme onu burada
açar. **Yer imi** anahtarı ve değeri bir yorumla saklar, yer imi de onu yazan kuralları listeler.
Çubuktaki **Bul** seçili anahtarın altında regedit'in araması gibi arar: anahtar adlarında, değer
adlarında ve verilerde, istenirse yalnızca dizenin tamamında. Arama arka planda yürür, **Durdur** onu
bitirir ve bulunan bir yere tıklamak onu gösterir.

Çubuktaki **İzinler**, seçili anahtarın sahibini ve erişim listesini gösterir: devralınan girdiler
gösterilir, anahtarın kendi girdileri değişir (izin ver veya reddet, tam denetim veya okuma, nelere
uygulandığı), bu bilgisayarın bir hesabı veya grubu yeni bir girdi alır ve üstteki anahtardan devralma
kapatılabilir. Uygula önce sorar; bilgisayarın bir anahtarı yalnızca WinWeaver yönetici olarak çalışırken
değişir.

Bir kayıt dosyası (`.reg`) kendi sekmesinde metin olarak değil, yapı olarak açılır: solda dosyanın
anahtarları ve yorumları sırasıyla, sağda seçili anahtar, regedit türlerindeki değerleri ve aralarındaki
yorumlarla, kayıt defteri sekmesine bir bağlantı ve anahtarı ya da seçili değeri yazan yüklü
kataloglardaki kurallar. **Yeni anahtar**, **Yeni değer**, **Düzenle** ve **Sil** çubuktadır; silme önce
sorar. **Kaydet** her satırı geri yazar: yorumlar ve programın okumadığı satırlar yerinde kalır. **Bu
bilgisayar** ile kaydedilen bir dosya kurallarını yorumlarda adlandırır ve böyle bir yorumun oku, kuralı
profilde gösterir.

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
| `--script KIND FILE` | Pencere olmadan bir profilin (`.wwprofile`) veya bir farkın (`.wwdiff`) bağımsız betiğini yazar: `apply`, `revert` (Windows varsayılanlarına dönüş) veya `audit`; uygulama veya dönüş betiğinin yanına `Undo-Apply.ps1` konur |
| `--reg KIND FILE` | Bir profilin veya farkın uygulama (`apply`) ya da varsayılanlara dönüş (`revert`) kayıt dosyasını yazar |
| `--only IDS` | `--script` veya `--reg` ile: yalnızca bu kurallar veya gruplar, kimlikleri virgülle ayrılmış |
| `--out FOLDER` | `--script` veya `--reg` ile: dosyaların klasörü, varsayılan olarak programın yanındaki `output` |
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

`winweaver.settings.json` (dil, tema, son kullanılan dosyalar, son oturumun sekmeleri), `profiles/`
(profilleriniz), `output/` (yanıt dosyası ve betikler), `drafts/` (bir sonraki başlangıçta geri yüklenen
kaydedilmemiş değişiklikler), `mcp/` (MCP sunucusunun yapılandırmaları), `logs/` (programın yaptıkları), `bookmarks.wwmarks`. Başka hiçbir yere bir
şey yazılmaz.

Ayarlar, yer işaretleri, taslaklar ve fark dosyaları bu klasördeki bir dosyanın yolunu klasöre göre
saklar; böylece klasörün tamamı taşınabilir veya başka bir bilgisayara kopyalanabilir. Başka yerdeki bir
dosya tam yolunu korur.

## Henüz yok

Şimdilik plandan eksik bir şey yok: programın bundan sonra neye ihtiyacı olduğunu kullanım gösterecek.
