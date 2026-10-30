# Panduan WinWeaver

WinWeaver menyiapkan instalasi otomatis Windows 10 dan 11 serta menjaga konfigurasi PC yang sudah
terpasang. Sebuah profil menyatakan aturan katalog mana yang aktif, dengan parameter apa, dan apa isi
formulir instalasi; dari profil itu program membuat file jawaban `autounattend.xml` beserta skripnya,
membandingkan profil, membaca pengaturan PC yang berjalan, dan menerapkannya. Katalog ini ditulis untuk
kantor kecil tanpa domain: keamanan dan pembaruan lebih dulu.

Halaman ini terbuka saat program pertama kali dimulai, dan selanjutnya dari menu atau dengan F1. Halaman
ini adalah file Markdown; halaman dokumentasi terbuka dengan cara yang sama, masing-masing di tabnya sendiri.

## Cara program tersusun

- **Profil** (`.wwprofile`) menyimpan pilihan Anda; preset Office, Strict, Laptop, dan Home menjadi titik
  awal.
- **Katalog** terdiri dari aturan bawaan dalam tiga bagian dan templat kebijakan (ADMX) yang Anda impor,
  yang juga menjadi aturan. **Instalasi** memuat apa yang dilakukan file jawaban dan skripnya;
  **Kebijakan** memuat setiap aturan yang hanya menulis registri dan setiap impor; **Lainnya** memuat
  sisanya: layanan, fitur, aplikasi, program, dan skrip di Windows yang sedang berjalan.
- **Tab**: setiap dokumen terbuka di tabnya sendiri: profil, perbandingan, katalog, penanda, server MCP,
  halaman seperti ini.
- **PC Ini** membaca Windows yang berjalan, memeriksa aturan di sana, menerapkannya dengan cadangan, dan
  membuat skrip untuk PC lain.
- **Folder program** menyimpan semuanya di samping `winweaver.exe` (lihat akhir halaman ini); tidak ada
  yang diinstal.

## Langkah pertama

1. Buka preset: **Menu, Baru, Profil dari preset Office**. Office berisi default katalog;
   Strict menambahkan pembatasan, Laptop cocok untuk PC portabel, Home adalah daftar izin untuk PC rumah.
2. Telusuri pohon di sebelah kiri. Kotak centang mengaktifkan atau menonaktifkan aturan; kotak centang grup
   mengaktifkan atau menonaktifkan semua aturannya sekaligus; aturan yang berwarna abu-abu menunggu aturan
   yang diperlukannya. Panel kanan menampilkan parameter aturan yang dipilih dan deskripsinya.
3. Isi formulir **Instalasi**, **Akun**, dan **Bahasa** di bawah akar pertama.
4. Periksa profil dengan F7. Pesan muncul di bawah baris navigasi; klik pada pesan akan memilih aturan
   atau formulir yang dimaksud.
5. Buat file jawaban dengan F9: `autounattend.xml` disimpan di folder `output` di samping file
   program, bersama skrip yang disematkannya agar dapat dilihat. Salin file tersebut ke akar flash disk USB
   instalasi.
6. Simpan profil dengan Ctrl+S. Profil adalah file `.wwprofile` di folder `profiles` di samping file
   program, atau di mana pun yang Anda pilih dengan Simpan sebagai.
7. Coba file jawaban di mesin virtual sebelum PC sungguhan: instal, lalu baca PC itu dengan **PC Ini**
   dan bandingkan dengan profilnya.
8. Jaga PC yang terpasang tetap sesuai: **PC Ini** menerapkan aturan terpilih sebagai administrator dengan
   cadangan, atau menyimpan skrip mandiri yang menerapkan, mengembalikan, mengaudit, dan membatalkan di PC
   lain.

## Nama komputer

Di formulir **Instalasi**, nama komputer bisa acak (seperti bawaan Windows), nama tetap, atau templat
yang diuraikan file jawaban di setiap PC selama Setup. Templat berisi huruf Latin, angka, dan tanda
hubung di sekitar placeholder berikut:

| Placeholder | Hasilnya |
|---|---|
| `{serial}`, `{serial:N}` | Nomor seri komputer dari firmware-nya, huruf dan angkanya dalam huruf kapital; `:N` menyimpan N karakter terakhir, tanpa itu semuanya (hingga 15) |
| `{mac}`, `{mac:N}` | Alamat MAC adaptor jaringan fisik pertama, 12 digit heksadesimal dalam huruf kapital; `:N` menyimpan N digit terakhir, tanpa itu semua 12 |
| `{random}`, `{random:N}` | Huruf kapital dan angka acak tanpa yang mirip I, O, 0, dan 1; `:N` memberi N karakter, tanpa itu 6 |

N dari 1 hingga 15. Nomor seri atau alamat yang tidak dapat dibaca (tidak ada, atau pabrikan meninggalkan
pengganti seperti `Default string`) menjadi karakter acak dengan panjang yang sama; yang lebih pendek
tetap utuh. Nama dengan panjang penuh setiap placeholder muat dalam 15 karakter, jadi `PC-{serial}`
terlalu panjang dan `PC-{serial:12}` muat; sisakan satu huruf, karena Windows menolak nama yang hanya
berisi angka. `PC-{random}` menghasilkan `PC-` dan enam karakter acak, misalnya `PC-K7Q2MX`; dengan nomor
seri `5CG1234XYZ` dan alamat `00155D3E5A01`, `WS-{serial:6}` menghasilkan `WS-234XYZ` dan `{mac:4}-LAB`
menghasilkan `5A01-LAB`. Di bawah kolom, formulir menunjukkan hasil templat.

## Bahasa input

Formulir **Bahasa** mencantumkan bahasa input sesuai urutannya; yang pertama adalah default. Setiap
bahasa menampilkan namanya dan daftar tata letak keyboard: lebih dulu tata letak yang diberikan Windows
secara default, lalu semua tata letak Windows menurut namanya. **Tambah** mencari bahasa berdasarkan
sebagian nama atau tagnya di antara semua bahasa yang dikenal Windows dan menambahkannya dengan tata
letak yang dipilih; bahasa yang sama boleh muncul dua kali dengan dua tata letak, misalnya Ukraina
(Diperluas) dan Ukraina. Bahasa yang baru diberi nomor oleh Windows saat masuk pertama kali, seperti
Rusia (Ukraina), mendapat tata letak yang diberikan Windows saat itu, dan sampai masuk itu file jawaban
menyebut bahasa induknya. Bahasa dan tata letak berasal dari Windows tempat WinWeaver berjalan, dengan
nama dalam bahasanya.

Aturan **Wilayah pengguna** (`default-user.region`) pada profil pengguna bawaan menyimpan GeoID dan kode dua huruf negara pengguna baru. Kedua bidangnya diketik bebas, dan daftar di samping masing-masing memuat semua wilayah yang dikenal Windows, disaring menurut yang diketik: satu pilihan mengatur GeoID dan kode sekaligus, dan nama wilayah tampil setelah bidang. Pemeriksaan memperingatkan bila kedua nilai itu tidak menyebut satu wilayah yang sama dari daftar.

## File pengguna

File pengguna (`.wwusers`) mencantumkan akun lokal yang akan dibuat di PC beserta kata sandinya,
dienkripsi dengan kata sandi utama di file yang sama. **Menu, Baru, File pengguna** memulai file kosong,
**File pengguna berisi akun PC ini** file dengan semua akun normal PC ini (grup, status, kedaluwarsa kata
sandi, folder profil; tanpa kata sandi). Tab menampilkan akun dalam daftar dan akun yang dipilih dalam
formulir: nama, nama lengkap, deskripsi, grup, folder profil, dan perintah untuk akun yang sudah ada di
PC (atur kata sandi dari file, aktifkan atau nonaktifkan, tepat grup ini); tanpa perintah itu akun yang
ada dibiarkan apa adanya. **Buka kunci** meminta kata sandi utama, atau mengaturnya jika file belum
memilikinya; **Kata sandi** mengatur kata sandi akun yang dipilih; **Kunci** melupakannya. Tanpa kata
sandi utama, kata sandi tidak dapat dibaca lagi. **Periksa** mencantumkan masalah, **Baca PC ini**
menambahkan akun PC ini yang belum ada di file. **Buat di PC ini** dan **Terapkan kata sandi** mengubah
PC ini setelah pertanyaan dan memerlukan WinWeaver yang dijalankan sebagai administrator; hasilnya tampil
di bawah formulir, satu baris per akun. **Skrip** menyimpan `Create-<nama>.ps1` di sebelah file pengguna:
skrip itu meminta kata sandi utama sendiri dan melakukan hal yang sama di PC lain dari prompt perintah
yang dibuka sebagai administrator (`-PasswordsOnly` hanya mengatur kata sandi). Di formulir **Akun**
sebuah profil, **Dari file pengguna** memuat akunnya, beserta kata sandinya jika Anda memberikan kata
sandi utama. Mengubah grup di formulir menandai tepat grup ini, sehingga akun berpindah antara Administrators dan Users; akun yang menjalankan WinWeaver tetap di Administrators.

Selama file terbuka, formulir menampilkan kata sandi akun yang dipilih sebagai titik, dengan
**Tampilkan** dan **Salin** (riwayat papan klip Windows menyimpan apa yang disalin). Di formulir **Akun**
sebuah profil, **Ke file pengguna** membuat file pengguna dari akun-akunnya; kata sandinya menunggu kata
sandi utama sebelum penyimpanan pertama.

## Jendela

- **Tab**: setiap dokumen yang terbuka adalah tab: profil, perbandingan, panduan ini, penanda, server MCP,
  pengaturan. Tombol menu di ujung kiri baris tab berisi Baru, Buka, Simpan, Bandingkan, panduan,
  server MCP, penanda, Pengaturan, dan Keluar.
- **Tentang dan Lisensi** di menu: versi dengan commit build-nya dan lisensi (MIT) yang tertanam di
  program, sehingga tetap ikut `winweaver.exe` meski hanya file itu yang dipindahkan.
- **Bilah perintah**: Baru, Buka, Simpan, Periksa, Buat, PC Ini, Bandingkan. Di jendela yang sempit,
  bilah ini hanya menampilkan ikon, di baris navigasi.
- **Baris navigasi**: Kembali dan Maju berpindah di antara item yang telah Anda pilih; jalur navigasi
  menunjukkan posisi Anda dan dapat melompat ke tingkat mana pun; Penanda menandai posisi; Cari memfilter
  pohon saat Anda mengetik dan mengingat apa yang pernah Anda cari.
- **Pohon dan panel detail**: pohon di sebelah kiri, item yang dipilih di sebelah kanan. Deskripsi
  diciutkan dengan Ctrl+D. Aturan dari grup yang dipilih menampilkan tombol alih di ujung barisnya.
- **Tautan**: setiap aturan, grup, katalog, file profil, dan kunci registri yang disebut tampilan
  mengarah ke sana: aturan yang dibutuhkan suatu aturan, yang membutuhkannya, dan yang berkonflik
  dengannya; aturan yang disebut dalam pesan; grup di atas dan di dalam sebuah grup; kebijakan yang sama
  di katalog lain; aturan yang menulis kunci registri, di tab registri dan di file registri. Tautan di
  profil membuka pohonnya hingga ke item; dari tab lain, tautan menampilkan aturan di tab profil pertama.
- **Bilah status**: status dokumen, versi katalog, server MCP, dan pesan terakhir.
- **Buka folder**: setelah build atau menyimpan skrip, file registri, atau profil, tombol di ujung bilah
  status menampilkan file di foldernya di Explorer; Pengaturan, File membuka folder program.

- **Menu dan tab**: setiap item menu punya ikon. Saat tab lebih lebar dari jendela, barisnya bergulir
  dengan roda tetikus, tab aktif muncul ke tampilan, dan tombol di ujung kanan mendaftar semua tab.
- **Hanya yang aktif** di bilah profil hanya menampilkan aturan yang aktif dengan grupnya terbuka; alat
  MCP `list_groups`, `list_rules` dan `search_rules` menerima `only_on` dengan cara yang sama.
- **Parameter**: bidang sebuah aturan memakai lebar panel; teks dengan batas berkas jawaban (kontak
  dukungan) menghitung karakternya.
- **Pekerjaan di latar belakang**: pembacaan PC ini, impor templat kebijakan, perubahan PC ini dan
  pencarian registri tampil di panel pojok kanan bawah dengan waktu yang telah berjalan dan Hentikan bila
  pekerjaan bisa dihentikan; panel melipat menjadi indikator.

## PC Ini

**PC Ini** di bilah perintah membaca pengaturan komputer ini melalui Windows, tanpa mengubah
apa pun:

**Profil.** Formulir Profil, yang pertama di pohon, mengubah nama, penulis, dan komentar profil; bagian pertama baris navigasi menampilkan profil dengan formulirnya secara ringkas. Templat terbuka dengan bahasa jendela dan wilayah Windows Anda, serta bahasa input Windows ini (English International bila tidak terbaca). **Menu, PC ini** bekerja dari tab mana pun: membaca ke profil baru tidak memerlukan profil yang terbuka.

- **Baca PC ini ke profil baru**: setiap aturan yang dapat dinilai pada Windows yang sedang berjalan menjadi
  aktif (berlaku), nonaktif (berlaku sebagian atau tidak berlaku), atau mempertahankan statusnya (tidak dapat
  dibaca, atau sama seperti pada Windows yang bersih); formulir mengambil edisi, zona waktu, dan bahasa.
- **Baca pilihan ke profil ini** dan **Periksa pilihan di PC ini** bekerja pada aturan atau grup
  yang dipilih.
- **Simpan skrip penerapan**, **skrip pengembalian ke default**, atau **skrip audit** menulis skrip
  PowerShell untuk PC ini atau PC lain. Jalankan skrip penerapan atau pengembalian sebagai administrator,
  terlebih dahulu di PC uji; skrip ini menyimpan cadangan yang dapat dipulihkan oleh `Undo-Apply.ps1` di
  sampingnya. Laporan skrip audit dibuka dengan **Buka laporan audit** dan menjadi profil.
- **Simpan file registri** atau **file registri pengembalian ke default** menulis nilai registri dari
  pilihan sebagai file `.reg` untuk regedit atau `reg import`; yang tidak dapat dimuat file registri
  (langkah skrip, aplikasi, kapabilitas) tetap ada di dalamnya sebagai komentar. Nilai profil pengguna
  default ditulis ke `HKEY_USERS\UnattendDefault`, dan komentar di awal file menjelaskan cara memuat
  hive-nya di sana.

Aturan yang hanya bekerja selama instalasi atau saat pengguna masuk pertama kali tidak dibaca.

Fitur dan kemampuan Windows dibaca melalui DISM saat WinWeaver berjalan sebagai administrator, sehingga
pembacaan bertambah beberapa detik; tanpa hak tersebut keduanya tidak dapat dibaca.
Pembacaan berjalan di latar belakang: jendela tetap bekerja, dan indikator di bilah status menunjukkannya.

Saat diterapkan sebagai administrator, aturan yang menghapus aplikasi atau kemampuan Windows juga
menghapusnya dari jendela: kemampuan melalui DISM, aplikasi untuk semua pengguna dan untuk akun yang dibuat
kemudian. Cadangan mencantumkannya, tetapi Batalkan perubahan tidak mengembalikannya.

Langkah skrip aturan juga diterapkan secara native, tanpa PowerShell: .NET Framework 3.5 dan driver dari
media instalasi, tipe mulai layanan, NetBIOS, drive lepasan di panel navigasi, "kata sandi tidak pernah
kedaluwarsa" untuk akun profil, penonaktifan akun bawaan Administrator dan Guest (tidak pernah akun yang
sedang dipakai), dan penghapusan salinan file jawaban. Urungkan tidak mengembalikan program yang
dijalankan, akun yang diubah, dan file yang dihapus; konfirmasi menyebut berapa banyak langkah seperti
itu dalam rencana.

## Membandingkan profil

**Menu, Bandingkan dengan file** atau **Bandingkan dengan preset** membuka tab perbandingan: setiap perbedaan
antara profil aktif dan profil lainnya, dalam tiga kolom. "Terapkan sisi kanan" mengatur nilai-nilai tersebut
di profil tempat perbandingan dibuat; "Simpan sebagai file perbedaan" menyimpan perbedaan sebagai file
`.wwdiff` yang nanti dapat diterapkan ke profil mana pun.

**Bandingkan, Perubahan belum disimpan** membuka tab perbedaan profil dengan filenya, atau dengan preset
asalnya: di kiri profil seperti sekarang, di kanan seperti yang disimpan.

## Membuka file lain

Buka menerima profil (`.wwprofile`), perbandingan (`.wwdiff`), file penanda (`.wwmarks`), halaman Markdown,
file jawaban dengan profil tersemat (profil yang tersemat di dalamnya), profil lama dalam JSON, dan
laporan audit (`.json`).

File registri (`.reg`) terbuka di tabnya sendiri, seperti yang dijelaskan di bagian tentang registri.

## Templat kebijakan

**Pengaturan, Templat kebijakan** mengimpor templat ADMX Windows ini, sebuah folder, atau file katalog;
kebijakannya menjadi aturan di bawah Kebijakan, dan impor disimpan di `imports/`. Hanya impor
yang dicentang yang dibaca dan dimuat; yang lain tidak memakai memori. Profil dapat memuat opsi dari
impor yang tidak dimuat: opsi tetap di profil tanpa perubahan, tercantum di **Opsi tidak dikenal** di akhir
pohon, dan tidak disertakan dalam pembuatan, dengan peringatan saat profil dibuka dan saat file jawaban
dibuat. **Muat katalog** di grup itu menemukan impor yang memuatnya dan hanya memuat impor tersebut.
Sebuah pengaturan (Umum, Jendela) menyembunyikan grup itu.

Menu **Katalog** pada bilah profil mencantumkan setiap berkas katalog di `imports/`, dimuat atau tidak: yang dimuat bertanda centang dan berwarna aksen, dan klik membuka tabnya; yang lain dimuat dengan klik dan juga membuka tabnya. Daftar dibaca ulang setiap kali menu dibuka. Secara default, klik pada katalog di menu memuat atau melepasnya; di Pengaturan, Umum, pilihan lain membuat menu hanya mencantumkan katalog yang dimuat dan klik membuka tabnya.

Arsip zip berisi templat, seperti yang dirilis Chrome, Edge, dan Firefox, atau berisi file katalog diimpor
dengan **Impor arsip...** dan dibaca tanpa diekstrak.

Kebijakan yang ada di dua impor tampil di kedua pohon. **Bandingkan** di bilah tab katalog
menyandingkannya dengan impor lain yang dimuat atau file katalog yang terbuka: kebijakan yang hanya ada
di salah satunya dan yang judul, deskripsi, nilai, atau parameternya berbeda. Menghapus impor, seperti
setiap penghapusan, meminta konfirmasi lebih dulu.

Panel detail sebuah opsi menautkan tab katalog yang memuatnya dan kebijakan yang sama di pohon impor lain;
penanda opsi itu juga. Di Pengaturan, Umum dipilih bagian deskripsi mana yang terbuka sejak awal.

**Sunting** di bilah tab katalog membuka katalog di editor, dan **Katalog baru** di menu templat memulai katalog
kosong milik Anda. Sebuah kebijakan mendapat judul, deskripsi, komputer atau profil pengguna bawaan, dan nilai
registri yang ditulis atau dihapusnya; **Terapkan** memeriksa seluruh file, dan tidak ada yang berubah bila
pemeriksaan gagal. Simpan (Ctrl+S) menulis file dan memuatnya lagi. **Hapus grup** menghapus grup yang dipilih, setelah sebuah pertanyaan, beserta semua grup dan kebijakan di dalamnya; grup teratas file tetap ada.

Perbandingan terbuka di sebuah tab: kebijakan yang berbeda dalam pohon kategorinya, dengan sakelar untuk
yang hanya ada di katalog pertama, hanya di katalog kedua dan yang berbeda, serta kebijakan terpilih apa
adanya di kedua katalog, berdampingan. **Simpan laporan** menulis laporan sebagai berkas Markdown,
**Sebagai halaman** menampilkannya sebagai halaman. **Salin ke** di bilah tab katalog menyalin kebijakan
terpilih atau grup terpilih beserta grup di bawahnya ke katalog yang terbuka di penyuntingnya (Anda memilih
yang mana bila ada beberapa) atau ke katalog baru, dengan id yang sama dan teks bahasa lain: katalog yang
hanya berisi kebijakan yang diperlukan, untuk diteruskan sebagai berkas.

## Registri

**Menu, Registri** membuka registri PC ini di sebuah tab: kunci di kiri, nilai dari kunci terpilih di
kanan beserta jenisnya (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD, REG_BINARY), jalurnya
di baris atas. Klik ganda menyunting nilai; Kunci baru, Nilai baru, Sunting, dan Hapus ada di bilah, dan
penghapusan meminta konfirmasi lebih dulu. Kunci mesin (HKEY_LOCAL_MACHINE) hanya berubah bila WinWeaver
berjalan sebagai administrator. Tombol di samping kunci pada **Yang ditulis** membukanya di sini.
**Penanda** menyimpan kunci dan nilai dengan komentar, dan penanda itu menampilkan aturan yang
menulisnya. **Cari** di bilah mencari di bawah kunci terpilih seperti pencarian regedit: di nama kunci,
nama nilai, dan data, hanya seluruh string bila diminta. Pencarian berjalan di latar belakang,
**Hentikan** menghentikannya, dan klik pada tempat yang ditemukan menampilkannya.

**Izin** di bilah menampilkan pemilik dan daftar akses kunci terpilih: entri warisan ditampilkan, entri
milik kunci itu sendiri dapat diubah (izinkan atau tolak, kontrol penuh atau baca, berlaku untuk apa),
akun atau grup PC ini mendapat entri baru, dan pewarisan dari kunci di atasnya dapat dimatikan. Terapkan
bertanya terlebih dahulu; kunci komputer hanya berubah bila WinWeaver dijalankan sebagai administrator.

File registri (`.reg`) terbuka di tabnya sendiri sebagai struktur, bukan teks: di kiri kunci dan komentar
file sesuai urutannya, di kanan kunci terpilih dengan nilainya dalam tipe regedit dan komentar di
antaranya, tautan ke tab registri, serta aturan dari katalog yang dimuat yang menulis kunci itu atau
nilai terpilih. **Kunci baru**, **Nilai baru**, **Sunting**, dan **Hapus** ada di bilah, dan penghapusan
bertanya terlebih dahulu. **Simpan** menulis kembali setiap baris: komentar dan baris yang tidak dibaca
program tetap di tempatnya. File yang disimpan dari **PC Ini** menyebut aturannya dalam komentar, dan
panah komentar seperti itu menunjukkan aturan di profil.

Berkas Kebijakan Grup (`Registry.pol`) juga terbuka di tabnya sendiri: berkas komputer dan pengguna
Windows ini (**Menu, Kebijakan Grup**; bilah profil, katalog dan registri) atau berkas `.pol` mana pun.
Kunci menjadi pohon di kiri, entri kunci terpilih di kanan dengan apa yang dilakukan masing-masing. **Kunci
baru**, **Nilai baru**, **Ubah** (atau klik ganda) dan **Hapus** mengubah entri di jendela di atas tab,
seperti nilai berkas registri; **Akar** memilih apakah berkas di luar Kebijakan Grup berlaku untuk
komputer atau pengguna (berkas Windows ini mengambilnya dari foldernya); **Tambahkan berkas** mengambil
entri berkas Kebijakan Grup lain atau perubahan berkas registri akar itu; **Ke .reg** menyimpan berkas
sebagai berkas registri. Simpan menulis kembali format Kebijakan Grup; berkas Windows ini disimpan sebagai
salinan di tempat lain, karena Kebijakan Grup sendiri yang menulisnya. Perubahan yang belum disimpan
kembali dari draf setelah mulai ulang.

## Server MCP

**Menu, Server MCP** membuka tab server yang dipakai agen AI untuk membaca profil dan, bila mode mengizinkan, mengubahnya. Tab ini menyunting file konfigurasi, `mcp/default.wwmcp` di samping program, dan menyimpannya setiap kali ada perubahan: port dan token, mode (read hanya membaca, edit mengubah profil di memori, files juga menyimpannya dan menulis file jawaban), izin membaca PC ini, membaca atau mengubah registri, dan mengubah penanda serta katalog, profil awal server, dan log terperinci. **Buka** mengambil file `.wwmcp` lain dan **Simpan sebagai** membuat salinan; `winweaver --mcp FILE` melayani tanpa jendela sesuai isi file. Log server yang berjalan mencantumkan setiap panggilan; **Tampilkan rincian** membuka panel di sebelah kanannya dengan argumen dan jawaban panggilan terpilih serta tombol ke aturan, profil, katalog, dan kunci registri yang disebutnya. Log server adalah `logs/mcp-<tanggal>.jsonl`, satu objek JSON per baris; log terperinci, mati secara bawaan, menyimpan setiap permintaan dan jawaban utuh tanpa kata sandi, kunci, dan token.

## Perender

Jendela digambar dengan OpenGL atau DirectX 12. **Otomatis** (Pengaturan, Umum, Jendela) mencoba OpenGL
lebih dulu, yang memakai lebih sedikit memori, lalu beralih ke DirectX 12 bila OpenGL tidak berjalan: di
mesin virtual tanpa akselerasi 3D, di instalasi baru tanpa driver kartu grafis, di beberapa sesi jarak
jauh. DirectX 12 juga menggambar di sana, dengan rasterizer perangkat lunak Windows. Jika jendela tampil
salah dengan OpenGL (driver lama), pilih DirectX 12. Perubahan berlaku pada start berikutnya;
`winweaver.exe --renderer directx` (atau `opengl`, `auto`) memilih hanya untuk satu start. Log di `logs/`
menyebutkan apa yang dipakai jendela untuk menggambar.

## Baris perintah

`winweaver.exe` tanpa parameter membuka jendela. Nilai ditulis setelah parameternya dengan spasi atau `=`:
`--renderer=directx`.

| Parameter | Fungsinya |
|---|---|
| `--renderer NAME` | Membuka jendela dan mencoba perender NAME lebih dulu, hanya untuk start ini: `auto`, `opengl`, atau `directx` |
| `--mcp stdio` | Melayani MCP lewat input dan output standar, tanpa jendela; klien MCP menjalankannya dengan cara ini |
| `--mcp http --token TOKEN` | Melayani MCP di `http://127.0.0.1:8765/mcp` tanpa jendela; token minimal 32 karakter |
| `--port N` | Dengan `--mcp http`: port lain |
| `--mode NAME` | Dengan `--mcp`: `read` (default) hanya membaca, `edit` mengubah profil di memori, `files` juga menyimpannya dan menulis file jawaban |
| `--read-pc` | Dengan `--mcp`: mengizinkan membaca pengaturan PC ini |
| `--registry NAME` | Dengan `--mcp`: `off` (default), `read` mengizinkan alat membaca registri, `write` juga mengubahnya |
| `--edit-library` | Dengan `--mcp`: alat boleh mengubah penanda dan file katalog; membacanya selalu diizinkan |
| `--profile NAME` | Dengan `--mcp`: preset atau profil dari `profiles/` menjadi profil saat ini |
| `--mcp FILE` | Melayani MCP tanpa jendela sesuai file konfigurasi: jalur ke file `.wwmcp` atau nama dari folder `mcp` di samping program |
| `--config FILE` | Dengan `--mcp stdio` atau `--mcp http`: pengaturan dari file konfigurasi, termasuk token; opsi lain mengubahnya untuk start ini |
| `--detailed-log` | Dengan `--mcp`: log menyimpan setiap permintaan dan jawaban utuh, tanpa kata sandi, kunci, dan token |
| `--version` | Menampilkan versi |
| `--check-catalog` | Memuat katalog bawaan dengan terjemahannya dan menampilkan masalahnya |
| `--build PROFILE` | Memeriksa profil dan membuat file jawabannya tanpa jendela: `autounattend.xml` di `output`, dengan skrip yang disematkannya di sampingnya; PROFILE adalah file `.wwprofile`, preset (`Office`, `Strict`, `Laptop`, `Home`), atau nama profil dari `profiles/`; kesalahan pemeriksaan menghentikan pembuatan |
| `--import-admx SOURCE` | Mengimpor templat kebijakan seperti Pengaturan, Templat kebijakan: `system` (templat Windows ini), folder file ADMX, file `.admx` dengan ADML dari folder bahasanya, arsip `.zip`, atau file `.wwcatalog`; file katalog masuk ke `imports/`, dan jendela serta server MCP memuatnya pada start berikutnya (jalankan saat jendela tertutup) |
| `--export-catalog SOURCE` | Menulis file katalog (`.wwcatalog`) seperti Ekspor di Pengaturan, Templat kebijakan: dari impor menurut id-nya (nama file-nya di `imports/`), atau dari sumber `--import-admx` yang dibaca tanpa mengimpornya; `<id>.wwcatalog` di `output` |
| `--read-pc` | Tanpa `--mcp`: membaca PC ini ke profil baru bernama komputer, seperti PC Ini, Baca PC ini, dan menyimpannya di `profiles/`; dengan file `.xml` di `--out`, menulis file jawaban profil itu sebagai gantinya; fitur dan kemampuan Windows hanya dibaca saat program berjalan sebagai administrator |
| `--base PROFILE` | Dengan `--read-pc`: profil awal pembacaan, diberikan seperti untuk `--build`; secara default preset Office |
| `--diff A B` | Menulis perbedaan dari profil A ke profil B sebagai file perbedaan, `A-vs-B.wwdiff` di `profiles/`; A dan B diberikan seperti untuk `--build` |
| `--script KIND FILE` | Menulis skrip mandiri dari profil (`.wwprofile`) atau perbedaan (`.wwdiff`) tanpa jendela: `apply`, `revert` (kembali ke default Windows) atau `audit`; skrip penerapan atau pengembalian mendapat `Undo-Apply.ps1` di sampingnya |
| `--reg KIND FILE` | Menulis file registri penerapan (`apply`) atau pengembalian ke default (`revert`) dari profil atau perbedaan |
| `--only IDS` | Dengan `--script` atau `--reg`: hanya aturan atau grup ini, id-nya dipisahkan koma |
| `--out FILE` | Dengan `--build`, `--export-catalog`, `--read-pc`, `--diff`, `--script`, atau `--reg`: file yang ditulis, jika nilainya berakhir dengan ekstensi yang ditulis perintah itu (`.xml`, `.wwcatalog`, `.wwprofile`, `.wwdiff`, `.ps1`, `.reg`); nilai lain adalah folder, dibuat bila belum ada, untuk file dengan nama biasanya; `Undo-Apply.ps1` dan skrip file jawaban diletakkan di samping file itu; tanpa `--out` file masuk ke `output` di samping program (`profiles/` untuk `--read-pc` dan `--diff`) |
| `--help` | Menampilkan daftar parameter |
| `--smoke-window` | Membuka jendela kecil, menggambar dua bingkai, dan menampilkan apa yang menggambarnya (juga dengan `--renderer`): pemeriksaan grafis |
| `--elevated-apply PLAN DIGEST` | Jendela menjalankan dirinya sendiri dengan cara ini untuk menerapkan rencana yang dikonfirmasi sebagai administrator; bukan untuk dijalankan manual |

Perintah yang menulis file menyebutkan setiap file yang ditulisnya dan berakhir dengan kode 0; penggunaan
parameter yang salah berakhir dengan kode 2, kegagalan dengan 1, dan alasannya ditampilkan.

Mode dan izin membaca PC ini diberikan pada setiap start dan tidak pernah disimpan. Pesan masuk ke konsol
tempat program dijalankan dan ke `logs/`.

## Keyboard

| Tombol | Tindakan |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | Baru, Buka, Simpan, Simpan sebagai |
| F7, F9 | Periksa profil, buat file jawaban |
| Ctrl+F, Esc | Cari, kosongkan dan tutup pencarian |
| Alt+Left, Alt+Right | Kembali, Maju |
| Ctrl+B | Tandai posisi |
| Ctrl+D | Ciutkan atau tampilkan deskripsi |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Tutup tab, tab berikutnya, tab sebelumnya |
| Esc | Batalkan konfirmasi perubahan PC ini |
| Ctrl+, | Pengaturan |
| F1 | Panduan ini |

## File di samping file program

`winweaver.settings.json` (bahasa, tema, file terbaru, tab dari sesi terakhir), `profiles/` (profil
Anda), `output/` (file jawaban dan skrip), `drafts/` (perubahan yang belum disimpan, dipulihkan saat
program dimulai berikutnya), `mcp/` (konfigurasi server MCP), `logs/` (apa yang dilakukan program), `bookmarks.wwmarks`. Tidak ada yang
ditulis di tempat lain.

Pengaturan, penanda, draf, dan file perbedaan menyimpan jalur file di dalam folder ini secara relatif
terhadapnya, sehingga seluruh folder dapat dipindahkan atau disalin ke PC lain; file di tempat lain
menyimpan jalur lengkapnya.

## Belum tersedia

Untuk saat ini tidak ada yang tersisa dari rencana: kebutuhan program berikutnya akan muncul dari
pemakaiannya.
