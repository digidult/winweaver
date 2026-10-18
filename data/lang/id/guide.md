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
- **Katalog** terdiri dari aturan bawaan dalam tiga bagian (Instalasi, Kebijakan standar, Lainnya) dan
  templat kebijakan (ADMX) yang Anda impor, yang juga menjadi aturan.
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

## PC Ini

**PC Ini** di bilah perintah membaca pengaturan komputer ini melalui Windows, tanpa mengubah
apa pun:

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

## Membandingkan profil

**Menu, Bandingkan dengan file** atau **Bandingkan dengan preset** membuka tab perbandingan: setiap perbedaan
antara profil aktif dan profil lainnya, dalam tiga kolom. "Terapkan sisi kanan" mengatur nilai-nilai tersebut
di profil tempat perbandingan dibuat; "Simpan sebagai file perbedaan" menyimpan perbedaan sebagai file
`.wwdiff` yang nanti dapat diterapkan ke profil mana pun.

## Membuka file lain

Buka menerima profil (`.wwprofile`), perbandingan (`.wwdiff`), file penanda (`.wwmarks`), halaman Markdown,
file jawaban yang dibuat oleh WinWeaver atau WinKickOff (profil yang tersemat di dalamnya), profil WinKickOff,
dan laporan audit (`.json`).

File registri (`.reg`) terbuka di tabnya sendiri, seperti yang dijelaskan di bagian tentang registri.

## Templat kebijakan

**Pengaturan, Templat kebijakan** mengimpor templat ADMX Windows ini, sebuah folder, atau file katalog;
kebijakannya menjadi aturan di bawah Kebijakan standar, dan impor disimpan di `imports/`. Hanya impor
yang dicentang yang dibaca dan dimuat; yang lain tidak memakai memori. Profil dapat memuat opsi dari
impor yang tidak dimuat: opsi tetap di profil tanpa perubahan, tercantum di **Opsi tidak dikenal** di akhir
pohon, dan tidak disertakan dalam pembuatan, dengan peringatan saat profil dibuka dan saat file jawaban
dibuat. **Muat katalog** di grup itu menemukan impor yang memuatnya dan hanya memuat impor tersebut.
Sebuah pengaturan (Umum, Jendela) menyembunyikan grup itu.

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
pemeriksaan gagal. Simpan (Ctrl+S) menulis file dan memuatnya lagi.

## Registri

**Menu, Registri** membuka registri PC ini di sebuah tab: kunci di kiri, nilai dari kunci terpilih di kanan
beserta jenisnya (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD, REG_BINARY), jalurnya di baris
atas. Klik ganda menyunting nilai; Kunci baru, Nilai baru, Sunting, dan Hapus ada di bilah, dan penghapusan
meminta konfirmasi lebih dulu. Kunci mesin (HKEY_LOCAL_MACHINE) hanya berubah bila WinWeaver berjalan sebagai
administrator. Tombol di samping kunci pada **Yang ditulis** membukanya di sini. **Penanda** menyimpan kunci dan
nilai dengan komentar, dan penanda itu menampilkan aturan yang menulisnya. **Cari** di bilah mencari di bawah kunci
terpilih seperti pencarian regedit: di nama kunci, nama nilai, dan data, hanya seluruh string bila diminta.
Pencarian berjalan di latar belakang, **Hentikan** menghentikannya, dan klik pada tempat yang ditemukan
menampilkannya. Hak akses kunci menyusul.

File registri (`.reg`) terbuka di tabnya sendiri sebagai struktur, bukan teks: di kiri kunci dan komentar
file sesuai urutannya, di kanan kunci terpilih dengan nilainya dalam tipe regedit dan komentar di
antaranya, tautan ke tab registri, serta aturan dari katalog yang dimuat yang menulis kunci itu atau
nilai terpilih. **Kunci baru**, **Nilai baru**, **Sunting**, dan **Hapus** ada di bilah, dan penghapusan
bertanya terlebih dahulu. **Simpan** menulis kembali setiap baris: komentar dan baris yang tidak dibaca
program tetap di tempatnya. File yang disimpan dari **PC Ini** menyebut aturannya dalam komentar, dan
panah komentar seperti itu menunjukkan aturan di profil.

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
| `--help` | Menampilkan daftar parameter |
| `--smoke-window` | Membuka jendela kecil, menggambar dua bingkai, dan menampilkan apa yang menggambarnya (juga dengan `--renderer`): pemeriksaan grafis |
| `--elevated-apply PLAN DIGEST` | Jendela menjalankan dirinya sendiri dengan cara ini untuk menerapkan rencana yang dikonfirmasi sebagai administrator; bukan untuk dijalankan manual |

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

`settings.json` (bahasa, tema, file terbaru, tab dari sesi terakhir), `profiles/` (profil
Anda), `output/` (file jawaban dan skrip), `drafts/` (perubahan yang belum disimpan, dipulihkan saat
program dimulai berikutnya), `mcp/` (konfigurasi server MCP), `logs/` (apa yang dilakukan program), `bookmarks.wwmarks`. Tidak ada yang
ditulis di tempat lain.

## Belum tersedia

Langkah skrip beberapa aturan dan profil pengguna lain saat diterapkan dari jendela: sementara ini skrip
penerapan yang melakukannya.
