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

Varsayılan kullanıcı profilindeki **Kullanıcı bölgesi** (`default-user.region`) kuralı, yeni kullanıcıların ülkesinin GeoID'sini ve iki harfli kodunu tutar. İki alanı da serbestçe yazılır ve her birinin yanındaki liste, Windows'un bildiği bütün bölgeleri yazılana göre süzülmüş olarak içerir: bir seçim GeoID ve kodu birlikte ayarlar, bölgenin adı alanlardan sonra gösterilir. Denetim, iki değer listedeki tek bir bölgeyi adlandırmadığında uyarır.

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
hesaplarını yükler. Formda grubun değişmesi tam olarak bu grubu işaretler, böylece hesap Administrators ile Users arasında geçer; WinWeaver'ın çalıştığı hesap Administrators'da kalır.

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

- **Menüler ve sekmeler**: her menü öğesinin bir simgesi vardır. Sekmeler pencereden genişse satır
  tekerlekle kayar, etkin sekme görünüme gelir ve sağ uçtaki bir düğme tüm sekmeleri listeler.
- Profil çubuğundaki **Yalnızca açık** yalnızca açık kuralları, grupları açılmış olarak gösterir; MCP
  araçları `list_groups`, `list_rules` ve `search_rules` de aynı şekilde `only_on` alır.
- **Parametreler**: bir kuralın alanları panelin genişliğini kaplar; yanıt dosyasının sınırı olan bir
  metin (destek kişileri) karakterlerini sayar.
- **Arka plandaki iş**: bu bilgisayarın okunması, ilke şablonlarının içeri aktarılması, bu bilgisayarın
  değiştirilmesi ve kayıt defterinde arama, sağ alt köşedeki bir panelde geçen süreyle ve durabilen
  işlerde Durdur ile gösterilir; panel bir göstergeye daralır.

## Bu bilgisayar

Komut çubuğundaki **Bu bilgisayar**, bu bilgisayarın ayarlarını hiçbir şeyi değiştirmeden Windows
üzerinden okur:

**Profil.** Ağaçtaki ilk form olan Profil, profilin adını, yazarını ve yorumunu düzenler; gezinti satırının ilk bölümü profili formlarıyla kısaca gösterir. Bir şablon pencerenin dili ve Windows'unuzun bölgesiyle ve bu Windows'un giriş dilleriyle açılır (okunamazlarsa English International). **Menü, Bu bilgisayar** her sekmeden çalışır: yeni bir profile okumak için açık profil gerekmez.

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

Bir profilin çubuğundaki **Kataloglar** menüsü, yüklü olsun olmasın `imports/` içindeki her katalog dosyasını listeler: yüklü olanın bir işareti ve vurgu rengi vardır, tıklamak sekmesini açar; diğeri tıklamayla yüklenir ve o da sekmesini açar. Liste, menü her açıldığında yeniden okunur. Varsayılan olarak menüdeki bir kataloğa tıklama onu yükler veya kaldırır; Ayarlar, Genel'deki diğer seçenek menüde yalnızca yüklü katalogları bırakır ve tıklama birinin sekmesini açar.

Chrome, Edge ve Firefox'un yayımladığı biçimdeki bir şablon zip arşivi veya bir katalog dosyası arşivi
**Arşiv içeri aktar...** ile içeri aktarılır ve açılmadan okunur.

İki içeri aktarımda bulunan bir ilke her iki ağaçta da görünür. Bir katalog sekmesinin çubuğundaki
**Karşılaştır**, onu yüklenmiş başka bir içeri aktarımla veya açık bir katalog dosyasıyla yan yana koyar:
yalnızca birinde olan ilkeler ve başlığı, açıklaması, değerleri veya parametreleri farklı olanlar. Bir içeri
aktarımı kaldırmak, her kaldırma gibi, önce onay ister.

Bir seçeneğin ayrıntı paneli, onu içeren katalog sekmelerine ve diğer içeri aktarımların ağaçlarındaki aynı
ilkeye bağlantı verir; seçeneğin yer imi de öyle. Ayarlar, Genel'de açıklamanın hangi bölümlerinin baştan
açık olacağı seçilir.

Bir katalog sekmesinin çubuğundaki **Düzenle** kataloğu bir düzenleyicide açar, şablon menüsündeki **Yeni
katalog** de size ait boş bir katalog başlatır. Bir ilke, yerleşik kataloğun bir kuralı gibi bütünüyle
düzenlenir: başlığı ve açıklaması, grubu, ne zaman çalıştığı (yanıt dosyasının bir geçişi, kurulum betiği,
varsayılan kullanıcı profili, ilk oturum açma, OOBE sonrası), düzeyi, **Ayrıntılar** altındaki metinler,
parametreleri ve her türden eylemi (kayıt defteri değerleri, yanıt dosyasının ayarları ve komutları, betik
satırları, hizmetler, özellikler, yetenekler, uygulamalar, programlar), **Çeviriler** altında da başka bir
dildeki her metin; boş bir alanda İngilizcesi gösterilir. Bir grup başlığını, açıklamasını ve yerini alır:
yanıt dosyasının bir ilkesi Kurulum altında, yalnızca kayıt defterine yazan biri İlkeler altında, geri kalanı
Diğer altında yer alır. **Uygula** tüm dosyayı yerleşik katalog nasıl denetleniyorsa öyle denetler ve denetim
başarısız olursa hiçbir şey değişmez. Kaydet (Ctrl+S) dosyayı yazar ve yeniden yükler. Seçili grup, bir
sorudan sonra **Grubu kaldır** ile içindeki tüm grup ve ilkelerle birlikte kaldırılır; dosyanın en üst grubu
kalır. Kendi betikleri, komutları ya da programları olan bir katalog yalnızca bir sorudan sonra içe aktarılır:
böyle bir kataloğu yalnızca güvendiğiniz bir kaynaktan içe aktarın.

Karşılaştırma bir sekmede açılır: farklı ilkeler kategorilerinin ağacında, yalnızca ilk katalogda,
yalnızca ikincisinde olanlar ve farklı olanlar için anahtarlarla; seçili ilke her iki katalogdaki haliyle
yan yana. **Raporu kaydet** raporu Markdown dosyası olarak yazar, **Sayfa olarak** sayfa olarak gösterir.
Katalog sekmesinin çubuğundaki **Şuraya kopyala**, seçili ilkeyi ya da seçili grubu altındaki gruplarla
birlikte düzenleyicide açık bir kataloğa (birkaçı açıksa hangisini seçersiniz) ya da yeni bir kataloğa
aynı kimlikler ve diğer dillerin metinleriyle kopyalar: yalnızca gereken ilkelerin kataloğu, dosya olarak
aktarmak için.

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
profilde gösterir. Dosyanın önce silip sonra yeniden yazdığı bir anahtar (anahtarının değerlerini
değiştiren bir liste böyle kaydedilir) ağaçta bir kez, yeniden oluşturulduğunu gösteren ↻ işaretiyle ve
aldığı değerlerle görünür.

Bir Grup İlkesi dosyası (`Registry.pol`) da kendi sekmesinde açılır: bu Windows'un bilgisayar ve
kullanıcı dosyaları (**Menü, Grup İlkesi**; profil, katalog ve kayıt defteri çubuğu) ya da herhangi bir
`.pol` dosyası. Anahtarlar solda bir ağaç, seçili anahtarın girdileri sağda her birinin ne yaptığıyla
birlikte. **Yeni anahtar**, **Yeni değer**, **Düzenle** (ya da çift tıklama) ve **Kaldır** girdileri
sekmenin üzerindeki bir pencerede değiştirir, kayıt defteri dosyasının değerleri gibi; **Kök**, Grup
İlkesi dışındaki bir dosyanın bilgisayara mı kullanıcıya mı uygulanacağını seçer (bu Windows'un dosyaları
bunu klasöründen alır); **Dosya ekle** başka bir Grup İlkesi dosyasının girdilerini ya da o kökün bir
kayıt defteri dosyasının değişikliklerini alır; **.reg olarak** dosyayı kayıt defteri dosyası olarak
kaydeder. Kaydet, Grup İlkesi biçimini geri yazar; bu Windows'un dosyaları, onları Grup İlkesi'nin kendisi
yazdığı için başka yere kopya olarak kaydedilir. Kaydedilmemiş değişiklikler yeniden başlatmadan sonra
taslaktan geri gelir.

## MCP sunucusu

**Menü, MCP sunucusu**, yapay zeka ajanlarının profilleri okuduğu ve mod izin verdiğinde değiştirdiği sunucunun sekmesini açar. Sekme, programın yanındaki `mcp/default.wwmcp` yapılandırma dosyasını düzenler ve her değişiklikte kaydeder: bağlantı noktası ve belirteç, mod (read yalnızca okur, edit profilleri bellekte değiştirir, files bunları ayrıca kaydeder ve yanıt dosyasını yazar) ve ayrıntılı günlük. **Araçlar** altında her araç tek başına ya da grubuyla birlikte kapatılabilir ve WinWeaver kartı seçeneklerini içerir: bu bilgisayarı okuma, kayıt defterini okuma ya da değiştirme, yer imlerini ve katalogları değiştirme ve sunucunun başladığı profil. **Bir aracı çağır**, herhangi bir aracı bir ajanın yapacağı gibi adı ve bağımsız değişkenleriyle çalıştırır ve yanıtı gösterir. **Aç** başka bir `.wwmcp` dosyası alır ve **Farklı kaydet** bir kopya oluşturur (WinWeaver'ın önceki bir sürümüne ait dosya açılırken yeni biçimde yazılır); `winweaver --mcp FILE` dosyanın dediği gibi penceresiz hizmet verir. Günlük her çağrıyı listeler; **Ayrıntıları göster** sağında seçili çağrının bağımsız değişkenlerini ve yanıtını, adını verdiği kurallara, profillere, kataloglara ve kayıt defteri anahtarlarına giden düğmelerle gösteren bir panel açar. Sunucunun günlüğü `logs/mcp-<date>.jsonl` dosyasıdır, satır başına bir JSON nesnesi, ve **Günlüğü aç** onu gösterir; varsayılan olarak kapalı olan ayrıntılı günlük, her isteği ve yanıtı parolalar, anahtarlar ve belirteçler olmadan eksiksiz tutar.

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
| `--build PROFILE` | Pencere olmadan bir profili denetler ve yanıt dosyasını oluşturur: `output` içinde `autounattend.xml` ve yanında gömdüğü betikler; PROFILE bir `.wwprofile` dosyası, bir hazır ayar (`Office`, `Strict`, `Laptop`, `Home`) veya `profiles/` içindeki bir profilin adıdır; denetimin bir hatası oluşturmayı durdurur |
| `--import-admx SOURCE` | İlke şablonlarını Ayarlar, İlke şablonları gibi içeri aktarır: `system` (bu Windows'un şablonları), ADMX dosyalarından oluşan bir klasör, dil klasörlerinin ADML'leriyle bir `.admx` dosyası, bir `.zip` arşivi veya bir `.wwcatalog` dosyası; katalog dosyası `imports/` içine konur, pencere ve MCP sunucusu onu bir sonraki başlatmalarında yükler (pencere kapalıyken çalıştırın); kuralları kendi kodunu çalıştıran bir katalog `--code allow` gerektirir |
| `--export-catalog SOURCE` | Ayarlar, İlke şablonları içindeki Dışarı aktar gibi bir katalog dosyası (`.wwcatalog`) yazar: kimliğiyle bir içeri aktarımın (`imports/` içindeki dosyasının adı) veya içeri aktarılmadan okunan bir `--import-admx` kaynağının; `output` içinde `<id>.wwcatalog` |
| `--read-pc` | `--mcp` olmadan: bu bilgisayarı Bu bilgisayar, Bu bilgisayarı oku gibi bilgisayarın adını taşıyan yeni bir profile okur ve `profiles/` içine kaydeder; `--out` içinde bir `.xml` dosyasıyla bunun yerine o profilin yanıt dosyasını yazar; Windows özellikleri ve yetenekleri yalnızca program yönetici olarak çalıştığında okunur |
| `--base PROFILE` | `--read-pc` ile: okumanın başladığı profil, `--build` için olduğu gibi verilir; varsayılan olarak Office hazır ayarı |
| `--diff A B` | A profilinden B profiline olan farkları bir fark dosyası olarak yazar: `profiles/` içinde `A-vs-B.wwdiff`; A ve B, `--build` için olduğu gibi verilir |
| `--script KIND FILE` | Pencere olmadan bir profilin (`.wwprofile`) veya bir farkın (`.wwdiff`) bağımsız betiğini yazar: `apply`, `revert` (Windows varsayılanlarına dönüş) veya `audit`; uygulama veya dönüş betiğinin yanına `Undo-Apply.ps1` konur |
| `--reg KIND FILE` | Bir profilin veya farkın uygulama (`apply`) ya da varsayılanlara dönüş (`revert`) kayıt dosyasını yazar |
| `--only IDS` | `--script` veya `--reg` ile: yalnızca bu kurallar veya gruplar, kimlikleri virgülle ayrılmış |
| `--out FILE` | `--build`, `--export-catalog`, `--read-pc`, `--diff`, `--script` veya `--reg` ile: değer komutun yazdığı uzantıyla bitiyorsa (`.xml`, `.wwcatalog`, `.wwprofile`, `.wwdiff`, `.ps1`, `.reg`) yazılacak dosya; başka her değer, eksikse oluşturulan ve dosyanın olağan adıyla yazıldığı bir klasördür; `Undo-Apply.ps1` ve yanıt dosyasının betikleri dosyanın yanına konur; `--out` olmadan dosyalar programın yanındaki `output` klasörüne gider (`--read-pc` ve `--diff` için `profiles/`) |
| `--help` | Parametre listesini yazdırır |
| `--smoke-window` | Küçük bir pencere açar, iki kare çizer ve bunları neyin çizdiğini yazdırır (`--renderer` ile de): grafik denetimi |
| `--elevated-apply PLAN DIGEST` | Pencere, onaylanmış bir planı yönetici olarak uygulamak için kendini böyle başlatır; elle başlatmak için değildir |

Dosya yazan bir komut yazdığı her dosyayı adlandırır ve 0 koduyla biter; parametrelerin yanlış kullanımı 2
koduyla, bir hata 1 koduyla biter ve nedeni yazdırılır.

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
