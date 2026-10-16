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

## MCP sunucusu

**Menü, MCP sunucusu**, yapay zeka aracılarının profilleri okuduğu ve mod izin verdiğinde değiştirdiği
sunucunun sekmesini açar. Sunucu okuma modunda başlar; düzenleme modu profilleri bellekte değiştirmesine,
dosyalar modu ise profilleri kaydetmesine ve yanıt dosyasını yazmasına izin verir. Bu bilgisayarı okumak,
her başlangıçta kapalı olan ayrı bir izin gerektirir.

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

Ayarları doğrudan programdan bu bilgisayara uygulamak (bu arada uygulama betiğini kullanın) ve ilke
şablonlarını (ADMX) içeri aktarmak sonraki sürümlerle gelecek.
